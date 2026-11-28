//! editor: a lightweight modal editor for opening code fast, reading it, finding your way
//! around it and making quick edits. Built on Rune.
//!
//! `editor [path[:line]] [+line] [--wait] [--new]` opens a folder or files, in the editor
//! already running for the project when there is one. `editor ctl` and `editor mcp` talk to a
//! running editor (see `docs/agents.md`). `--snap out.png --scene NAME` renders a scene
//! headless (see `SCENES`).

mod app;
mod args;
mod config;
mod fuzzy;
mod highlight;
mod ipc;
mod scenes;
mod services;
mod theme;
mod timing;
mod ui;

const USAGE: &str = "editor: a modal code editor. Read fast. Edit quick.

Usage:
  editor [PATH[:LINE]]... [+LINE] [--wait] [--new]
  editor ctl COMMAND [ARGS]   drive the running editor (editor ctl --help)
  editor mcp                  serve the running editor to AI agents, over stdio

Examples:
  editor .                    open this folder as the project
  editor src/main.rs:42       open a file at line 42, in the project around it
  editor --wait NOTES.md      return when its buffer closes, for $EDITOR and git
  editor --new                open a new window, also when one runs for the project

A path opens in the editor already running for its project, when there is one.

Options:
  --wait          return when the last file's buffer closes
  --new           open a new window
  -h, --help      print this help
  -V, --version   print the version

Docs: https://github.com/bjarneo/editor/blob/main/docs/cli.md
";

const MCP_USAGE: &str = "editor mcp: a Model Context Protocol server, over stdio, for the editor
running in the current folder. An agent starts it. You do not run it by hand.

  claude mcp add editor -- editor mcp

Tools and setup for Codex and opencode: https://github.com/bjarneo/editor/blob/main/docs/agents.md
";

fn main() {
    timing::start();
    // `editor ctl …` and `editor mcp` talk to a running editor and never open a window.
    let argv: Vec<String> = std::env::args().collect();
    match argv.get(1).map(String::as_str) {
        Some("ctl") => std::process::exit(ipc::client::main(&argv[2..])),
        Some("mcp") if argv[2..].iter().any(|a| a == "-h" || a == "--help") => {
            print!("{MCP_USAGE}");
            return;
        }
        Some("mcp") => std::process::exit(ipc::mcp::main()),
        Some("-h" | "--help") => {
            print!("{USAGE}");
            return;
        }
        Some("-V" | "--version") => {
            println!("editor {}", env!("CARGO_PKG_VERSION"));
            return;
        }
        _ => {}
    }
    // `editor file` with an editor already open on that project opens the file there.
    let tooling =
        argv.iter().any(|a| matches!(a.as_str(), "--snap" | "--script" | "--video" | "--tree" | "--trace" | "--list-scenes" | "--scene"));
    if !tooling && !argv.iter().any(|a| a == "--new") {
        let (root, files) = args::targets();
        if let Some(code) = ipc::client::open_in_running(&root, &files, argv.iter().any(|a| a == "--wait")) {
            std::process::exit(code);
        }
    }
    timing::mark("arguments and running-editor check");
    rune::prewarm_gpu();
    rune::tooling::set_app_id("editor");
    // Open at the size (and, off Wayland, the place) the window had when it last closed.
    let frame = app::session::load_window();
    let size = frame.map_or((1440.0, 900.0), |f| (f.w, f.h));
    let position = frame.and_then(|f| Some((f.x?, f.y?))).filter(|_| std::env::var_os("WAYLAND_DISPLAY").is_none());
    let spec = rune::AppSpec::<app::App> {
        min_size: Some((640.0, 420.0)),
        scenes: scenes::SCENES,
        setup: Some(|s, waker| s.setup(waker)),
        poll: Some(|s, clock, tx| s.poll(clock, tx)),
        on_key: Some(app::keys::on_key),
        ..rune::AppSpec::new("editor", size, app::App::init, ui::view)
    };
    let transparent = match config::load() {
        config::Loaded::Ok(c) => c.window.opacity < 100,
        _ => false,
    };
    // On macOS the tab bar is the titlebar: the traffic lights sit in it, centred, and its
    // empty parts move the window. The lights sit 16 points in, centred in the bar's height.
    // Setting it here also puts them in the same place in a headless `--chrome` render on Linux.
    let lights = (16.0, ((ui::editor_view::BAR_H - rune::Chrome::TRAFFIC_LIGHT) / 2.0).round());
    let app = rune::App::new(spec)
        .window_options(rune::WindowOptions { transparent, position, ..Default::default() })
        .chrome(rune::Chrome { traffic_lights: Some(lights), ..rune::Chrome::DEFAULT });
    #[cfg(target_os = "macos")]
    let app = app.titlebar(rune::Titlebar::new(ui::editor_view::BAR_H));
    let app = app.on_close(|s, _tx| {
        app::session::save(s);
        s.release_waiters();
        if let Some(svc) = &s.svc {
            svc.close_socket();
        }
    });
    let _ = app::ipc::HANDLE.set(app.handle());
    app.run();
}
