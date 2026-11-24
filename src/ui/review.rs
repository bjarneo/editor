//! An agent's proposed change, drawn as a diff in the text area: removed lines tinted red,
//! added lines tinted green, unchanged runs folded, and a banner with the keys.

use rune::{Color, El, Span, col, el, row, spacer, text};

use super::font;
use crate::app::App;
use crate::app::review::{Proposal, Row};
use crate::highlight;
use crate::theme::Palette;

pub fn diff_view(app: &App, p: &Palette, prop: &Proposal, mut area: El) -> El {
    let style = font::code(app);
    let small = font::mono(app, 12.0, 400.0);
    let rh = font::row_height(app);
    let cw = app.m.cw;
    let lang = prop.path.extension().map(|_| highlight::lang_for(Some(&prop.path))).unwrap_or(rune_highlights::Lang::PLAIN);
    let total = prop.rows.len();
    let rows = app.ed.rows.saturating_sub(1).max(1);
    let banner = row()
        .h(rh)
        .w_full()
        .px(12.0)
        .gap(14.0)
        .items_center()
        .fill(p.bg2)
        .border_sides([0.0, 0.0, 1.0, 0.0], p.bd)
        .child(text("PROPOSAL").font(rune::tok::TextStyle { weight: 700.0, ..small }).color(p.vis))
        .child(text(prop.label.clone()).font(small).color(p.fg).truncate())
        .child(text(format!("+{}", prop.added)).font(small).color(p.ins))
        .child(text(format!("-{}", prop.removed)).font(small).color(p.err))
        .child(spacer())
        .child(
            text(format!(
                "{}/{} changes",
                prop.starts.iter().filter(|&&s| s <= prop.top + 3).count().max(1).min(prop.starts.len()),
                prop.starts.len()
            ))
            .font(small)
            .color(p.fg2),
        )
        .child(text("y accept · n reject · ]c next · [c previous").font(small).color(p.fg3));
    area = area.child(banner);
    if prop.base == u64::MAX {
        return area.child(row().h(rh).px(12.0).items_center().child(text("opening the file…").font(small).color(p.fg2)));
    }
    let gutter = 7.0 * cw;
    for r in prop.rows.iter().skip(prop.top).take(rows) {
        let (sign, sign_color, fill, num, body, colored): (&str, Color, Color, String, String, bool) = match r {
            Row::Same { new, text } => (" ", p.fg3, Color::TRANSPARENT, (new + 1).to_string(), text.clone(), true),
            Row::Added { new, text } => ("+", p.ins, p.ins.alpha(0.13), (new + 1).to_string(), text.clone(), true),
            Row::Removed { text } => ("-", p.err, p.err.alpha(0.13), String::new(), text.clone(), true),
            Row::Fold(n) => (" ", p.fg3, p.bg2, String::new(), format!("⋯ {n} unchanged lines"), false),
        };
        let spans: Vec<Span> = if colored {
            highlight::runs_of(&body, lang)
                .first()
                .map(|rs| {
                    rs.iter()
                        .filter_map(|(range, k)| {
                            let a = body.char_indices().nth(range.start).map_or(body.len(), |(b, _)| b);
                            let b = body.char_indices().nth(range.end).map_or(body.len(), |(b, _)| b);
                            (a < b).then(|| Span::new(a, b).color(highlight::color(p, *k)))
                        })
                        .collect()
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let ink = if matches!(r, Row::Fold(_)) {
            p.fg3
        } else if matches!(r, Row::Same { .. }) {
            p.fg2
        } else {
            p.fg
        };
        area = area.child(
            row()
                .h(rh)
                .w_full()
                .fill(fill)
                .child(row().w(2.0 * cw).justify_center().child(text(sign).font(style).color(sign_color)))
                .child(row().w(gutter - 2.0 * cw).justify_end().pr(2.0 * cw).child(text(num).font(style).color(p.fg3)))
                .child(text(body).font(style).color(ink).spans(spans)),
        );
    }
    if total == 0 {
        area =
            area.child(col().px(12.0).py(8.0).child(text("the proposal matches the buffer: nothing to change").font(small).color(p.fg2)));
    }
    area.child(el().h(0.0))
}
