//! Small pieces the screens share: a bordered box with its title in the top border, key caps,
//! toggles drawn as `[x]`, and segmented buttons.

use rune::tok::TextStyle;
use rune::{Color, El, col, el, row, text};

use crate::theme::Palette;

/// A box with a 1 pt border and a title sitting in its top edge, as the design draws panels.
pub fn titled(title: &str, title_color: Color, style: TextStyle, p: &Palette) -> El {
    col()
        .border(1.0, p.bd)
        .radius(6.0)
        .fill(p.bg)
        .child(el().absolute(12.0, -9.0).px(5.0).fill(p.bg).child(text(title).font(style).color(title_color)))
}

/// A key cap: `SPC`, `⌘`, `gd`.
pub fn kbd(k: &str, style: TextStyle, p: &Palette) -> El {
    row()
        .h(20.0)
        .min_w(20.0)
        .px(5.0)
        .items_center()
        .justify_center()
        .radius(3.0)
        .fill(p.bg3)
        .border(1.0, p.bd)
        .child(text(k).font(style).color(p.fg))
}

/// `[x] on ` or `[ ] off` style toggle text.
pub fn checkbox(on: bool) -> &'static str {
    if on { "[x]" } else { "[ ]" }
}
