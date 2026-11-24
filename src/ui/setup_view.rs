//! First-launch setup: keys, mouse, look, terminal, ready. Each step is cards and toggles;
//! the status line shows where you are and moves on.

use rune::tok::TextStyle;
use rune::{Color, Cursor, Cx, El, Span, col, el, row, spacer, text};

use super::{font, widgets};
use crate::app::setup::{Choice, FONTS, LOOKS, SIZES, STEPS, Setup};
use crate::app::{App, Screen};
use crate::theme::{self, Palette};

pub fn view(app: &App, p: &Palette, cx: &mut Cx<App>) -> El {
    let Screen::Setup(s) = &app.screen else { return el() };
    let st = Style::new(app);
    let choices = s.choices();
    let focused = choices.get(s.focus.min(choices.len().saturating_sub(1))).copied();
    let body = match s.step {
        1 => step_keys(s, p, &st, focused),
        2 => step_mouse(s, p, &st, focused),
        3 => step_look(app, s, p, &st, focused, cx.system_dark()),
        4 => step_shell(s, p, &st, focused),
        _ => step_ready(s, p, &st, focused),
    };
    let header = row().gap(14.0).items_center().child(icon(p, 34.0)).child(
        col()
            .gap(2.0)
            .child(text("editor").font(st.title).color(p.fg))
            .child(text(format!("setup {}/5 · {}", s.step, STEPS[s.step - 1])).font(st.small).color(p.fg2)),
    );
    let page = col().w_full().max_w(900.0).gap(26.0).child(header).child(body);
    col()
        .w_full()
        .h_full()
        .child(row().grow(1.0).min_h(0.0).w_full().justify_center().pt(40.0).px(32.0).clip_rounded(0.0).child(page))
        .child(status(s, p, &st))
        .child(
            row().h(22.0).px(8.0).items_center().child(
                text(if s.vim() { "j/k move   enter pick   tab next" } else { "arrows move   enter pick   tab next" })
                    .font(st.small)
                    .color(p.fg2),
            ),
        )
}

/// The Modes icon: four lines in the mode colours.
pub fn icon(p: &Palette, size: f32) -> El {
    let k = size / 128.0;
    let bar = |x: f32, y: f32, w: f32, c: Color| el().absolute(x * k, y * k).size(w * k, 12.0 * k).radius(6.0 * k).fill(c);
    el().size(size, size)
        .radius(28.0 * k)
        .fill(p.bg2)
        .border(1.0, p.bd)
        .label("editor")
        .child(bar(28.0, 32.0, 72.0, p.ac))
        .child(bar(28.0, 52.0, 50.0, p.ins))
        .child(bar(40.0, 72.0, 60.0, p.vis))
        .child(bar(28.0, 92.0, 34.0, p.cmd))
}

struct Style {
    body: TextStyle,
    small: TextStyle,
    bold: TextStyle,
    title: TextStyle,
    h1: TextStyle,
    card_title: TextStyle,
}

impl Style {
    fn new(app: &App) -> Style {
        Style {
            body: font::mono(app, 13.0, 400.0),
            small: font::mono(app, 12.0, 400.0),
            bold: font::mono(app, 12.5, 700.0),
            title: font::mono(app, 15.0, 700.0),
            h1: font::mono(app, 24.0, 700.0),
            card_title: font::mono(app, 15.0, 700.0),
        }
    }
}

fn heading(t: &str, sub: &str, p: &Palette, st: &Style) -> El {
    col().gap(6.0).child(text(t).font(st.h1).color(p.fg)).child(text(sub).font(st.body).color(p.fg2))
}

