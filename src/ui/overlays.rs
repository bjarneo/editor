//! The leader key menu and the finder overlay (files, project search, buffers, commands).

use rune::tok::TextStyle;
use rune::{Color, Cursor, El, Span, col, el, row, spacer, text};

use super::{font, widgets};
use crate::app::keys::{LEADER, LEADER_AGENT, LEADER_GIT, LEADER_LSP};
use crate::app::menu::{ITEMS, Menu};
use crate::app::{App, Overlay, PickKind, Picker};
use crate::highlight;
use crate::theme::Palette;

/// A full-window press target that closes the overlay.
fn scrim() -> El {
    el().absolute(0.0, 0.0).w_full().h_full().label("Close").on_press(|s: &mut App| s.overlay = Overlay::None)
}

pub fn leader(app: &App, p: &Palette, group: Option<char>) -> El {
    let style = font::mono(app, 12.5, 400.0);
    let bold = TextStyle { weight: 700.0, ..style };
    let (title, items) = match group {
        Some('g') => ("SPC g", LEADER_GIT),
        Some('l') => ("SPC l", LEADER_LSP),
        Some('a') => ("SPC a", LEADER_AGENT),
        _ => (if app.ed.vim.leader == ' ' { "SPC" } else { "\\" }, LEADER),
    };
    let mut grid = row().w_full().flex_wrap().gap(0.0);
    for &(k, label, is_group) in items {
        grid = grid.child(
            row()
                .w(250.0)
                .h(22.0)
                .px(8.0)
                .gap(8.0)
                .items_center()
                .cursor(Cursor::Pointer)
                .hover(p.bg3)
                .role("menuitem")
                .label(label)
                .on_press_tx(move |s: &mut App, tx| s.key(editor::editor::KeyInput::ch(k), tx))
                .child(text(k.to_string()).font(bold).color(p.ac))
                .child(text("→").font(style).color(p.fg3))
                .child(text(label).font(style).color(if is_group { p.vis } else { p.fg })),
        );
    }
    let panel =
        widgets::titled(title, p.ac, style, p).w_full().pt(14.0).pb(10.0).px(8.0).shadow(12.0, 36.0, Color::BLACK.alpha(0.35)).child(grid);
    let bottom = if app.is_vim() || app.ed.msg.is_some() { 54.0 } else { 32.0 };
    col()
        .absolute(0.0, 0.0)
        .w_full()
        .h_full()
        .child(scrim())
        .child(col().absolute(0.0, 0.0).w_full().h_full().justify_end().px(12.0).pb(bottom).child(panel))
}

