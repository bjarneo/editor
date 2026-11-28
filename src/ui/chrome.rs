//! The tab bar over the editor and the breadcrumbs over the text area.
//!
//! The tab bar sits on the chrome colour. The active tab takes the text area's colour, so it
//! joins the text below it, and a line along its top shows the mode colour.

use rune::tok::TextStyle;
use rune::{Color, Cursor, El, Span, el, row, spacer, text};

use super::editor_view::{BAR_H, Bar};
use super::{font, icons, status};
use crate::app::App;
use crate::theme::Palette;

/// The breadcrumbs' height.
pub const CRUMB_H: f32 = 26.0;

pub fn tab_bar(app: &App, p: &Palette, ui: TextStyle, top: Bar) -> El {
    let mut line = row().h(BAR_H).w_full().fill(p.bg2).items_center().clip_rounded(0.0);
    if top.drag {
        line = line.window_drag();
    }
    if app.tree.show && !app.on_settings {
        // The tree's own column, so its edge lines up with the tree's. The project path starts
        // after the traffic lights. The project's own folder shows bold.
        let root = crate::app::tilde(&app.ed.root);
        let cut = root.trim_end_matches('/').rfind('/').map_or(0, |i| i + 1);
        let spans = vec![Span::new(0, cut).color(p.fg3), Span::new(cut, root.len()).color(p.fg).bold()];
        line = line.child(
            row()
                .w(app.tree.width)
                .h_full()
                .items_center()
                .pl(top.lead + 12.0)
                .pr(12.0)
                .border_sides([0.0, 1.0, 0.0, 0.0], p.bd)
                .child(text(root).font(ui).color(p.fg2).spans(spans).truncate()),
        );
    } else if top.lead > 0.0 {
        line = line.child(el().w(top.lead + 4.0).h_full().border_sides([0.0, 0.0, 1.0, 0.0], p.bd));
    }
    let small = TextStyle { size: (app.cfg.ui.size - 1.0).max(9.0), ..ui };
    let mode = status::mode(app, p).0;
    let names: Vec<String> = app.ed.docs.iter().map(|d| d.buf.name()).collect();
    for (i, d) in app.ed.docs.iter().enumerate() {
        let on = i == app.ed.active && !app.on_settings;
        let name = names[i].clone();
        // Two tabs with one name show their folders.
        let folder = if names.iter().filter(|n| **n == name).count() > 1 {
            d.buf.path.as_deref().and_then(|pa| pa.parent()).and_then(|pa| pa.file_name()).map(|f| f.to_string_lossy().into_owned())
        } else {
            None
        };
        let (icon, tint) = icons::file(&name, p);
        let mut tab = tab(on, mode, p)
            .role("tab")
            .label(name.clone())
            .pointer_buttons([rune::PointerButton::Middle])
            .on_press_tx(move |s: &mut App, tx| {
                if tx.button() == rune::PointerButton::Middle {
                    s.close_tab(i);
                    return;
                }
                s.ed.active = i;
                s.on_settings = false;
                s.ed.mode = s.ed.rest_mode();
                s.apply_doc_opts();
            })
            .child(icons::draw(icon, 15.0, if on { tint } else { tint.alpha(0.7) }))
            .child(text(name).font(ui).color(if on { p.fg } else { p.fg2 }));
        if let Some(f) = folder {
            tab = tab.child(text(f).font(small).color(p.fg3));
        }
        let dirty = d.buf.is_dirty();
        tab = tab.child(close_button(dirty, on, p).on_press(move |s: &mut App| s.close_tab(i)));
        line = line.child(tab);
    }
    if app.settings.is_some() {
        let dirty = app.cfg != app.disk_cfg;
        let on = app.on_settings;
        line = line.child(
            tab(on, mode, p)
                .pr(14.0)
                .role("tab")
                .label("Settings")
                .on_press(|s: &mut App| s.on_settings = true)
                .child(icons::draw(&icons::SETTINGS, 15.0, if on { p.cmd } else { p.fg2 }))
                .child(text("Settings").font(ui).color(if on { p.fg } else { p.fg2 }))
                .child(if dirty { icons::draw(&icons::DOT, 14.0, p.fg2) } else { el() }),
        );
    }
    // The rest of the bar carries the line under the tabs.
    line.child(spacer().h_full().border_sides([0.0, 0.0, 1.0, 0.0], p.bd))
}

/// One tab's box. The active one has the text area's colour, no line under it and the mode
/// colour along its top.
fn tab(on: bool, mode: Color, p: &Palette) -> El {
    let mut t = row()
        .h_full()
        .items_center()
        .pl(12.0)
        .pr(6.0)
        .gap(7.0)
        .fill(if on { p.bg } else { Color::TRANSPARENT })
        .border_sides([0.0, 1.0, if on { 0.0 } else { 1.0 }, 0.0], p.bd)
        .cursor(Cursor::Pointer);
    if on {
        t = t.child(el().absolute(0.0, 0.0).w_full().h(2.0).fill(mode));
    } else {
        t = t.hover(p.fg.alpha(0.035));
    }
    t
}

/// The close button: a cross, or a dot while the buffer has changes.
fn close_button(dirty: bool, on: bool, p: &Palette) -> El {
    let glyph = if dirty {
        icons::draw(&icons::DOT, 16.0, if on { p.fg } else { p.fg2 })
    } else {
        icons::draw(&icons::CLOSE, 16.0, if on { p.fg2 } else { p.fg3 })
    };
    row()
        .size(22.0, 22.0)
        .items_center()
        .justify_center()
        .radius(5.0)
        .cursor(Cursor::Pointer)
        .hover(p.fg.alpha(0.1))
        .label("Close")
        .child(glyph)
}

/// The active file's path over the text area: one crumb per folder, then the file with its icon.
pub fn breadcrumbs(app: &App, p: &Palette) -> El {
    let ui = font::ui(app);
    let small = TextStyle { size: (app.cfg.ui.size - 0.5).max(9.0), ..ui };
    let mut line = row().h(CRUMB_H).w_full().px(14.0).gap(5.0).items_center().fill(p.bg).clip_rounded(0.0);
    if !app.ed.has_doc() {
        return line;
    }
    let buf = app.ed.buf();
    let path = buf.path.as_deref().map(|pa| app.rel(pa)).unwrap_or_else(|| "[No Name]".into());
    let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    for (i, part) in parts.iter().enumerate() {
        let last = i + 1 == parts.len();
        if i > 0 {
            line = line.child(icons::draw(&icons::CHEVRON_RIGHT, 12.0, p.fg3));
        }
        if last {
            let (icon, tint) = icons::file(part, p);
            line = line.child(icons::draw(icon, 14.0, tint));
        }
        line = line.child(text(part.to_string()).font(small).color(if last { p.fg } else { p.fg2 }));
    }
    line
}
