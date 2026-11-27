//! The editor screen: the buffer line, the file tree, the text area (or Settings), the status
//! line and the command line, with the leader menu and pickers over them.

use editor::editor::{Keymap, Mode};
use rune::tok::TextStyle;
use rune::{Color, Cursor, Cx, Drag, El, Phase, Tx, col, el, row, spacer, text};

use super::{font, overlays, settings_view, text as text_area};
use crate::app::{App, Overlay};
use crate::highlight;
use crate::theme::Palette;

/// The buffer line's height. On macOS it is also the window's titlebar (see `main.rs`).
pub const BAR_H: f32 = 28.0;

/// Where macOS puts the end of the zoom button in a titlebar `BAR_H` tall: 16 points in,
/// three 14 point buttons on a 23 point pitch. Only the `mac` scene uses it. A real window
/// reads the width from AppKit.
const MAC_LIGHTS: f32 = 16.0 + 2.0 * 23.0 + 14.0;

/// What the window's own titlebar takes of the buffer line.
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
    let main = if app.on_settings { settings_view::view(app, p, cx) } else { text_area::text_area(app, p, cx) };
    let mut body = row().grow(1.0).min_h(0.0).w_full();
    if app.tree.show && !app.on_settings {
        body = body.child(tree(app, p, ui));
    }
    body = body.child(main);
    let mut root = col().w_full().h_full().min_h(0.0).child(buffer_line(app, p, ui, top)).child(body).child(status_line(app, p, ui));
    if app.is_vim() || app.ed.mode == Mode::Command || app.ed.msg.is_some() {
        root = root.child(command_line(app, p));
    }
    match &app.overlay {
        Overlay::Leader(group) => root = root.child(overlays::leader(app, p, *group)),
        Overlay::Pick(pk) => root = root.child(overlays::picker(app, p, pk)),
        Overlay::Menu(m) => root = root.child(overlays::menu(app, p, m)),
        Overlay::None => {}
    }
    root
}

fn buffer_line(app: &App, p: &Palette, ui: TextStyle, top: Bar) -> El {
    let mut line = row().h(BAR_H).w_full().fill(p.bg2).items_center().clip_rounded(0.0);
    if top.drag {
        line = line.window_drag();
    }
    if app.tree.show && !app.on_settings {
        // The tree's own column, so its edge lines up with the tree's. The project path starts
        // after the traffic lights.
        let root = crate::app::tilde(&app.ed.root);
        line = line.child(
            row()
                .w(app.tree.width)
                .h_full()
                .items_center()
                .pl(top.lead + 12.0)
                .pr(12.0)
                .border_sides([0.0, 1.0, 0.0, 0.0], p.bd)
                .child(text(root).font(ui).color(p.fg2).truncate()),
        );
    } else if top.lead > 0.0 {
        line = line.child(el().w(top.lead + 4.0).h_full());
    }
    for (i, d) in app.ed.docs.iter().enumerate() {
        let on = i == app.ed.active && !app.on_settings;
        let dirty = d.buf.is_dirty();
        let name = d.buf.name();
        let tab = row()
            .h_full()
            .items_center()
            .fill(if on { p.bg } else { Color::TRANSPARENT })
            .cursor(Cursor::Pointer)
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
            .child(el().w(3.0).h(16.0).ml(2.0).mr(8.0).fill(if on { p.ac } else { Color::TRANSPARENT }))
            .child(text(name).font(ui).color(if on { p.fg } else { p.fg2 }))
            .child(
                row()
                    .h_full()
                    .px(10.0)
                    .items_center()
                    .cursor(Cursor::Pointer)
                    .hover(p.fg.alpha(0.06))
                    .label("Close")
                    .on_press(move |s: &mut App| s.close_tab(i))
                    .child(text(if dirty { "●" } else { "×" }).font(ui).color(if dirty { p.cmd } else { p.fg2 })),
            );
        line = line.child(tab);
    }
    if app.settings.is_some() {
        let dirty = app.cfg != app.disk_cfg;
        let on = app.on_settings;
        line = line.child(
            row()
                .h_full()
                .items_center()
                .pr(12.0)
                .fill(if on { p.bg } else { Color::TRANSPARENT })
                .cursor(Cursor::Pointer)
                .on_press(|s: &mut App| s.on_settings = true)
                .child(el().w(3.0).h(16.0).ml(2.0).mr(8.0).fill(if on { p.ac } else { Color::TRANSPARENT }))
                .child(text(if dirty { "settings [+]" } else { "settings" }).font(ui).color(if on { p.fg } else { p.fg2 })),
        );
    }
    line
}