/// A card that picks one option: title, `( )`/`(•)` mark, description and body.
#[allow(clippy::too_many_arguments)]
fn card(choice: Choice, label: &str, title: &str, desc: &str, on: bool, focus: bool, body: El, p: &Palette, st: &Style) -> El {
    let border = if on {
        p.ac
    } else if focus {
        p.fg3
    } else {
        p.bd
    };
    col()
        .grow(1.0)
        .min_w(260.0)
        .gap(12.0)
        .pt(18.0)
        .pb(14.0)
        .px(18.0)
        .radius(6.0)
        .border(if on { 2.0 } else { 1.0 }, border)
        .fill(p.bg)
        .cursor(Cursor::Pointer)
        .role("radio")
        .checked(on)
        .label(title.to_string())
        .on_press_tx(move |s: &mut App, tx| s.setup_choose(choice, tx))
        .child(el().absolute(12.0, -9.0).px(5.0).fill(p.bg).child(text(label).font(st.small).color(if on { p.ac } else { p.fg2 })))
        .child(
            row()
                .items_center()
                .child(text(title).font(st.card_title).color(p.fg))
                .child(spacer())
                .child(text(if on { "(•)" } else { "( )" }).font(st.body).color(if on { p.ac } else { p.fg3 })),
        )
        .child(text(desc).font(st.small).color(p.fg2).wrap())
        .child(body)
}

fn krow(k: &str, v: &str, p: &Palette, st: &Style) -> El {
    row()
        .h(24.0)
        .w_full()
        .items_center()
        .border_sides([1.0, 0.0, 0.0, 0.0], p.bd)
        .child(text(k).font(st.small).color(p.fg2))
        .child(spacer())
        .child(text(v).font(st.bold).color(p.fg))
}

fn pill(t: &str, bg: Color, fg: Color, st: &Style) -> El {
    row().px(7.0).py(2.0).radius(3.0).fill(bg).child(text(t).font(TextStyle { size: 11.0, ..st.bold }).color(fg))
}

fn toggle(choice: Choice, label: &str, sub: &str, on: bool, focus: bool, p: &Palette, st: &Style) -> El {
    let mut c = col().gap(2.0).child(text(label).font(st.body).color(p.fg));
    if !sub.is_empty() {
        c = c.child(text(sub).font(st.small).color(p.fg2));
    }
    row()
        .w_full()
        .py(8.0)
        .px(8.0)
        .gap(12.0)
        .radius(4.0)
        .fill(if focus { p.bg3 } else { Color::TRANSPARENT })
        .hover(p.bg3)
        .cursor(Cursor::Pointer)
        .role("switch")
        .checked(on)
        .label(label.to_string())
        .on_press_tx(move |s: &mut App, tx| s.setup_choose(choice, tx))
        .child(text(widgets::checkbox(on)).font(st.body).color(if on { p.ins } else { p.fg3 }))
        .child(c)
}

fn seg_button(choice: Choice, label: &str, on: bool, focus: bool, p: &Palette, st: &Style, family: Option<&'static str>) -> El {
    let f = match family {
        Some(fam) => TextStyle { family: rune::tok::Family::Named(fam), ..st.small },
        None => st.small,
    };
    row()
        .h(26.0)
        .px(10.0)
        .items_center()
        .radius(4.0)
        .border(
            1.0,
            if on {
                p.ac
            } else if focus {
                p.fg3
            } else {
                p.bd
            },
        )
        .fill(if on { p.ac } else { Color::TRANSPARENT })
        .cursor(Cursor::Pointer)
        .role("button")
        .label(label.to_string())
        .on_press_tx(move |s: &mut App, tx| s.setup_choose(choice, tx))
        .child(text(label).font(if on { TextStyle { weight: 700.0, ..f } } else { f }).color(if on { p.on } else { p.fg2 }))
}

