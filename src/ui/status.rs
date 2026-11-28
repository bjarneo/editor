//! The status line and the command line under it.
//!
//! The status line keeps Neovim's order: the mode, the branch and the file on the left, the
//! file type and the position on the right. The mode and the position are pills in the mode
//! colour.

use editor::editor::{Keymap, Mode};
use rune::tok::TextStyle;
use rune::{Color, Cursor, El, el, row, spacer, text};

use super::{font, icons};
use crate::app::{App, Overlay};
use crate::highlight;
use crate::theme::Palette;

/// The status line's height.
pub const STATUS_H: f32 = 26.0;

/// The colour and the name of the current mode. Replace and the `:s///c` prompt have their
/// own.
pub fn mode(app: &App, p: &Palette) -> (Color, &'static str) {
    let vim = app.ed.keymap == Keymap::Vim;
    if vim && editor::vim::replacing(&app.ed.vim) {
        return (p.err, "REPLACE");
    }
    if vim && editor::vim::confirming(&app.ed.vim) {
        return (p.cmd, "CONFIRM");
    }
    let color = match app.ed.mode {
        Mode::Normal => p.ac,
        Mode::Insert => p.ins,
        Mode::Visual(_) => p.vis,
        Mode::Command => p.cmd,
    };
    (color, app.ed.mode.label())
}

/// A filled pill with bold text, such as the mode.
fn pill(label: impl Into<String>, fill: Color, style: TextStyle, p: &Palette) -> El {
    row().h(18.0).px(8.0).items_center().radius(4.0).fill(fill).child(text(label.into()).font(style).color(p.on))
}

/// A status item: an optional icon, then text.
fn item(icon: Option<(&'static rune::Icon, Color)>, label: impl Into<String>, color: Color, style: TextStyle) -> El {
    let mut r = row().h_full().px(8.0).gap(5.0).items_center();
    if let Some((i, c)) = icon {
        r = r.child(icons::draw(i, 14.0, c));
    }
    r.child(text(label.into()).font(style).color(color))
}

pub fn status_line(app: &App, p: &Palette, ui: TextStyle) -> El {
    let small = TextStyle { size: (app.cfg.ui.size - 0.5).max(9.0), ..ui };
    let bold = TextStyle { weight: 700.0, ..small };
    let vim = app.ed.keymap == Keymap::Vim;
    let (mode_color, mode_label) = mode(app, p);
    let mut line = row().h(STATUS_H).w_full().px(4.0).fill(p.bg2).items_center().border_sides([1.0, 0.0, 0.0, 0.0], p.bd).clip_rounded(0.0);
    if app.on_settings {
        line = line.child(pill("SETTINGS", p.cmd, bold, p).mr(4.0));
    } else if vim {
        line = line.child(pill(mode_label, mode_color, bold, p).mr(4.0));
    }
    if let Some(b) = &app.git.branch {
        line = line.child(item(Some((&icons::BRANCH, p.fg2)), b.clone(), p.fg2, small));
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
    line = line.child(row().h_full().items_center().px(8.0).child(text(file).font(small).color(p.fg).truncate()));
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
        line = line.child(item(None, format!("{} · {}", app.cfg.theme.mode, crate::theme::name(&resolved)), p.fg2, small));
        return line.child(
            pill(":w", p.ac, bold, p).ml(4.0).cursor(Cursor::Pointer).label("Write config.toml").on_press(|s: &mut App| s.write_config()),
        );
    }
    if app.ed.has_doc() {
        let diags = app.ed.buf().path.as_ref().and_then(|pa| app.meta.get(pa)).map(|m| m.diags.as_slice()).unwrap_or(&[]);
        let errs = diags.iter().filter(|d| d.severity <= 1).count();
        let warns = diags.iter().filter(|d| d.severity == 2).count();
        if errs > 0 {
            line = line.child(item(Some((&icons::ERROR, p.err)), errs.to_string(), p.err, small));
        }
        if warns > 0 {
            line = line.child(item(Some((&icons::WARNING, p.warn)), warns.to_string(), p.warn, small));
        }
    }
    if !app.proposals.is_empty() {
        let n = app.proposals.len();
        line = line.child(item(Some((&icons::DIFF, p.vis)), format!("{n} proposal{}", if n == 1 { "" } else { "s" }), p.vis, small));
    }
    if !app.outside.is_empty() {
        line = line.child(item(Some((&icons::PENCIL, p.vis)), app.outside.len().to_string(), p.vis, small));
    }
    if !lang.is_empty() {
        let enc = if app.ed.has_doc() { app.ed.buf().line_ending.label() } else { "unix" };
        line = line
            .child(item(None, lang, p.fg2, small))
            .child(item(None, format!("utf-8 · {enc}"), p.fg2, small))
            .child(item(None, pct, p.fg2, small));
    }
    if !pos.is_empty() {
        line = if vim { line.child(pill(pos, mode_color, bold, p).ml(4.0)) } else { line.child(item(None, pos, p.fg, small)) };
    }
    line
}

pub fn command_line(app: &App, p: &Palette) -> El {
    let style = font::mono(app, 12.5, 400.0);
    let bold = TextStyle { weight: 700.0, ..style };
    let mut line = row().h(22.0).w_full().items_center().px(10.0).fill(p.bg);
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
