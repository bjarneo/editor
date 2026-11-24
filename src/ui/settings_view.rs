//! The Settings buffer: sections, options, a live preview and the config.toml it writes.

use rune::tok::{Family, TextStyle};
use rune::{Color, Cursor, Cx, El, Span, col, el, row, spacer, text};

use super::{font, widgets};
use crate::app::App;
use crate::app::settings::{self, Ctl, Item, SECTIONS};
use crate::theme::{self, Palette};

const SNIPPET: &[(usize, &str)] = &[
    (0, "/// w: start of the next word."),
    (0, "pub fn word_forward(buf: &Buffer, from: Pos) -> Target {"),
    (1, "let mut pos = from;"),
    (1, "let kind = match buf.char_at(pos) {"),
    (2, "c if c != ' ' => Class::Word,"),
    (2, "_ => Class::Blank,"),
    (1, "};"),
    (1, "while class(buf.char_at(pos)) == kind && !buf.at_end(pos) {"),
    (2, "pos = buf.next(pos);"),
    (1, "}"),
    (1, "Target { pos, inclusive: false }"),
    (0, "}"),
];

pub fn view(app: &App, p: &Palette, cx: &mut Cx<App>) -> El {
    let Some(st) = &app.settings else { return el() };
    let ui = font::ui(app);
    let small = TextStyle { size: 11.5, ..ui };
    let list = settings::items(st.section, &app.cfg, app.omarchy.as_ref(), cx.system_dark());
    // Sections.
    let mut nav = col().w(230.0).h_full().fill(p.bg2).border_sides([0.0, 1.0, 0.0, 0.0], p.bd).pt(6.0);
    for &(sec, label) in SECTIONS {
        let on = sec == st.section;
        nav = nav.child(
            row()
                .h(26.0)
                .w_full()
                .items_center()
                .fill(if on { p.sel } else { Color::TRANSPARENT })
                .hover(p.bg3)
                .cursor(Cursor::Pointer)
                .role("tab")
                .label(label)
                .on_press(move |s: &mut App| {
                    if let Some(st) = s.settings.as_mut() {
                        st.section = sec;
                        st.row = 0;
                    }
                })
                .child(el().w(3.0).h(16.0).mr(12.0).fill(if on { p.ac } else { Color::TRANSPARENT }))
                .child(text(label).font(ui).color(p.fg)),
        );
    }
    nav = nav.child(spacer()).child(
        col()
            .w_full()
            .gap(6.0)
            .pt(12.0)
            .pb(14.0)
            .px(18.0)
            .border_sides([1.0, 0.0, 0.0, 0.0], p.bd)
            .child(text(crate::app::tilde(&crate::config::config_path())).font(small).color(p.fg2))
            .child(
                text("open as a buffer")
                    .font(small)
                    .color(p.ac)
                    .cursor(Cursor::Pointer)
                    .on_press(|s: &mut App| s.open_path(crate::config::config_path(), None)),
            ),
    );
    // Options.
    let mut opts = col().grow(1.0).min_w(0.0).h_full().px(24.0).pt(18.0).pb(24.0).clip_rounded(0.0);
    for (i, item) in list.iter().enumerate() {
        match item {
            Item::Head(t) => {
                opts = opts.child(text(*t).font(TextStyle { size: 12.0, ..ui }).color(p.fg2).pt(if i == 0 { 0.0 } else { 18.0 }).pb(6.0))
            }
            Item::Row { key, desc, ctl } => opts = opts.child(option_row(app, p, ui, key, desc, ctl, i == st.row)),
        }
    }
    let width = cx.layout_size("settings-root").map_or(1200.0, |s| s[0]);
    let mut out = row().key("settings-root").grow(1.0).min_w(0.0).h_full().fill(p.bg).child(nav).child(opts);
    if width > 1100.0 {
        out = out.child(preview(app, p));
    }
    out
}