fn step_keys(s: &Setup, p: &Palette, st: &Style, focus: Option<Choice>) -> El {
    let vim = s.vim();
    let m = s.mod_key();
    let leader = if s.toggle_on("leader") { "SPC" } else { "\\" };
    let vim_body = col()
        .gap(12.0)
        .child(
            row()
                .gap(6.0)
                .child(pill("NORMAL", p.ac, p.on, st))
                .child(pill("INSERT", p.ins, p.on, st))
                .child(pill("VISUAL", p.vis, p.on, st)),
        )
        .child(
            col()
                .child(krow("save", ":w", p, st))
                .child(krow("find file", &format!("{leader} f"), p, st))
                .child(krow("search", "/", p, st))
                .child(krow("comment", "gcc", p, st)),
        );
    let classic_body = col()
        .gap(12.0)
        .child(
            row()
                .gap(6.0)
                .child(pill(m, p.bg3, p.fg, st))
                .child(pill(if s.mac { "⌥" } else { "Alt" }, p.bg3, p.fg, st))
                .child(pill("⇧", p.bg3, p.fg, st)),
        )
        .child(
            col()
                .child(krow("save", &format!("{m} S"), p, st))
                .child(krow("find file", &format!("{m} P"), p, st))
                .child(krow("search", &format!("{m} F"), p, st))
                .child(krow("comment", &format!("{m} /"), p, st)),
        );
    let mut opts = widgets::titled("options", p.fg2, st.small, p).w_full().pt(14.0).pb(8.0).px(8.0);
    let toggles: &[(&str, &str, String)] = if vim {
        &[
            ("leader", "Use Space as the leader key", "Otherwise backslash, as in stock Neovim.".to_string()),
            ("import", "Import keymaps from ~/.config/nvim", "Reads your leader. Plugins are skipped.".to_string()),
        ]
    } else {
        &[
            ("multi", "Add cursors with Alt-click", "Every cursor types at once.".to_string()),
            ("autoclose", "Auto-close brackets and quotes", "Typing ( also types ).".to_string()),
        ]
    };
    for (k, label, sub) in toggles {
        opts = opts.child(toggle(Choice::Toggle(k), label, sub, s.toggle_on(k), focus == Some(Choice::Toggle(k)), p, st));
    }
    col()
        .gap(22.0)
        .child(
            row()
                .items_end()
                .flex_wrap()
                .gap(20.0)
                .child(heading("How do you want to edit?", "Switch later with :set keymap.", p, st))
                .child(spacer())
                .child(
                    row()
                        .gap(6.0)
                        .items_center()
                        .child(text("keys shown for").font(st.small).color(p.fg2))
                        .child(seg_button(Choice::Platform(true), "macOS", s.mac, focus == Some(Choice::Platform(true)), p, st, None))
                        .child(seg_button(
                            Choice::Platform(false),
                            "linux / windows",
                            !s.mac,
                            focus == Some(Choice::Platform(false)),
                            p,
                            st,
                            None,
                        )),
                ),
        )
        .child(
            row()
                .gap(16.0)
                .flex_wrap()
                .items_start()
                .child(card(
                    Choice::Keymap(true),
                    "neovim",
                    "Neovim motions",
                    "Modal: move in Normal, type in Insert. Your muscle memory works as is.",
                    vim,
                    focus == Some(Choice::Keymap(true)),
                    vim_body,
                    p,
                    st,
                ))
                .child(card(
                    Choice::Keymap(false),
                    "classic",
                    "Classic shortcuts",
                    "The keys from most editors. No modes; start typing straight away.",
                    !vim,
                    focus == Some(Choice::Keymap(false)),
                    classic_body,
                    p,
                    st,
                )),
        )
        .child(opts)
}