fn tree(app: &App, p: &Palette, ui: TextStyle) -> El {
    let rows = app.tree.index.rows(&app.tree.open);
    let active = if app.ed.has_doc() && !app.on_settings {
        app.ed.buf().path.as_ref().and_then(|pa| pa.strip_prefix(&app.ed.root).ok()).map(|r| r.to_string_lossy().replace('\\', "/"))
    } else {
        None
    };
    let visible = app.tree_visible();
    let start = app.tree_top(&rows);
    let mut list = col().w_full().pt(4.0);
    for r in rows.iter().skip(start).take(visible + 1) {
        let on = active.as_deref() == Some(r.path.as_str());
        let kb = app.tree.focus && app.tree.sel.as_deref() == Some(r.path.as_str());
        let letter = app.git_letter(r);
        let name_color = if r.dir {
            p.ac
        } else {
            match letter {
                Some('U') | Some('A') => p.ins,
                Some('M') => p.cmd,
                Some('D') => p.err,
                _ => p.fg,
            }
        };
        let guides = "│ ".repeat(r.depth);
        let chev = if r.dir { if r.open { "▾ " } else { "▸ " } } else { "  " };
        let path = r.path.clone();
        let dir = r.dir;
        let mut item = row()
            .h(crate::app::tree::ROW_H)
            .w_full()
            .items_center()
            .pl(8.0)
            .pr(12.0)
            .fill(if kb {
                p.sel
            } else if on {
                p.sel.alpha(0.6)
            } else {
                Color::TRANSPARENT
            })
            .hover(p.bg3)
            .cursor(Cursor::Pointer)
            .role("treeitem")
            .label(r.name.clone())
            .on_press(move |s: &mut App| {
                s.tree.focus = false;
                s.tree_press(&path, dir);
            })
            .child(text(guides).font(ui).color(p.bd))
            .child(text(chev).font(ui).color(p.fg2))
            .child(text(r.name.clone()).font(ui).color(name_color).truncate())
            .child(spacer());
        if let Some(l) = letter {
            let c = match l {
                'U' | 'A' => p.ins,
                'D' => p.err,
                _ => p.cmd,
            };
            item = item.child(text(l.to_string()).font(ui).color(c).pl(8.0));
        }
        list = list.child(item);
    }
    let width = app.tree.width;
    let edge = el().absolute(width - 3.0, 0.0).w(6.0).h_full().cursor(Cursor::ResizeColumn).on_drag(
        move |s: &mut App, d: &mut Drag, tx: &mut Tx| {
            if !s.cfg.mouse.enabled {
                return;
            }
            if d.phase == Phase::Down {
                d.data = s.tree.width;
            }
            s.tree.width = (d.data + d.dx).clamp(160.0, 640.0);
            if d.phase == Phase::Up {
                s.cfg.ui.tree_width = s.tree.width;
            }
            tx.instant();
        },
    );
    col()
        .w(width)
        .h_full()
        .min_h(0.0)
        .fill(p.bg2)
        .border_sides([0.0, 1.0, 0.0, 0.0], p.bd)
        .clip_rounded(0.0)
        .on_wheel(|s: &mut App, e, _tx| s.tree_wheel(e))
        .child(list)
        .child(edge)
}

