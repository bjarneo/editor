//! The text area: gutter signs, line numbers, the code with its colours, the selection, search
//! matches and the cursor. Only the lines on screen are built.

use editor::editor::{Keymap, Mode, VisualKind};
use rune::{Color, Cx, El, Span, TextHighlight, col, el, row, text};
use unicode_width::UnicodeWidthChar;

use super::font;
use crate::app::App;
use crate::highlight::{self, Kind};
use crate::services::git::SignKind;
use crate::theme::Palette;

/// Chars of the probe that measures the font's advance.
const PROBE: usize = 100;

/// Gutter width in chars: 2 for signs, then numbers right-aligned with 2 chars after them.
pub fn gutter_cols(app: &App) -> usize {
    if app.cfg.editor.numbers == "off" {
        return 2;
    }
    let lines = if app.ed.has_doc() { app.ed.buf().line_count() } else { 1 };
    let digits = lines.to_string().len().max(3);
    2 + digits + 1 + 2
}

/// A line as drawn: the display text (tabs expanded, dots for leading spaces), where each
/// source char starts in it as a byte offset, and its display column.
struct Shown {
    text: String,
    byte: Vec<usize>,
    col: Vec<usize>,
}

fn show(line: &str, from: usize, max_cols: usize, tab: usize, dots: bool) -> Shown {
    let mut text = String::new();
    let mut byte = Vec::new();
    let mut colv = Vec::new();
    let mut c = 0usize;
    let mut leading = true;
    for (i, ch) in line.chars().enumerate() {
        if ch != ' ' && ch != '\t' {
            leading = false;
        }
        if i < from {
            continue;
        }
        if c > max_cols {
            break;
        }
        byte.push(text.len());
        colv.push(c);
        match ch {
            '\t' => {
                let w = tab - (c % tab.max(1));
                if dots && leading {
                    text.push('→');
                    text.extend(std::iter::repeat_n(' ', w.saturating_sub(1)));
                } else {
                    text.extend(std::iter::repeat_n(' ', w));
                }
                c += w;
            }
            ' ' if dots && leading => {
                text.push('·');
                c += 1;
            }
            ch => {
                text.push(ch);
                c += ch.width().unwrap_or(1).max(1);
            }
        }
    }
    byte.push(text.len());
    colv.push(c);
    Shown { text, byte, col: colv }
}

/// The selected chars of `line` as `[a, b)` in chars, and whether it runs past the line end.
fn selected(app: &App, line: usize, len: usize) -> Option<(usize, usize, bool)> {
    let doc = app.ed.doc();
    let anchor = doc.view.anchor?;
    let cur = doc.view.cursor;
    let (s, e) = if anchor <= cur { (anchor, cur) } else { (cur, anchor) };
    if line < s.line || line > e.line {
        return None;
    }
    match (app.ed.keymap, app.ed.mode) {
        (Keymap::Vim, Mode::Visual(VisualKind::Line)) => Some((0, len, true)),
        (Keymap::Vim, Mode::Visual(VisualKind::Block)) => {
            let (c0, c1) = (anchor.col.min(cur.col), anchor.col.max(cur.col));
            Some((c0.min(len), (c1 + 1).min(len), false))
        }
        (Keymap::Vim, Mode::Visual(VisualKind::Char)) => {
            let a = if line == s.line { s.col } else { 0 };
            let (b, past) = if line == e.line { ((e.col + 1).min(len), e.col >= len) } else { (len, true) };
            Some((a.min(len), b, past))
        }
        (Keymap::Classic, _) => {
            let a = if line == s.line { s.col } else { 0 };
            let (b, past) = if line == e.line { (e.col.min(len), false) } else { (len, true) };
            (a < b || past).then_some((a.min(len), b, past))
        }
        _ => None,
    }
}