fn step_mouse(s: &Setup, p: &Palette, st: &Style, focus: Option<Choice>) -> El {
    let on = s.cfg.mouse.enabled;
    let vim = s.vim();
    let on_body = col().radius(4.0).border(1.0, p.bd).padding(8.0).child(
        text("let mut pos = from;\nfor _ in 0..count.max(1) {\n    pos = buf.next(pos);")
            .font(TextStyle { size: 11.5, ..st.small })
            .color(p.fg),
    );
    let off_body = col().child(krow("move", if vim { "h j k l" } else { "← ↑ ↓ →" }, p, st)).child(krow(
        "select",
        if vim { "v" } else { "⇧ arrows" },
        p,
        st,
    ));
    let mut c = col()
        .gap(22.0)
        .child(heading("Use the mouse?", "editor is keyboard-first. This only changes the text area and panes.", p, st))
        .child(
            row()
                .gap(16.0)
                .flex_wrap()
                .items_start()
                .child(card(
                    Choice::Mouse(true),
                    "mouse=a",
                    "Mouse on",
                    "Click to place the cursor, drag to select, scroll, resize the file tree.",
                    on,
                    focus == Some(Choice::Mouse(true)),
                    on_body,
                    p,
                    st,
                ))
                .child(card(
                    Choice::Mouse(false),
                    "mouse=",
                    "Keyboard only",
                    "Clicks inside the editor do nothing, so a stray click never moves the cursor. The wheel still scrolls.",
                    !on,
                    focus == Some(Choice::Mouse(false)),
                    off_body,
                    p,
                    st,
                )),
        );
    if on {
        let mut b = widgets::titled("the mouse can", p.fg2, st.small, p).w_full().pt(14.0).pb(8.0).px(8.0);
        let drag = if vim { "Drag selects and enters Visual mode" } else { "Drag selects text" };
        for (k, label) in [
            ("click", "Click places the cursor"),
            ("drag", drag),
            ("double", "Double-click selects a word, triple-click a line"),
            ("right", "Right-click opens a context menu"),
        ] {
            b = b.child(toggle(Choice::Toggle(k), label, "", s.toggle_on(k), focus == Some(Choice::Toggle(k)), p, st));
        }
        c = c.child(b);
    }
    c
}

fn step_look(app: &App, s: &Setup, p: &Palette, st: &Style, focus: Option<Choice>, system_dark: bool) -> El {
    let oma = s.omarchy_name.clone().unwrap_or_else(|| "none found".into());
    let descs = [
        "Paper white, dark ink.",
        "The default. Low glare at night.",
        "Light or dark with your OS.",
        &format!("Follows the desktop theme. Found: {oma}."),
    ];
    let titles = ["Light", "Dark", "System", "Omarchy"];
    let mut cards = row().gap(12.0).flex_wrap().items_start();
    for (i, id) in LOOKS.iter().enumerate() {
        let on = s.cfg.theme.mode == *id;
        let lp = theme::resolve(theme::Source::parse(id), "dark", "light", system_dark, app.omarchy.as_ref()).0;
        let comment = match *id {
            "system" => "follows the OS".to_string(),
            "omarchy" => oma.clone(),
            other => other.to_string(),
        };
        let src = format!("fn class(c: char) {{\n    // {comment}\n    \"word\"\n}}");
        let runs = crate::highlight::runs_of(&src, rune_highlights::Lang::from_name("rust").unwrap_or(rune_highlights::Lang::PLAIN));
        let mut pv = col().radius(4.0).padding(8.0).fill(lp.bg).border(1.0, lp.bd);
        for (li, l) in src.lines().enumerate() {
            let spans: Vec<Span> = runs
                .get(li)
                .map(|rs| rs.iter().map(|(r, k)| Span::new(r.start, r.end).color(crate::highlight::color(&lp, *k))).collect())
                .unwrap_or_default();
            pv = pv.child(text(l.to_string()).font(TextStyle { size: 11.5, ..st.small }).color(lp.fg).spans(spans));
        }
        let c = card(Choice::Look(i), id, titles[i], descs[i], on, focus == Some(Choice::Look(i)), pv, p, st).min_w(190.0);
        cards = cards.child(c);
    }
    let mut fonts = row().gap(6.0).flex_wrap();
    for (i, f) in FONTS.iter().enumerate() {
        fonts = fonts.child(seg_button(
            Choice::Font(i),
            f,
            s.cfg.font.family == *f,
            focus == Some(Choice::Font(i)),
            p,
            st,
            Some(font::family(f)),
        ));
    }
    let mut sizes = row().gap(6.0);
    for (i, z) in SIZES.iter().enumerate() {
        sizes = sizes.child(seg_button(
            Choice::Size(i),
            &format!("{z} pt"),
            s.cfg.font.size == *z,
            focus == Some(Choice::Size(i)),
            p,
            st,
            None,
        ));
    }
    let sample_style = TextStyle::new(s.cfg.font.size, 400.0).family(rune::tok::Family::Named(font::family(&s.cfg.font.family)));
    let sample = col()
        .radius(4.0)
        .padding(8.0)
        .border(1.0, p.bd)
        .child(text("/// The quick brown fox -> jumps != over").font(sample_style).color(p.com))
        .child(text("pub fn word_forward(buf: &Buffer, from: Pos) -> Target {").font(sample_style).color(p.fg));
    let font_box = widgets::titled("font", p.fg2, st.small, p)
        .w_full()
        .pt(16.0)
        .pb(12.0)
        .px(16.0)
        .gap(12.0)
        .child(row().gap(16.0).items_center().child(text("family").font(st.small).color(p.fg2).w(64.0)).child(fonts))
        .child(row().gap(16.0).items_center().child(text("size").font(st.small).color(p.fg2).w(64.0)).child(sizes))
        .child(sample);
    col()
        .gap(22.0)
        .child(heading("Pick a look", "This window follows your pick. Colors, fonts and spacing are all in settings later.", p, st))
        .child(cards)
        .child(font_box)
}

