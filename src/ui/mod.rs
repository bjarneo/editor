//! The views. `view` picks the screen: setup on first launch, else the editor (with the
//! Settings tab when it shows).

pub mod editor_view;
pub mod font;
pub mod overlays;
pub mod review;
pub mod settings_view;
pub mod setup_view;
pub mod text;
pub mod widgets;

use rune::{Cx, El};

use crate::app::{App, Screen};
use crate::theme::{self, Palette};

/// The palette to paint with: the setup's pick while it runs, else config.toml's theme.
pub fn palette(app: &App, system_dark: bool) -> Palette {
    let cfg = match &app.screen {
        Screen::Setup(s) => &s.cfg,
        Screen::Editor => &app.cfg,
    };
    theme::resolve(theme::Source::parse(&cfg.theme.mode), &cfg.theme.dark, &cfg.theme.light, system_dark, app.omarchy.as_ref()).0
}

pub fn view(app: &App, cx: &mut Cx<App>) -> El {
    let p = palette(app, cx.system_dark());
    let opacity = (app.cfg.window.opacity as f32 / 100.0).clamp(0.7, 1.0);
    let root = match &app.screen {
        Screen::Setup(_) => setup_view::view(app, &p, cx),
        Screen::Editor => editor_view::view(app, &p, cx),
    };
    // Text, the cursor and lists change at once, as an editor's should. Colours still ease,
    // so a theme switch fades.
    let scale = cx.env().scale;
    root.fill(p.bg.alpha(opacity))
        .instant_changes()
        // The window's frame, kept for the next launch. Nothing on screen reads it.
        .on_layout_tx(|s: &mut App, r, tx| {
            s.window.w = r[2];
            s.window.h = r[3];
            tx.live_only();
        })
        .on_screen_layout(move |s: &mut App, r| {
            if std::env::var_os("WAYLAND_DISPLAY").is_none() {
                s.window.x = Some((r[0] * scale).round() as i32);
                s.window.y = Some((r[1] * scale).round() as i32);
            }
        })
}