pub fn text_area(app: &App, p: &Palette, _cx: &mut Cx<App>) -> El {
    let style = font::code(app);
    let rh = font::row_height(app);
    let cw = app.m.cw;
    let gcols = gutter_cols(app);
    let gutter = gcols as f32 * cw;
    let mut area = col()
        .key("text-area")
        .grow(1.0)
        .min_w(0.0)
        .h_full()
        .min_h(0.0)
        .fill(p.bg)
        .clip_rounded(0.0)
        .on_layout(|s: &mut App, r| s.text_rect(r))
        .on_drag(|s: &mut App, d, tx| s.text_drag(d, tx))
        .on_wheel(|s: &mut App, e, tx| s.text_wheel(e, tx))
        .on_secondary_press_local(|s: &mut App, e, tx| s.context_menu(e, tx));
    // Measures one char of the editor font; nothing shows.
    area = area.child(
        el().absolute(0.0, 0.0)
            .opacity(0.0)
            .child(text("0".repeat(PROBE)).key("cw-probe").font(style).on_layout(|s: &mut App, r| s.char_width(r[2] / PROBE as f32))),
    );
    if !app.ed.has_doc() {
        return area.child(empty_state(app, p));
    }
    let doc = app.ed.doc();
    let buf = &doc.buf;
    let path = buf.path.as_deref();
    let meta = path.and_then(|pa| app.meta.get(pa));
    let lang = meta.and_then(|m| m.lang).unwrap_or(rune_highlights::Lang::PLAIN);
    let rows = app.ed.rows.max(1);
    let top = doc.view.top.min(buf.last_line());
    let left = if app.cfg.editor.wrap { 0 } else { doc.view.left };
    let cols = app.ed.cols.max(10);
    let cur = doc.view.cursor;
    let count = rows.min(buf.line_count().saturating_sub(top));
    let runs = highlight::runs(path_id(path), buf, lang, top, count);
    let vim = app.ed.keymap == Keymap::Vim;
    let mode = app.ed.mode;
    let numbers = if vim {
        app.cfg.editor.numbers.as_str()
    } else if app.cfg.editor.numbers == "off" {
        "off"
    } else {
        "absolute"
    };
    let search = if app.ed.search.pattern.is_empty() || !app.search_shown() {
        Vec::new()
    } else {
        editor::vim::search_matches(buf, &app.ed.search.pattern, top, top + count)
    };
    let signs = meta.map(|m| m.signs.as_slice()).unwrap_or(&[]);
    let diags = meta.map(|m| m.diags.as_slice()).unwrap_or(&[]);
    let mut cursor_at: Option<(f32, f32)> = None;
    let replacing = vim && editor::vim::replacing(&app.ed.vim);
    let cursor_shape = if replacing {
        "underline"
    } else if mode == Mode::Insert || !vim {
        "bar"
    } else if matches!(mode, Mode::Visual(_)) {
        "block"
    } else {
        app.cfg.editor.cursor.as_str()
    };
    let show_cursor = mode != Mode::Command && !app.tree.focus;
    let mut drawn = 0usize;
    let mut line = top;
    while drawn < rows {
        if line > buf.last_line() {
            area = area.child(row().h(rh).child(el().w(gutter)).child(text("~").font(style).color(p.bd)));
            drawn += 1;
            continue;
        }
        let full = buf.line(line);
        let len = full.chars().count();
        let pieces: Vec<usize> = if app.cfg.editor.wrap && len > cols { (0..len).step_by(cols).collect() } else { vec![left] };
        for (pi, &from) in pieces.iter().enumerate() {
            if drawn >= rows {
                break;
            }
            let max = if app.cfg.editor.wrap { cols.saturating_sub(1) } else { cols + 1 };
            let shown = show(&full, from, max, app.ed.opts.tab_width, app.cfg.editor.whitespace);
            let n = shown.byte.len() - 1;
            let to_byte = |c: usize| -> usize { shown.byte[c.saturating_sub(from).min(n)] };
            let in_piece = |c: usize| c >= from && c <= from + n;
            let is_cur = line == cur.line;
            // Syntax colours.
            let mut spans: Vec<Span> = Vec::new();
            for (r, k) in &runs[line - top] {
                if r.end <= from || r.start >= from + n {
                    continue;
                }
                let (a, b) = (to_byte(r.start.max(from)), to_byte(r.end.min(from + n)));
                if a < b {
                    let mut sp = Span::new(a, b).color(highlight::color(p, *k));
                    if *k == Kind::Comment {
                        sp = sp.italic();
                    }
                    spans.push(sp);
                }
            }
            if app.cfg.editor.whitespace {
                for (i, ch) in shown.text.char_indices() {
                    if ch == '·' || ch == '→' {
                        spans.push(Span::new(i, i + ch.len_utf8()).color(p.bd));
                    }
                }
                spans.sort_by_key(|s| s.start);
            }
            for d in diags.iter().filter(|d| d.line <= line && d.end_line >= line) {
                let a = if d.line == line { d.col } else { 0 };
                let b = if d.end_line == line { d.end_col.max(a + 1) } else { len };
                let (a, b) = (to_byte(a.max(from)), to_byte(b.min(from + n)));
                if a < b {
                    spans.push(Span::new(a, b).underline(if d.severity <= 1 { p.err } else { p.warn }));
                }
            }
            spans.sort_by_key(|s| s.start);
            let mut code = text(shown.text.clone()).font(style).color(p.fg).spans(spans);
            // Search matches, then the selection over them.
            for &(a, b) in &search {
                let (pa, pb) = (buf.pos(a), buf.pos(b));
                if pa.line == line && in_piece(pa.col) {
                    let end = if pb.line == line { pb.col } else { len };
                    code = code.text_highlight(TextHighlight::new(to_byte(pa.col)..to_byte(end), p.warn.alpha(0.28)).pad(0.0));
                }
            }
            let sel = selected(app, line, len);
            let mut row_fill =
                if is_cur && app.cfg.editor.cursorline && pi == 0 && sel.is_none() && show_cursor { p.bg3 } else { Color::TRANSPARENT };
            let mut code_fill = Color::TRANSPARENT;
            if let Some((a, b, past)) = sel {
                if a == 0 && b >= len && past {
                    code_fill = p.sel;
                } else if b > a {
                    code = code.text_highlight(TextHighlight::new(to_byte(a.max(from))..to_byte(b.min(from + n)), p.sel).pad(0.0));
                }
                row_fill = Color::TRANSPARENT;
            }
            // Extra cursors' selections (classic multiple cursors).
            for c in &doc.extra {
                let Some(anchor) = c.anchor else { continue };
                let (s0, e0) = if anchor <= c.pos { (anchor, c.pos) } else { (c.pos, anchor) };
                if line < s0.line || line > e0.line {
                    continue;
                }
                let a = if line == s0.line { s0.col } else { 0 };
                let b = if line == e0.line { e0.col } else { len };
                if b > a {
                    code = code.text_highlight(TextHighlight::new(to_byte(a.max(from))..to_byte(b.min(from + n)), p.sel).pad(0.0));
                }
            }
            // Gutter.
            let sign = if pi == 0 { signs.iter().find(|s| s.line == line) } else { None };
            let diag = if pi == 0 { diags.iter().filter(|d| d.line == line).min_by_key(|d| d.severity) } else { None };
            let (sign_text, sign_color) = match (diag, sign.map(|s| s.kind)) {
                (Some(d), _) if d.severity <= 1 => ("E", p.err),
                (Some(d), _) if d.severity == 2 => ("W", p.warn),
                (Some(_), _) => ("I", p.fg2),
                (None, Some(SignKind::Added)) => ("▎", p.ins),
                (None, Some(SignKind::Modified)) => ("▎", p.cmd),
                (None, Some(SignKind::Deleted)) => ("▁", p.err),
                (None, None) => ("", p.fg3),
            };
            let num = if pi > 0 || numbers == "off" {
                String::new()
            } else {
                match numbers {
                    "relative" => line.abs_diff(cur.line).to_string(),
                    "absolute" => (line + 1).to_string(),
                    _ if is_cur => (line + 1).to_string(),
                    _ => line.abs_diff(cur.line).to_string(),
                }
            };
            let num_style = if is_cur && pi == 0 { rune::tok::TextStyle { weight: 700.0, ..style } } else { style };
            let mut gut =
                row().w(gutter).h(rh).child(row().w(2.0 * cw).justify_center().child(text(sign_text).font(style).color(sign_color)));
            if numbers != "off" {
                gut = gut.child(row().w((gcols - 4) as f32 * cw).justify_end().child(text(num).font(num_style).color(if is_cur {
                    p.fg
                } else {
                    p.fg3
                })));
            }
            let mut code_box = row().grow(1.0).min_w(0.0).h(rh).fill(code_fill).child(code);
            if let Some(d) = diag.filter(|d| d.severity <= 2 && pi + 1 == pieces.len()) {
                let msg: String = d.message.chars().take(160).collect();
                code_box =
                    code_box.child(text(format!("    ■ {msg}")).font(style).color(if d.severity <= 1 { p.err } else { p.warn }).truncate());
            }
            // The cursor.
            if is_cur && show_cursor && in_piece(cur.col) && (pi + 1 == pieces.len() || cur.col < from + n) {
                let x = shown.col[cur.col.saturating_sub(from).min(n)] as f32 * cw;
                let ch = full.chars().nth(cur.col).filter(|c| *c != '\t').unwrap_or(' ');
                let w = ch.width().unwrap_or(1).max(1) as f32 * cw;
                let cursor = match cursor_shape {
                    "bar" => el().absolute(x - 1.0, 2.0).size(2.0, rh - 4.0).fill(if vim { p.ins } else { p.ac }),
                    "underline" => el().absolute(x, rh - 2.0).size(w, 2.0).fill(p.fg),
                    _ => {
                        let fill = if matches!(mode, Mode::Visual(_)) { p.vis } else { p.fg };
                        let ink = if matches!(mode, Mode::Visual(_)) { p.on } else { p.bg };
                        el().absolute(x, 0.0).size(w, rh).fill(fill).child(text(ch.to_string()).font(style).color(ink))
                    }
                };
                code_box = code_box.child(cursor.key("cursor"));
                cursor_at = Some((gutter + x, drawn as f32 * rh));
            }
            if show_cursor {
                for c in doc.extra.iter().filter(|c| c.pos.line == line && in_piece(c.pos.col)) {
                    let x = shown.col[c.pos.col.saturating_sub(from).min(n)] as f32 * cw;
                    code_box = code_box.child(el().absolute(x - 1.0, 2.0).size(2.0, rh - 4.0).fill(p.ac.alpha(0.85)));
                }
            }
            area = area.child(row().h(rh).w_full().fill(row_fill).child(gut).child(code_box));
            drawn += 1;
        }
        line += 1;
    }
    // The ruler.
    let ruler = app.cfg.editor.ruler as usize;
    if ruler > left && !app.cfg.editor.wrap {
        area = area.child(el().absolute(gutter + (ruler - left) as f32 * cw, 0.0).w(1.0).h_full().fill(p.bd.alpha(0.7)));
    }
    if let Some((x, y)) = cursor_at {
        let below = y < app.m.text[3] * 0.6;
        if let Some(c) = &app.completion {
            let start_x = x - (cur.col.saturating_sub(c.start.col)) as f32 * cw;
            area = area.child(completion(app, p, c, start_x - 8.0, y, rh, below));
        } else if let Some(h) = &app.hover {
            area = area.child(hover(app, p, h, x, y, rh, below));
        }
    }
    area
}