fn step_shell(s: &Setup, p: &Palette, st: &Style, focus: Option<Choice>) -> El {
    let mut c =
        col().gap(22.0).child(heading("Use editor from the terminal", "Optional. Everything here can be undone by hand later.", p, st));
    let cli = &s.cfg.cli;
    if cli.path {
        let valid = s.name_valid();
        let taken = s.taken();
        let (status, color) = if !valid {
            ("✗ use a–z, 0–9, . _ - (no spaces)".to_string(), p.err)
        } else if taken.is_none() {
            ("✓ free".into(), p.ins)
        } else if s.shadow {
            (format!("! will shadow {}", taken.clone().unwrap_or_default().split(',').next().unwrap_or("")), p.warn)
        } else {
            ("✗ taken".into(), p.err)
        };
        let name_focus = focus == Some(Choice::Name);
        let input = row()
            .h(34.0)
            .w(200.0)
            .px(8.0)
            .items_center()
            .radius(4.0)
            .border(1.0, if name_focus { p.ac } else { p.bd })
            .fill(p.bg2)
            .cursor(Cursor::Text)
            .role("textbox")
            .label("command name")
            .on_press_tx(|s: &mut App, tx| s.setup_choose(Choice::Name, tx))
            .child(text(cli.name.clone()).font(TextStyle { size: 14.0, ..st.body }).color(p.fg))
            .child(el().w(1.5).h(16.0).fill(if name_focus { p.ac } else { Color::TRANSPARENT }));
        let checked = if !valid {
            "—".to_string()
        } else {
            match &taken {
                Some(t) => format!("command -v {} → {}", cli.name, t.split(',').next().unwrap_or("")),
                None => format!("command -v {} → nothing found", cli.name),
            }
        };
        let mut b = widgets::titled("command name", p.fg2, st.small, p)
            .w_full()
            .pt(18.0)
            .pb(14.0)
            .px(16.0)
            .gap(12.0)
            .child(
                row()
                    .gap(16.0)
                    .items_center()
                    .child(text("name").font(st.small).color(p.fg2))
                    .child(input)
                    .child(text(status).font(st.small).color(color)),
            )
            .child(text(format!("checked $PATH: {checked}")).font(st.small).color(p.fg2));
        if let Some(t) = taken {
            let mut sug = row().gap(6.0).items_center().flex_wrap().child(text("free").font(st.small).color(p.fg2));
            for (i, n) in s.suggestions().iter().enumerate() {
                sug = sug.child(seg_button(Choice::Suggest(i), n, false, focus == Some(Choice::Suggest(i)), p, st, None));
            }
            sug = sug.child(
                toggle(Choice::Shadow, &format!("keep {}, mine first", cli.name), "", s.shadow, focus == Some(Choice::Shadow), p, st)
                    .w(260.0),
            );
            b = b
                .child(
                    text(format!("{} is already {t}. Pick a free name, or keep this one and put editor's copy first on $PATH.", cli.name))
                        .font(st.small)
                        .color(p.fg2)
                        .wrap(),
                )
                .child(sug);
        }
        c = c.child(b);
    }
    let name = if s.name_valid() { cli.name.clone() } else { "editor".into() };
    let mut integ = widgets::titled("integrations", p.fg2, st.small, p).grow(1.0).min_w(360.0).pt(14.0).pb(8.0).px(8.0);
    for (k, label, sub) in [
        ("path", "Add a terminal command".to_string(), format!("Open anything with {name} . or {name} file.rs")),
        ("env", "Set as $EDITOR and $VISUAL".into(), "For tools that open an editor, like crontab -e".into()),
        ("git", "Use for git commit messages".into(), "git waits until you close the buffer".into()),
        ("default", "Make editor the default for text files".into(), "A double-click opens code and text files".into()),
    ] {
        if k == "default" && s.mac {
            continue;
        }
        integ = integ.child(toggle(Choice::Toggle(k), &label, &sub, s.toggle_on(k), focus == Some(Choice::Toggle(k)), p, st));
    }
    let cmds = s.commands();
    let mut runs = widgets::titled("what runs", p.fg2, st.small, p).grow(1.0).shrink().min_w(360.0).pt(16.0).pb(14.0).px(16.0).gap(4.0);
    if cmds.is_empty() {
        runs = runs.child(text("nothing; editor stays inside its window").font(st.small).color(p.fg2));
    }
    for cmd in cmds {
        // A long command wraps inside the panel instead of running past its border. A `#` line
        // says what a step writes, as a comment in a shell script does.
        let note = cmd.starts_with("# ");
        let line = text(cmd).font(st.small).color(if note { p.fg2 } else { p.fg }).wrap_anywhere().grow(1.0).min_w(0.0);
        runs = runs.child(row().w_full().items_start().child(text(if note { "" } else { "$ " }).font(st.small).color(p.fg3)).child(line));
    }
    c.child(row().gap(16.0).flex_wrap().items_start().child(integ).child(runs))
}

