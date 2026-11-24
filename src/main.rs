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
mod ui;

fn main() {
    // `editor ctl …` and `editor mcp` talk to a running editor and never open a window.
    let argv: Vec<String> = std::env::args().collect();
    match argv.get(1).map(String::as_str) {
        Some("ctl") => std::process::exit(ipc::client::main(&argv[2..])),
        Some("mcp") => std::process::exit(ipc::mcp::main()),
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
    let app =
        rune::App::new(spec).window_options(rune::WindowOptions { transparent, position, ..Default::default() }).on_close(|s, _tx| {
            app::session::save(s);
            s.release_waiters();
            if let Some(svc) = &s.svc {
                svc.close_socket();
            }
        });
    let _ = app::ipc::HANDLE.set(app.handle());
    app.run();
}