pub fn picker(app: &App, p: &Palette, pk: &Picker) -> El {
    let style = font::mono(app, 12.5, 400.0);
    let input_style = font::mono(app, 13.0, 400.0);
    let win_w = app.m.text[2] + if app.tree.show { app.tree.width } else { 0.0 };
    let w = (win_w - 32.0).clamp(320.0, 1040.0);
    let h = 560.0f32.min(app.m.text[3] + 40.0).max(200.0);
    let rows_fit = ((h - 40.0 - 14.0 - 16.0 - 20.0) / 22.0).floor().max(3.0) as usize;
    let grep = pk.kind == PickKind::Grep;
    let locs = pk.kind == PickKind::Locations;
    let n = pk.len(if locs { app.locations.len() } else { app.grep.hits.len() });
    let start = if pk.sel >= rows_fit { pk.sel + 1 - rows_fit } else { 0 };
    let mut list = col().w_full().grow(1.0).min_h(0.0).pt(10.0).pb(6.0).clip_rounded(0.0);
    for i in start..(start + rows_fit).min(n) {
        let on = i == pk.sel;
        let (dir, name, hits, extra) = row_text(app, pk, i);
        let mut spans = Vec::new();
        // Matched chars of the name are bold in the accent colour.
        for &h in &hits {
            if let Some((b, ch)) = name.char_indices().nth(h) {
                spans.push(Span::new(b, b + ch.len_utf8()).color(p.ac).bold());
            }
        }
        let mut r = row()
            .h(22.0)
            .w_full()
            .px(8.0)
            .items_center()
            .fill(if on { p.sel } else { Color::TRANSPARENT })
            .cursor(Cursor::Pointer)
            .role("option")
            .label(format!("{dir}{name}"))
            .on_press_tx(move |s: &mut App, tx| {
                if let Overlay::Pick(pk) = &mut s.overlay {
                    pk.sel = i;
                }
                s.pick_accept(tx);
            })
            .child(text(if on { "> " } else { "  " }).font(style).color(p.ac))
            .child(text(dir).font(style).color(p.fg2))
            .child(text(name).font(style).color(p.fg).spans(spans));
        if !extra.is_empty() {
            r = r.child(text(extra).font(style).color(p.fg2).pl(12.0).truncate());
        }
        list = list.child(r);
    }
    if n == 0 {
        let msg = if grep && app.grep.running {
            "searching…".to_string()
        } else if let Some(e) = &app.grep.error {
            e.clone()
        } else if grep && pk.query.is_empty() {
            "type to search the project".into()
        } else {
            "no matches".into()
        };
        list = list.child(text(msg).font(style).color(p.fg2).px(24.0));
    }
    let count = if locs {
        format!("{}", app.locations.len())
    } else if grep {
        format!("{}", app.grep.hits.len())
    } else {
        format!("{} / {}", pk.total, if pk.kind == PickKind::Files { app.files.len() } else { pk.items.len() })
    };
    let title = if locs { app.locations_title } else { pk.kind.title() };
    let input = widgets::titled(title, p.ac, style, p).h(40.0).w_full().child(
        row()
            .h_full()
            .w_full()
            .px(8.0)
            .items_center()
            .child(text(">").font(input_style).color(p.ac).w(18.0))
            .child(text(pk.query.clone()).font(input_style).color(p.fg))
            .child(el().w(1.5).h(16.0).fill(p.ac))
            .child(text(if pk.query.is_empty() { "  type to filter" } else { "" }).font(input_style).color(p.fg3))
            .child(spacer())
            .child(text(count).font(style).color(p.fg2)),
    );
    let results = widgets::titled("Results", p.ac, style, p).w_full().grow(1.0).min_h(0.0).child(list);
    let left = col()
        .w(if matches!(pk.kind, PickKind::Commands | PickKind::Actions) { w - 20.0 } else { (w * 0.42).max(300.0) })
        .h_full()
        .gap(14.0)
        .child(input)
        .child(results);
    let mut panel = row().w(w).h(h).gap(14.0).padding(10.0).radius(8.0).fill(p.bg).shadow(24.0, 70.0, Color::BLACK.alpha(0.45)).child(left);
    if !matches!(pk.kind, PickKind::Commands | PickKind::Actions) {
        panel = panel.child(preview(app, p, pk));
    }
    let x = ((win_w - w) / 2.0).max(16.0);
    col().absolute(0.0, 0.0).w_full().h_full().child(scrim()).child(el().absolute(x, 80.0f32.min(app.m.text[3] * 0.1 + 20.0)).child(panel))
}

/// The directory, the name, the matched char indices of the name, and any trailing text.
fn row_text(app: &App, pk: &Picker, i: usize) -> (String, String, Vec<usize>, String) {
    match pk.kind {
        PickKind::Grep => {
            let h = &app.grep.hits[i];
            (String::new(), format!("{}:{}", h.path, h.line + 1), Vec::new(), h.text.trim().chars().take(160).collect())
        }
        PickKind::Locations => {
            let l = &app.locations[i];
            (String::new(), format!("{}:{}", app.rel(&l.path), l.pos.line + 1), Vec::new(), l.text.trim().chars().take(160).collect())
        }
        _ => {
            let m = &pk.matches[i];
            let full = if pk.kind == PickKind::Files { app.files[m.index].clone() } else { pk.items[m.index].clone() };
            let split = if pk.kind == PickKind::Files { full.rfind('/').map_or(0, |i| i + 1) } else { 0 };
            let dir = full[..split].to_string();
            let name = full[split..].to_string();
            let dir_chars = dir.chars().count();
            let hits = m.hits.iter().filter(|&&h| h >= dir_chars).map(|h| h - dir_chars).collect();
            (dir, name, hits, String::new())
        }
    }
}