fn completion(app: &App, p: &Palette, c: &crate::app::lsp::Completion, x: f32, y: f32, rh: f32, below: bool) -> El {
    let style = font::code(app);
    let small = rune::tok::TextStyle { size: (app.cfg.font.size - 1.0).max(9.0), line_height: 0.0, ..style };
    let typed = c.start.col.abs_diff(app.ed.cursor().col);
    let mut list =
        col().min_w(36.0 * app.m.cw).py(3.0).radius(6.0).fill(p.bg2).border(1.0, p.bd).shadow(10.0, 30.0, Color::BLACK.alpha(0.3));
    for (j, &i) in c.shown.iter().enumerate() {
        let it = &c.items[i];
        let head: String = it.label.chars().take(typed).collect();
        let rest: String = it.label.chars().skip(typed).collect();
        let kind_color = match it.kind {
            "Function" | "Method" => p.fun,
            "Struct" | "Enum" | "Class" | "Interface" | "TypeParam" => p.ty,
            "Keyword" => p.kw,
            "Constant" | "EnumMember" => p.num,
            _ => p.fg2,
        };
        list = list.child(
            row()
                .h(22.0)
                .px(8.0)
                .gap(8.0)
                .items_center()
                .fill(if j == c.sel { p.sel } else { Color::TRANSPARENT })
                .child(text(head).font(rune::tok::TextStyle { weight: 700.0, line_height: 0.0, ..style }).color(p.ac))
                .child(text(rest).font(rune::tok::TextStyle { line_height: 0.0, ..style }).color(p.fg))
                .child(rune::spacer())
                .child(text(it.kind).font(small).color(kind_color).pl(16.0)),
        );
    }
    let h = c.shown.len() as f32 * 22.0 + 8.0;
    let top = if below { y + rh } else { (y - h).max(0.0) };
    el().absolute(x.max(0.0), top).child(list)
}

