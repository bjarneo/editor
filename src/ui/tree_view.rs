//! The file tree: a chevron and an icon on each row, indent guides, and git status at the end.

use rune::tok::TextStyle;
use rune::{Color, Cursor, Drag, El, Phase, Tx, col, el, row, spacer, text};

use super::icons;
use crate::app::App;
use crate::app::tree::ROW_H;
use crate::theme::Palette;

/// How far each level steps in, and where the first level starts.
const INDENT: f32 = 14.0;
const LEFT: f32 = 6.0;

pub fn tree(app: &App, p: &Palette, ui: TextStyle) -> El {
    let rows = app.tree.index.rows(&app.tree.open);
    let active = if app.ed.has_doc() && !app.on_settings {
        app.ed.buf().path.as_ref().and_then(|pa| pa.strip_prefix(&app.ed.root).ok()).map(|r| r.to_string_lossy().replace('\\', "/"))
    } else {
        None
    };
    let small = TextStyle { size: (app.cfg.ui.size - 1.0).max(9.0), weight: 600.0, ..ui };
    let visible = app.tree_visible();
    let start = app.tree_top(&rows);
    let mut list = col().w_full().pt(4.0).px(4.0);
    for r in rows.iter().skip(start).take(visible + 1) {
        let on = active.as_deref() == Some(r.path.as_str());
        let kb = app.tree.focus && app.tree.sel.as_deref() == Some(r.path.as_str());
        let letter = app.git_letter(r);
        let git = match letter {
            Some('U') | Some('A') => Some(p.ins),
            Some('D') => Some(p.err),
            Some(_) => Some(p.cmd),
            None => None,
        };
        let name_color = git.filter(|_| !r.dir).unwrap_or(p.fg);
        let path = r.path.clone();
        let dir = r.dir;
        let mut item = row()
            .h(ROW_H)
            .w_full()
            .items_center()
            .pl(LEFT + r.depth as f32 * INDENT)
            .pr(8.0)
            .gap(5.0)
            .radius(4.0)
            .fill(if kb {
                p.sel
            } else if on {
                p.sel.alpha(0.6)
            } else {
                Color::TRANSPARENT
            })
            .hover(if kb || on { p.sel } else { p.bg3 })
            .cursor(Cursor::Pointer)
            .role("treeitem")
            .label(r.name.clone())
            .on_press(move |s: &mut App| {
                s.tree.focus = false;
                s.tree_press(&path, dir);
            });
        if kb {
            item = item.border(1.0, p.ac.alpha(0.7));
        }
        // Indent guides, one per level, under the chevron of the folder above.
        for d in 0..r.depth {
            item = item.child(el().absolute(LEFT + d as f32 * INDENT + 7.0, 0.0).w(1.0).h(ROW_H).fill(p.bd));
        }
        let (icon, tint) = if r.dir { (if r.open { &icons::FOLDER_OPEN } else { &icons::FOLDER }, p.ac) } else { icons::file(&r.name, p) };
        let chevron = if !r.dir {
            el().size(14.0, 14.0)
        } else if r.open {
            icons::draw(&icons::CHEVRON_DOWN, 14.0, p.fg2)
        } else {
            icons::draw(&icons::CHEVRON_RIGHT, 14.0, p.fg2)
        };
        item = item
            .child(chevron)
            .child(icons::draw(icon, 15.0, tint))
            .child(text(r.name.clone()).font(ui).color(name_color).truncate())
            .child(spacer());
        match (letter, git) {
            (Some(_), Some(c)) if r.dir => item = item.child(icons::draw(&icons::DOT, 12.0, c.alpha(0.8))),
            (Some(l), Some(c)) => item = item.child(text(l.to_string()).font(small).color(c)),
            _ => {}
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
        .on_layout(|s: &mut App, r| s.m.tree_h = r[3])
        .on_wheel(|s: &mut App, e, _tx| s.tree_wheel(e))
        .child(list)
        .child(edge)
}