fn step_ready(s: &Setup, p: &Palette, st: &Style, focus: Option<Choice>) -> El {
    let vim = s.vim();
    let m = s.mod_key();
    let cfg = &s.cfg;
    let leader = if cfg.keys.leader == "space" { "Space" } else { "backslash" };
    let shell: Vec<&str> = [("path", "command"), ("env", "$EDITOR"), ("git", "git editor"), ("default", "default app")]
        .iter()
        .filter(|(k, _)| s.toggle_on(k) && (*k != "path" || s.installable()))
        .map(|(_, v)| *v)
        .collect();
    let rows = [
        ("keys", if vim { format!("Neovim motions, {leader} leader") } else { format!("Classic shortcuts ({m})") }),
        ("mouse", if cfg.mouse.enabled { "on".into() } else { "keyboard only".into() }),
        (
            "theme",
            if cfg.theme.mode == "omarchy" {
                format!("Omarchy ({} right now)", s.omarchy_name.clone().unwrap_or_default())
            } else {
                cfg.theme.mode.clone()
            },
        ),
        ("font", format!("{}, {} pt", cfg.font.family, cfg.font.size)),
        ("terminal", if shell.is_empty() { "none".into() } else { shell.join(", ") }),
    ];
    let mut summary = widgets::titled("your setup", p.fg2, st.small, p).grow(1.0).min_w(420.0).pt(12.0).pb(6.0).px(16.0);
    for (i, (k, v)) in rows.iter().enumerate() {
        let f = focus == Some(Choice::Change(i));
        summary = summary.child(
            row()
                .min_h(34.0)
                .w_full()
                .gap(16.0)
                .items_center()
                .border_sides([0.0, 0.0, 1.0, 0.0], p.bd)
                .child(text(*k).font(st.body).color(p.fg2).w(84.0))
                .child(text(v.clone()).font(st.body).color(p.fg).grow(1.0))
                .child(
                    text("change")
                        .font(st.small)
                        .color(p.ac)
                        .cursor(Cursor::Pointer)
                        .fill(if f { p.bg3 } else { Color::TRANSPARENT })
                        .on_press_tx(move |s: &mut App, tx| s.setup_choose(Choice::Change(i), tx)),
                ),
        );
    }
    let lk = if cfg.keys.leader == "space" { "SPC" } else { "\\" };
    let tips: Vec<(&str, Vec<String>)> = if vim {
        vec![
            ("Find a file", vec![lk.into(), "f".into()]),
            ("Every key, in a popup", vec![lk.into()]),
            ("Settings", vec![":Settings".into()]),
        ]
    } else {
        vec![
            ("Find a file", vec![m.into(), "P".into()]),
            ("Command palette", vec![m.into(), "⇧".into(), "P".into()]),
            ("Settings", vec![m.into(), ",".into()]),
        ]
    };
    let mut tips_box = widgets::titled("try first", p.fg2, st.small, p).grow(1.0).min_w(300.0).pt(12.0).pb(6.0).px(16.0);
    for (t, keys) in tips {
        let mut kr = row().gap(4.0);
        for k in keys {
            kr = kr.child(widgets::kbd(&k, TextStyle { size: 11.0, ..st.small }, p));
        }
        tips_box = tips_box.child(
            row()
                .min_h(34.0)
                .w_full()
                .items_center()
                .border_sides([0.0, 0.0, 1.0, 0.0], p.bd)
                .child(text(t).font(st.body).color(p.fg).grow(1.0))
                .child(kr),
        );
    }
    col()
        .gap(22.0)
        .child(heading("Ready", "Saved to ~/.config/editor/config.toml when you open the editor.", p, st))
        .child(row().gap(16.0).flex_wrap().items_start().child(summary).child(tips_box))
}