fn option_row(_app: &App, p: &Palette, ui: TextStyle, key: &'static str, desc: &str, ctl: &Ctl, kb: bool) -> El {
    let label = col()
        .grow(1.0)
        .min_w(200.0)
        .gap(2.0)
        .child(text(key).font(ui).color(p.fg))
        .child(text(desc.to_string()).font(TextStyle::new(11.5, 400.0).family(Family::Sans)).color(p.fg2));
    let small = TextStyle { size: 12.0, ..ui };
    let ctl_el: El = match ctl {
        Ctl::Seg { opts, cur } => {
            let mut g = row().border(1.0, p.bd).radius(4.0).clip_rounded(4.0);
            for (j, (v, l)) in opts.iter().enumerate() {
                let on = v == cur;
                let v = v.clone();
                g = g.child(
                    row()
                        .h(26.0)
                        .px(10.0)
                        .items_center()
                        .fill(if on { p.ac } else { Color::TRANSPARENT })
                        .border_sides([0.0, if j + 1 < opts.len() { 1.0 } else { 0.0 }, 0.0, 0.0], p.bd)
                        .cursor(Cursor::Pointer)
                        .hover(p.fg.alpha(0.06))
                        .role("button")
                        .label(l.clone())
                        .on_press(move |s: &mut App| {
                            settings::pick(&mut s.cfg, key, &v);
                            s.settings_changed(key);
                        })
                        .child(text(l.clone()).font(if on { TextStyle { weight: 700.0, ..small } } else { small }).color(if on {
                            p.on
                        } else {
                            p.fg2
                        })),
                );
            }
            g
        }
        Ctl::Font { opts, cur } => {
            let mut g = row().gap(6.0).flex_wrap();
            for f in opts {
                let on = f == cur;
                let fam = font::family(f);
                let v = f.clone();
                g = g.child(
                    row()
                        .h(30.0)
                        .px(12.0)
                        .items_center()
                        .radius(4.0)
                        .border(1.0, if on { p.ac } else { p.bd })
                        .fill(if on { p.sel } else { Color::TRANSPARENT })
                        .cursor(Cursor::Pointer)
                        .label(f.clone())
                        .on_press(move |s: &mut App| {
                            settings::pick(&mut s.cfg, key, &v);
                            s.settings_changed(key);
                        })
                        .child(text(f.clone()).font(TextStyle::new(13.0, 400.0).family(Family::Named(fam))).color(if on {
                            p.fg
                        } else {
                            p.fg2
                        })),
                );
            }
            g
        }
        Ctl::Step { shown } => row()
            .h(26.0)
            .items_center()
            .border(1.0, p.bd)
            .radius(4.0)
            .child(step_btn("−", key, -1.0, p, ui))
            .child(row().min_w(64.0).justify_center().child(text(shown.clone()).font(ui).color(p.fg)))
            .child(step_btn("+", key, 1.0, p, ui)),
        Ctl::Toggle(on) => row()
            .h(26.0)
            .px(8.0)
            .items_center()
            .radius(4.0)
            .border(1.0, if *on { p.ins } else { p.bd })
            .cursor(Cursor::Pointer)
            .role("switch")
            .checked(*on)
            .label(key)
            .on_press(move |s: &mut App| {
                settings::toggle(&mut s.cfg, key);
                s.settings_changed(key);
            })
            .child(text(if *on { "[x] on " } else { "[ ] off" }).font(ui).color(if *on { p.ins } else { p.fg2 })),
        Ctl::Info(v) => text(v.clone()).font(ui).color(p.fg),
        Ctl::Chips(colors) => {
            let mut g = row().gap(4.0);
            for c in colors {
                g = g.child(el().size(22.0, 22.0).radius(3.0).fill(*c).border(1.0, p.bd));
            }
            g
        }
        Ctl::Themes { opts, cur } => {
            let mut g = row().flex_wrap().gap(8.0).w_full();
            for id in opts {
                let on = id == cur;
                let v = id.clone();
                let sw = theme::swatches(id);
                let mut chips = row().radius(3.0).clip_rounded(3.0).border(1.0, p.bd);
                for (k, c) in sw.iter().enumerate() {
                    chips = chips.child(el().w(if k == 0 { 16.0 } else { 7.0 }).h(18.0).fill(*c));
                }
                g = g.child(
                    row()
                        .w(196.0)
                        .h(34.0)
                        .px(8.0)
                        .gap(10.0)
                        .items_center()
                        .radius(4.0)
                        .border(if on { 2.0 } else { 1.0 }, if on { p.ac } else { p.bd })
                        .cursor(Cursor::Pointer)
                        .label(theme::name(id).to_string())
                        .on_press(move |s: &mut App| {
                            settings::pick(&mut s.cfg, key, &v);
                            s.settings_changed(key);
                        })
                        .child(chips)
                        .child(text(theme::name(id).to_string()).font(ui).color(if on { p.fg } else { p.fg2 })),
                );
            }
            return col()
                .w_full()
                .gap(10.0)
                .py(10.0)
                .border_sides([0.0, 0.0, 1.0, 0.0], p.bd)
                .fill(if kb { p.bg3 } else { Color::TRANSPARENT })
                .child(label)
                .child(g);
        }
    };
    row()
        .w_full()
        .min_h(40.0)
        .py(4.0)
        .gap(24.0)
        .items_center()
        .flex_wrap()
        .border_sides([0.0, 0.0, 1.0, 0.0], p.bd)
        .fill(if kb { p.bg3 } else { Color::TRANSPARENT })
        .child(label)
        .child(ctl_el)
}