fn hover(app: &App, p: &Palette, h: &str, x: f32, y: f32, rh: f32, below: bool) -> El {
    let style = rune::tok::TextStyle { line_height: 0.0, ..font::code(app) };
    let lines: Vec<&str> = h.lines().filter(|l| !l.trim_start().starts_with("```")).take(14).collect();
    let mut b = col().max_w(80.0 * app.m.cw).px(10.0).py(8.0).gap(2.0).radius(6.0).fill(p.bg2).border(1.0, p.bd).shadow(
        10.0,
        30.0,
        Color::BLACK.alpha(0.3),
    );
    for l in &lines {
        b = b.child(text(l.to_string()).font(style).color(p.fg).wrap());
    }
    let h = lines.len() as f32 * (app.cfg.font.size * 1.3) + 18.0;
    let top = if below { y + rh } else { (y - h).max(0.0) };
    el().absolute((x - 10.0).max(0.0), top).child(b)
}

/// A cache slot for a buffer's highlights.
fn path_id(path: Option<&std::path::Path>) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut h);
    h.finish()
}

fn empty_state(app: &App, p: &Palette) -> El {
    let style = font::ui(app);
    let lines: Vec<(&str, String)> = if app.is_vim() {
        let l = if app.ed.vim.leader == ' ' { "SPC" } else { "\\" };
        vec![
            ("find a file", format!("{l} f")),
            ("search the project", format!("{l} /")),
            ("every key", l.to_string()),
            ("settings", ":Settings".into()),
        ]
    } else {
        let m = if cfg!(target_os = "macos") { "⌘" } else { "Ctrl" };
        vec![
            ("find a file", format!("{m} P")),
            ("search the project", format!("{m} ⇧ F")),
            ("commands", format!("{m} ⇧ P")),
            ("settings", format!("{m} ,")),
        ]
    };
    let mut c = col().gap(10.0).child(text("no file open").font(font::mono(app, 15.0, 700.0)).color(p.fg));
    for (what, keys) in lines {
        c = c.child(row().gap(16.0).child(text(what).font(style).color(p.fg2).w(180.0)).child(text(keys).font(style).color(p.ac)));
    }
    col().grow(1.0).w_full().h_full().center().child(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tabs_expand_and_leading_spaces_become_dots() {
        let s = show("\tab", 0, 80, 4, false);
        assert_eq!(s.text, "    ab");
        assert_eq!(s.col, vec![0, 4, 5, 6]);
        let s = show("  x", 0, 80, 4, true);
        assert_eq!(s.text, "··x");
        let s = show("abcdef", 2, 80, 4, false);
        assert_eq!(s.text, "cdef");
        assert_eq!(s.byte, vec![0, 1, 2, 3, 4]);
    }
}
