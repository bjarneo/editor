//! The editor screen: the tab bar, the file tree, the breadcrumbs over the text area (or
//! Settings), the status line and the command line, with the leader menu and pickers over them.
//! The parts are in `chrome`, `tree_view`, `status` and `scrollbar`.

use editor::editor::Mode;
use rune::{Cx, El, col, row};

use super::{chrome, font, overlays, scrollbar, settings_view, status, text as text_area, tree_view};
use crate::app::{App, Overlay};
use crate::theme::Palette;

/// The tab bar's height. On macOS it is also the window's titlebar (see `main.rs`).
pub const BAR_H: f32 = 34.0;

/// Where macOS puts the end of the zoom button in a titlebar `BAR_H` tall: 16 points in,
/// three 14 point buttons on a 23 point pitch. Only the `mac` scene uses it. A real window
/// reads the width from AppKit.
const MAC_LIGHTS: f32 = 16.0 + 2.0 * 23.0 + 14.0;

/// What the window's own titlebar takes of the tab bar.
#[derive(Clone, Copy)]
pub struct Bar {
    /// The width the macOS traffic lights take at the start of the line. 0 in full screen
    /// and on Linux.
    pub lead: f32,
    /// Pressing an empty part of the line moves the window. Only where the app draws the
    /// titlebar, which is macOS. Linux keeps its own frame.
    pub drag: bool,
}

pub fn bar(app: &App, cx: &Cx<App>) -> Bar {
    if app.mac_titlebar {
        return Bar { lead: MAC_LIGHTS, drag: true };
    }
    let c = cx.chrome_insets();
    Bar { lead: c.leading, drag: c.height > 0.0 }
}

pub fn view(app: &App, p: &Palette, cx: &mut Cx<App>) -> El {
    let ui = font::ui(app);
    let top = bar(app, cx);
    let main = if app.on_settings { settings_view::view(app, p, cx) } else { editor_column(app, p, cx) };
    let mut body = row().grow(1.0).min_h(0.0).w_full();
    if app.tree.show && !app.on_settings {
        body = body.child(tree_view::tree(app, p, ui));
    }
    body = body.child(main);
    let mut root =
        col().w_full().h_full().min_h(0.0).child(chrome::tab_bar(app, p, ui, top)).child(body).child(status::status_line(app, p, ui));
    if app.is_vim() || app.ed.mode == Mode::Command || app.ed.msg.is_some() {
        root = root.child(status::command_line(app, p));
    }
    match &app.overlay {
        Overlay::Leader(group) => root = root.child(overlays::leader(app, p, *group)),
        Overlay::Pick(pk) => root = root.child(overlays::picker(app, p, pk)),
        Overlay::Menu(m) => root = root.child(overlays::menu(app, p, m)),
        Overlay::None => {}
    }
    root
}

/// The breadcrumbs over the text area and the scrollbar at its side.
fn editor_column(app: &App, p: &Palette, cx: &mut Cx<App>) -> El {
    let text = row().grow(1.0).min_h(0.0).w_full().child(text_area::text_area(app, p, cx)).child(scrollbar::scrollbar(app, p));
    col().grow(1.0).min_w(0.0).h_full().min_h(0.0).child(chrome::breadcrumbs(app, p)).child(text)
}