fn step_btn(t: &'static str, key: &'static str, dir: f32, p: &Palette, ui: TextStyle) -> El {
    row()
        .w(28.0)
        .h(24.0)
        .justify_center()
        .items_center()
        .cursor(Cursor::Pointer)
        .hover(p.bg3)
        .role("button")
        .label(if dir < 0.0 { "Decrease" } else { "Increase" })
        .on_press(move |s: &mut App| {
            settings::step_by(&mut s.cfg, key, dir);
            s.settings_changed(key);
        })
        .child(text(t).font(TextStyle { size: 14.0, ..ui }).color(p.fg2))
}

fn preview(app: &App, p: &Palette) -> El {
    let cfg = &app.cfg;
    let style = font::code(app);
    let rh = font::row_height(app);
    let tab = cfg.indent.width.max(1) as usize;
    let cur_line = 3usize;
    let lang = rune_highlights::Lang::from_name("rust").unwrap_or(rune_highlights::Lang::PLAIN);
    let src: Vec<String> = SNIPPET.iter().map(|(lvl, t)| format!("{}{t}", " ".repeat(lvl * tab))).collect();
    let runs = crate::highlight::runs_of(&src.join("\n"), lang);
    let mut code = col().w_full().pt(14.0).pb(12.0).clip_rounded(0.0);
    for (i, line) in src.iter().enumerate() {
        let n = i + 1;
        let is_cur = n == cur_line;
        let num = match cfg.editor.numbers.as_str() {
            "off" => String::new(),
            "absolute" => n.to_string(),
            "relative" => n.abs_diff(cur_line).to_string(),
            _ if is_cur => n.to_string(),
            _ => n.abs_diff(cur_line).to_string(),
        };
        let lead = line.chars().take_while(|c| *c == ' ').count();
        let shown = if cfg.editor.whitespace { format!("{}{}", "·".repeat(lead), &line[lead..]) } else { line.clone() };
        let shift = if cfg.editor.whitespace { "·".len() - 1 } else { 0 } * lead;
        let mut spans: Vec<Span> = runs
            .get(i)
            .map(|rs| rs.iter().map(|(r, k)| Span::new(r.start + shift, r.end + shift).color(crate::highlight::color(p, *k))).collect())
            .unwrap_or_default();
        if cfg.editor.whitespace && lead > 0 {
            spans.insert(0, Span::new(0, lead * "·".len()).color(p.bd));
        }
        let mut code_el = row().grow(1.0).min_w(0.0).child(text(shown).font(style).color(p.fg).spans(spans));
        if is_cur {
            let x = (lead + 4) as f32 * app.m.cw;
            let c = line.chars().nth(lead + 4).unwrap_or(' ');
            let cursor = match cfg.editor.cursor.as_str() {
                "bar" => el().absolute(x, 2.0).size(2.0, rh - 4.0).fill(p.ac),
                "underline" => el().absolute(x, rh - 2.0).size(app.m.cw, 2.0).fill(p.fg),
                _ => el().absolute(x, 0.0).size(app.m.cw, rh).fill(p.fg).child(text(c.to_string()).font(style).color(p.bg)),
            };
            code_el = code_el.child(cursor);
        }
        code = code.child(
            row()
                .h(rh)
                .fill(if is_cur && cfg.editor.cursorline { p.bg3 } else { Color::TRANSPARENT })
                .child(row().w(3.0 * app.m.cw + 2.0 * app.m.cw).justify_end().pr(2.0 * app.m.cw).child(
                    text(num).font(if is_cur { TextStyle { weight: 700.0, ..style } } else { style }).color(if is_cur {
                        p.fg
                    } else {
                        p.fg3
                    }),
                ))
                .child(code_el),
        );
    }
    let small = font::mono(app, 12.0, 400.0);
    let mut toml_box = col().w_full().pt(16.0).pb(12.0).px(16.0).gap(2.0);
    let doc: toml::Table = toml::from_str(&cfg.to_toml()).unwrap_or_default();
    for (section, keys) in [
        ("theme", &["mode", "dark", "light"][..]),
        ("font", &["family", "size", "line_height"][..]),
        ("editor", &["numbers"][..]),
        ("indent", &["width"][..]),
        ("format", &["trim_whitespace"][..]),
        ("keys", &["map"][..]),
    ] {
        for k in keys {
            let v = doc.get(section).and_then(|t| t.get(*k)).map(|v| v.to_string()).unwrap_or_default();
            let vc = if v.starts_with('"') {
                p.str_
            } else if v == "true" || v == "false" {
                p.kw
            } else {
                p.num
            };
            toml_box = toml_box.child(
                row()
                    .child(text(format!("{section}.{k}")).font(small).color(p.prop))
                    .child(text(" = ").font(small).color(p.pun))
                    .child(text(v).font(small).color(vc)),
            );
        }
    }
    col()
        .w(480.0)
        .h_full()
        .pt(18.0)
        .pr(20.0)
        .gap(18.0)
        .child(widgets::titled("preview · motion.rs", p.ac, small, p).w_full().child(code))
        .child(widgets::titled("config.toml", p.ac, small, p).w_full().child(toml_box))
}