fn status_line(app: &App, p: &Palette, ui: TextStyle) -> El {
    let small = TextStyle { size: (app.cfg.ui.size - 0.5).max(9.0), ..ui };
    let bold = TextStyle { weight: 700.0, ..small };
    let vim = app.ed.keymap == Keymap::Vim;
    let mode = app.ed.mode;
    let replacing = vim && editor::vim::replacing(&app.ed.vim);
    let confirming = vim && editor::vim::confirming(&app.ed.vim);
    let mode_color = match mode {
        _ if replacing => p.err,
        _ if confirming => p.cmd,
        Mode::Normal => p.ac,
        Mode::Insert => p.ins,
        Mode::Visual(_) => p.vis,
        Mode::Command => p.cmd,
    };
    let mode_label = if replacing {
        "REPLACE"
    } else if confirming {
        "CONFIRM"
    } else {
        mode.label()
    };
    let mut line = row().h(24.0).w_full().fill(p.bg2).items_center().clip_rounded(0.0);
    if app.on_settings {
        line = line.child(row().h_full().items_center().px(12.0).fill(p.cmd).child(text("SETTINGS").font(bold).color(p.on)));
    } else if vim {
        line = line.child(row().h_full().items_center().px(12.0).fill(mode_color).child(text(mode_label).font(bold).color(p.on)));
    }
    if let Some(b) = &app.git.branch {
        line = line.child(
            row()
                .h_full()
                .items_center()
                .px(12.0)
                .gap(6.0)
                .fill(p.bg3)
                .child(text("⎇").font(small).color(p.fg2))
                .child(text(b.clone()).font(small).color(p.fg)),
        );
    }
    let (file, lang, pct, pos) = if app.on_settings {
        let dirty = if app.cfg != app.disk_cfg { " [+]" } else { "" };
        (format!("config.toml{dirty}"), String::new(), String::new(), String::new())
    } else if app.ed.has_doc() {
        let d = app.ed.doc();
        let name = d.buf.path.as_deref().map(|pa| app.rel(pa)).unwrap_or_else(|| "[No Name]".into());
        let dirty = if d.buf.is_dirty() { " [+]" } else { "" };
        let conflict = d.buf.path.as_ref().and_then(|pa| app.meta.get(pa)).is_some_and(|m| m.conflict);
        let lang = d.buf.path.as_ref().and_then(|pa| app.meta.get(pa)).and_then(|m| m.lang).map(highlight::lang_label).unwrap_or("text");
        let total = d.buf.line_count().max(1);
        let line_no = d.view.cursor.line + 1;
        let pct = if d.view.top == 0 && total <= app.ed.rows {
            "All".to_string()
        } else if line_no <= 1 {
            "Top".into()
        } else if line_no >= total {
            "Bot".into()
        } else {
            format!("{}%", line_no * 100 / total)
        };
        let col = d.view.cursor.col + 1;
        let pos = if vim { format!("{line_no}:{col}") } else { format!("Ln {line_no}, Col {col}") };
        (format!("{name}{dirty}{}", if conflict { " [changed on disk]" } else { "" }), lang.to_string(), pct, pos)
    } else {
        ("[No Name]".into(), String::new(), String::new(), String::new())
    };
    line = line.child(row().h_full().items_center().px(12.0).child(text(file).font(small).color(p.fg).truncate()));
    line = line.child(spacer());
    if app.on_settings {
        let resolved = crate::theme::resolve(
            crate::theme::Source::parse(&app.cfg.theme.mode),
            &app.cfg.theme.dark,
            &app.cfg.theme.light,
            true,
            app.omarchy.as_ref(),
        )
        .1;
        line = line.child(text(format!("{} · {}", app.cfg.theme.mode, crate::theme::name(&resolved))).font(small).color(p.fg2).px(12.0));
        line = line.child(
            row()
                .h_full()
                .items_center()
                .px(12.0)
                .fill(p.ac)
                .cursor(Cursor::Pointer)
                .label("Write config.toml")
                .on_press(|s: &mut App| s.write_config())
                .child(text(":w").font(bold).color(p.on)),
        );
        return line;
    }
    if app.ed.has_doc() && !app.on_settings {
        let diags = app.ed.buf().path.as_ref().and_then(|pa| app.meta.get(pa)).map(|m| m.diags.as_slice()).unwrap_or(&[]);
        let errs = diags.iter().filter(|d| d.severity <= 1).count();
        let warns = diags.iter().filter(|d| d.severity == 2).count();
        if errs > 0 {
            line = line.child(text(format!("E {errs}")).font(small).color(p.err).px(12.0));
        }
        if warns > 0 {
            line = line.child(text(format!("W {warns}")).font(small).color(p.warn).px(12.0));
        }
    }
    if !app.proposals.is_empty() {
        line = line.child(
            text(format!("⇄ {} proposal{}", app.proposals.len(), if app.proposals.len() == 1 { "" } else { "s" }))
                .font(small)
                .color(p.vis)
                .px(12.0),
        );
    }
    if !app.outside.is_empty() {
        line = line.child(text(format!("Δ {}", app.outside.len())).font(small).color(p.vis).px(12.0));
    }
    if !lang.is_empty() {
        let enc = if app.ed.has_doc() { app.ed.buf().line_ending.label() } else { "unix" };
        line = line
            .child(text(lang).font(small).color(p.fg2).px(12.0))
            .child(text(format!("utf-8 · {enc}")).font(small).color(p.fg2).px(12.0))
            .child(text(pct).font(small).color(p.fg2).px(12.0));
    }
    if !pos.is_empty() {
        let (bg, fg, f) = if vim { (mode_color, p.on, bold) } else { (p.bg3, p.fg, small) };
        line = line.child(row().h_full().items_center().px(12.0).fill(bg).child(text(pos).font(f).color(fg)));
    }
    line
}

