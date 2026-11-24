//! editor: a lightweight modal editor for opening code fast, reading it, finding your way
//! around it and making quick edits. Built on Rune.
//!
//! `editor [path[:line]] [+line]` opens a folder or files. `--snap out.png --scene NAME`
//! renders a scene headless (see `SCENES`).

mod app;
mod args;
mod config;
mod fuzzy;
mod highlight;
mod scenes;
mod services;
mod theme;
mod ui;

fn main() {
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
    rune::App::new(spec)
        .window_options(rune::WindowOptions { transparent, position, ..Default::default() })
        .on_close(|s, _tx| app::session::save(s))
        .run();
}
