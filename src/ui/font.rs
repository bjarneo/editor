//! Text styles for code and for the interface, from config.toml's font settings.

use std::collections::HashMap;
use std::sync::Mutex;

use rune::tok::{Family, TextStyle};

use crate::app::App;

/// The CSS family list for a configured font, with monospace fallbacks after it. Rune takes
/// `&'static str` families, so each distinct name is kept for the life of the process: a
/// handful at most, one per font the user tries.
pub fn family(name: &str) -> &'static str {
    static NAMES: Mutex<Option<HashMap<String, &'static str>>> = Mutex::new(None);
    let mut names = NAMES.lock().unwrap_or_else(|e| e.into_inner());
    let map = names.get_or_insert_with(HashMap::new);
    if let Some(f) = map.get(name) {
        return f;
    }
    let clean: String = name.chars().filter(|c| !matches!(c, ',' | '"' | '\'')).collect();
    let list: &'static str =
        Box::leak(format!("\"{clean}\", \"JetBrainsMono Nerd Font\", \"JetBrains Mono\", ui-monospace").into_boxed_str());
    map.insert(name.to_string(), list);
    list
}

/// Rows of code are this tall: the font size times the line height, on whole points.
pub fn row_height(app: &App) -> f32 {
    (app.cfg.font.size * app.cfg.font.line_height).round().max(app.cfg.font.size + 2.0)
}

/// The editor font at its configured size, weight, spacing and ligatures.
pub fn code(app: &App) -> TextStyle {
    let f = &app.cfg.font;
    let mut s = TextStyle::new(f.size, f.weight as f32).family(Family::Named(app.font_family));
    s.line_height = row_height(app);
    s.tracking = f.letter_spacing;
    if !f.ligatures {
        s.features = "'liga' 0, 'calt' 0";
    }
    s
}

/// The interface font (tree, tabs, status line) at `size`.
pub fn ui_at(app: &App, size: f32) -> TextStyle {
    let fam = if app.cfg.ui.font == "system" { Family::Sans } else { Family::Named(app.font_family) };
    TextStyle::new(size, 400.0).family(fam)
}

/// The interface font at its configured size.
pub fn ui(app: &App) -> TextStyle {
    ui_at(app, app.cfg.ui.size)
}

/// The editor family at `size`, for the design's monospace headings and labels.
pub fn mono(app: &App, size: f32, weight: f32) -> TextStyle {
    TextStyle::new(size, weight).family(Family::Named(app.font_family))
}