fn command_line(app: &App, p: &Palette) -> El {
    let style = font::mono(app, 12.5, 400.0);
    let bold = TextStyle { weight: 700.0, ..style };
    let mut line = row().h(22.0).w_full().items_center().px(8.0).fill(p.bg);
    if app.ed.mode == Mode::Command
        && let Some(c) = &app.ed.cmd
    {
        let before: String = c.text.chars().take(c.caret).collect();
        let after: String = c.text.chars().skip(c.caret).collect();
        return line
            .child(text(format!("{}{before}", c.kind)).font(style).color(p.fg))
            .child(el().w(1.5).h(15.0).fill(p.ac))
            .child(text(after).font(style).color(p.fg));
    }
    let (msg, s, color) = if let Some(m) = &app.ed.msg {
        (m.text.clone(), style, if m.error { p.err } else { p.fg2 })
    } else if app.on_settings {
        (":Settings".to_string(), style, p.fg2)
    } else {
        match app.ed.mode {
            Mode::Insert if app.is_vim() && editor::vim::replacing(&app.ed.vim) => ("-- REPLACE --".to_string(), bold, p.fg),
            Mode::Insert if app.is_vim() => ("-- INSERT --".to_string(), bold, p.fg),
            Mode::Visual(editor::editor::VisualKind::Line) => ("-- VISUAL LINE --".into(), bold, p.fg),
            Mode::Visual(editor::editor::VisualKind::Block) => ("-- VISUAL BLOCK --".into(), bold, p.fg),
            Mode::Visual(_) => ("-- VISUAL --".into(), bold, p.fg),
            _ if app.ed.has_doc() => {
                let d = app.ed.doc();
                let name = d.buf.path.as_deref().map(|pa| app.rel(pa)).unwrap_or_else(|| "[No Name]".into());
                (format!("\"{name}\" {}L, {}", d.buf.line_count(), crate::app::human_bytes(d.buf.len_bytes())), style, p.fg2)
            }
            _ => (String::new(), style, p.fg2),
        }
    };
    line = line.child(text(msg).font(s).color(color).truncate()).child(spacer());
    let pending = editor::vim::showcmd(&app.ed.vim);
    if matches!(app.overlay, Overlay::Leader(_)) {
        line = line.child(text("<space>").font(style).color(p.fg));
    } else if !pending.is_empty() {
        line = line.child(text(pending).font(style).color(p.fg));
    } else if app.is_vim() && matches!(app.overlay, Overlay::None) && app.ed.vim.leader == ' ' {
        line = line.child(
            row()
                .h_full()
                .items_center()
                .px(4.0)
                .cursor(Cursor::Pointer)
                .hover(p.fg.alpha(0.06))
                .on_press(|s: &mut App| s.overlay = Overlay::Leader(None))
                .child(text("<space> for keys").font(font::mono(app, 12.0, 400.0)).color(p.fg3)),
        );
    }
    line
}
