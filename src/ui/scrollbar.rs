//! The scrollbar at the right of the text area. It is also an overview of the whole file: git
//! changes in a lane on the left, errors and warnings on the right, and the cursor's line
//! across it. Drag the thumb or press the track to scroll. The wheel scrolls as over the text.

use rune::{Color, Cursor, El, col, el};

use crate::app::App;
use crate::services::git::SignKind;
use crate::theme::Palette;

/// The scrollbar's width.
pub const W: f32 = 14.0;

/// The lines the track stands for. Past the last line the view can still scroll until that
/// line is at the top, so a long file counts `rows - 1` more. A file that fits counts the
/// rows, so each mark sits beside its line.
fn span(total: usize, rows: usize) -> f32 {
    if total <= rows { rows.max(1) as f32 } else { (total + rows - 1) as f32 }
}

/// The thumb's top and height on a track `h` tall, or `None` when the file fits.
pub fn thumb(top: usize, rows: usize, total: usize, h: f32) -> Option<(f32, f32)> {
    if total <= rows || h <= 0.0 {
        return None;
    }
    let s = span(total, rows);
    let th = (h * rows as f32 / s).max(24.0).min(h);
    let y = (h * top as f32 / s).min(h - th);
    Some((y, th))
}

/// The first line on screen for a thumb whose top is at `y` on a track `h` tall.
pub fn top_at(y: f32, rows: usize, total: usize, h: f32) -> usize {
    let s = span(total, rows);
    ((y.max(0.0) / h.max(1.0)) * s).round().clamp(0.0, total.saturating_sub(1) as f32) as usize
}

/// Marks for runs of lines: each `(line, colour)` joins the mark above it when it is the same
/// colour and the marks touch. A file with thousands of changes still draws a few hundred.
fn marks(mut lines: Vec<(usize, Color)>, x: f32, w: f32, s: f32, h: f32, out: &mut Vec<El>) {
    lines.sort_by_key(|l| l.0);
    let step = h / s;
    let mut run: Option<(f32, f32, Color)> = None;
    for (line, c) in lines {
        let y0 = line as f32 * step;
        let y1 = y0 + step.max(2.0);
        match &mut run {
            Some((_, end, rc)) if *rc == c && y0 <= *end + 1.0 => *end = end.max(y1),
            _ => {
                if let Some((a, b, rc)) = run.take() {
                    out.push(el().absolute(x, a).size(w, b - a).radius(1.0).fill(rc));
                }
                run = Some((y0, y1, c));
            }
        }
    }
    if let Some((a, b, rc)) = run {
        out.push(el().absolute(x, a).size(w, b - a).radius(1.0).fill(rc));
    }
}

pub fn scrollbar(app: &App, p: &Palette) -> El {
    let mut bar = col()
        .w(W)
        .h_full()
        .fill(p.bg)
        .clip_rounded(0.0)
        .on_drag(|s: &mut App, d, tx| s.scrollbar_drag(d, tx))
        .on_wheel(|s: &mut App, e, tx| s.text_wheel(e, tx));
    if !app.ed.has_doc() || app.proposal_here().is_some() {
        return bar;
    }
    let doc = app.ed.doc();
    let total = doc.buf.line_count().max(1);
    let rows = app.ed.rows.max(1);
    let h = app.m.text[3];
    let s = span(total, rows);
    let meta = doc.buf.path.as_deref().and_then(|pa| app.meta.get(pa));
    let mut out = Vec::new();
    if let Some(m) = meta {
        let git = m.signs.iter().map(|g| {
            let c = match g.kind {
                SignKind::Added => p.ins,
                SignKind::Modified => p.cmd,
                SignKind::Deleted => p.err,
            };
            (g.line, c.alpha(0.85))
        });
        let outside = m.outside.iter().map(|&l| (l, p.vis.alpha(0.85)));
        marks(git.chain(outside).collect(), 2.0, 3.0, s, h, &mut out);
        let diags = m.diags.iter().filter(|d| d.severity <= 2).map(|d| (d.line, if d.severity <= 1 { p.err } else { p.warn }));
        marks(diags.collect(), W - 5.0, 3.0, s, h, &mut out);
    }
    let cur = doc.view.cursor.line as f32 * h / s;
    out.push(el().absolute(1.0, cur.min(h - 2.0)).size(W - 2.0, 2.0).fill(p.fg2.alpha(0.7)));
    if let Some((y, th)) = thumb(doc.view.top, rows, total, h) {
        bar = bar.child(
            el().absolute(3.0, y).size(W - 6.0, th).radius(4.0).fill(p.fg.alpha(0.1)).hover(p.fg.alpha(0.18)).cursor(Cursor::Default),
        );
    }
    for m in out {
        bar = bar.child(m);
    }
    bar
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_thumb_follows_the_view_and_a_press_finds_the_line() {
        assert_eq!(thumb(0, 40, 30, 800.0), None, "a file that fits has no thumb");
        let (y, th) = thumb(0, 40, 1000, 800.0).unwrap();
        assert_eq!(y, 0.0);
        assert!((24.0..800.0).contains(&th));
        let (y, _) = thumb(500, 40, 1000, 800.0).unwrap();
        assert!((y - 800.0 * 500.0 / 1039.0).abs() < 0.01);
        assert_eq!(top_at(y, 40, 1000, 800.0), 500);
        assert_eq!(top_at(-10.0, 40, 1000, 800.0), 0);
        assert_eq!(top_at(5000.0, 40, 1000, 800.0), 999, "the last line can reach the top, no further");
    }

    #[test]
    fn touching_marks_of_one_colour_join() {
        let mut out = Vec::new();
        let red = Color::BLACK;
        let green = Color::WHITE;
        marks(vec![(3, red), (1, red), (2, red), (9, green), (500, red)], 0.0, 3.0, 1000.0, 1000.0, &mut out);
        assert_eq!(out.len(), 3, "lines 1 to 3 make one mark");
    }
}