fn preview(app: &App, p: &Palette, pk: &Picker) -> El {
    let style = font::mono(app, 12.5, 400.0);
    let picked = app.picked();
    let title = picked.as_ref().map(|(pa, _)| app.rel(pa)).unwrap_or_else(|| "Preview".into());
    let mut body = col().w_full().grow(1.0).min_h(0.0).pt(12.0).clip_rounded(0.0);
    if let (Some((path, at)), Some((ppath, src))) = (&picked, &pk.preview)
        && path == ppath
    {
        let lang = highlight::lang_for(Some(path));
        let lines: Vec<&str> = src.split('\n').collect();
        let first = at.map_or(0, |a| a.line.saturating_sub(6));
        let shown: Vec<&str> = lines.iter().skip(first).take(40).copied().collect();
        let runs = highlight::runs_of(&lines[..(first + shown.len()).min(lines.len())].join("\n"), lang);
        for (j, l) in shown.iter().enumerate() {
            let li = first + j;
            let mut spans = Vec::new();
            if let Some(rs) = runs.get(li) {
                for (r, k) in rs {
                    let a = l.char_indices().nth(r.start).map_or(l.len(), |(b, _)| b);
                    let b = l.char_indices().nth(r.end).map_or(l.len(), |(b, _)| b);
                    if a < b {
                        spans.push(Span::new(a, b).color(highlight::color(p, *k)));
                    }
                }
            }
            let hit = at.is_some_and(|a| a.line == li);
            body = body.child(
                row()
                    .h(21.0)
                    .fill(if hit { p.sel } else { Color::TRANSPARENT })
                    .child(text((li + 1).to_string()).font(style).color(p.fg3).w(44.0).pr(14.0))
                    .child(text(l.to_string()).font(style).color(p.fg).spans(spans)),
            );
        }
    }
    widgets::titled(&title, p.ac, style, p).grow(1.0).min_w(0.0).h_full().child(body)
}

/// The in-app context menu at the pointer, kept inside the window.
pub fn menu(app: &App, p: &Palette, m: &Menu) -> El {
    let style = font::ui(app);
    let hint_style = TextStyle { size: (app.cfg.ui.size - 1.0).max(9.0), ..style };
    let vim = app.is_vim();
    let has_sel = app.selection_chars().is_some();
    let mut list = col().w(260.0).py(4.0).radius(6.0).fill(p.bg2).border(1.0, p.bd).shadow(10.0, 30.0, Color::BLACK.alpha(0.35));
    let mut h = 8.0;
    for (i, &(id, title, vk, ck)) in ITEMS.iter().enumerate() {
        if id == "-" {
            list = list.child(el().h(1.0).w_full().my(4.0).fill(p.bd));
            h += 9.0;
            continue;
        }
        let enabled = has_sel || !matches!(id, "cut" | "copy");
        let on = i == m.sel;
        let mut row_el = row()
            .h(24.0)
            .px(12.0)
            .items_center()
            .fill(if on { p.sel } else { Color::TRANSPARENT })
            .hover(p.bg3)
            .role("menuitem")
            .label(title)
            .child(text(title).font(style).color(if enabled { p.fg } else { p.fg3 }))
            .child(spacer())
            .child(text(if vim { vk } else { ck }).font(hint_style).color(p.fg3));
        if enabled {
            row_el = row_el.cursor(Cursor::Pointer).on_press_tx(move |s: &mut App, tx| s.menu_pick(id, tx));
        }
        list = list.child(row_el);
        h += 24.0;
    }
    let win_w = app.m.text[0] + app.m.text[2];
    let win_h = app.m.text[1] + app.m.text[3] + 46.0;
    let x = m.x.min(win_w - 268.0).max(4.0);
    let y = if m.y + h > win_h { (m.y - h).max(4.0) } else { m.y };
    col().absolute(0.0, 0.0).w_full().h_full().child(scrim()).child(el().absolute(x, y).child(list))
}