fn status(s: &Setup, p: &Palette, st: &Style) -> El {
    let small = st.small;
    let bold = TextStyle { weight: 700.0, ..small };
    let mut line = row()
        .h(24.0)
        .w_full()
        .items_center()
        .fill(p.bg2)
        .child(row().h_full().items_center().px(12.0).fill(p.ac).child(text("SETUP").font(bold).color(p.on)));
    for (i, t) in STEPS.iter().enumerate() {
        let n = i + 1;
        let (c, f) = if n == s.step {
            (p.fg, bold)
        } else if n < s.step {
            (p.ins, small)
        } else {
            (p.fg2, small)
        };
        line = line.child(text(format!("{}{t}", if i > 0 { "› " } else { "" })).font(f).color(c).px(6.0));
    }
    line = line.child(spacer());
    let btn = |label: &'static str, go: bool, by: isize| {
        row()
            .h_full()
            .px(12.0)
            .items_center()
            .fill(if go { p.ac } else { Color::TRANSPARENT })
            .hover(p.fg.alpha(0.08))
            .cursor(Cursor::Pointer)
            .role("button")
            .label(label)
            .on_press_tx(move |s: &mut App, tx| {
                if by == 99 {
                    s.finish_setup(tx);
                } else {
                    s.setup_go(by);
                }
            })
            .child(text(label).font(if go { bold } else { small }).color(if go { p.on } else { p.fg2 }))
    };
    if s.step < 5 {
        line = line.child(btn("skip", false, 10));
    }
    if s.step > 1 {
        line = line.child(btn("back", false, -1));
    }
    if s.step < 5 {
        line = line.child(btn("next →", true, 1));
    } else {
        line = line.child(btn("open editor →", true, 99));
    }
    line
}
