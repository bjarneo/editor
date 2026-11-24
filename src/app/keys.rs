//! Key routing. A key goes to the first of: setup, an open overlay (leader menu, picker), the
//! command line, the focused file tree, the Settings tab, then the active keymap.

use editor::editor::{Action, KeyCode, KeyInput, Keymap, Mode};
use rune::{Key, KeyEvent, Tx};

use super::{App, Overlay, Screen};

/// The window's `on_key`: every key, since the editor draws its own text and focus.
pub fn on_key(app: &mut App, ev: &KeyEvent, tx: &mut Tx) -> bool {
    let keys = convert(ev);
    if keys.is_empty() {
        return false;
    }
    // A key's result shows in the next frame, with nothing easing into place.
    tx.instant();
    for k in keys {
        app.key(k, tx);
    }
    true
}

/// Turns Rune's key into the keymaps' keys. Typed text with several chars (an IME commit)
/// becomes one key per char.
pub fn convert(ev: &KeyEvent) -> Vec<KeyInput> {
    let m = ev.mods;
    let one = |code| vec![KeyInput { code, ctrl: m.ctrl, alt: m.alt, shift: m.shift, cmd: m.cmd }];
    match &ev.key {
        Key::Text(t) => {
            let mut out = Vec::new();
            for c in t.chars() {
                // A Control chord some platforms deliver as a control char (Ctrl-R as U+0012).
                let (c, ctrl) =
                    if (c as u32) < 0x20 && c != '\t' && c != '\n' && c != '\r' { (((c as u8) + 0x60) as char, true) } else { (c, m.ctrl) };
                out.push(KeyInput { code: KeyCode::Char(c), ctrl, alt: m.alt, shift: m.shift, cmd: m.cmd || ctrl });
            }
            out
        }
        Key::Command(c) => one(KeyCode::Char(*c)),
        Key::Enter => one(KeyCode::Enter),
        Key::Backspace => one(KeyCode::Backspace),
        Key::Delete => one(KeyCode::Delete),
        Key::Left => one(KeyCode::Left),
        Key::Right => one(KeyCode::Right),
        Key::Up => one(KeyCode::Up),
        Key::Down => one(KeyCode::Down),
        Key::Home => one(KeyCode::Home),
        Key::End => one(KeyCode::End),
        Key::PageUp => one(KeyCode::PageUp),
        Key::PageDown => one(KeyCode::PageDown),
        Key::Escape => one(KeyCode::Esc),
        Key::Tab => one(KeyCode::Tab),
        Key::Function(n) => one(KeyCode::F(*n)),
        _ => Vec::new(),
    }
}

/// The leader menu's entries: key, label, and whether it opens a group.
pub const LEADER: &[(char, &str, bool)] = &[
    ('f', "find file", false),
    ('e', "file tree", false),
    ('/', "grep project", false),
    ('b', "buffers", false),
    ('w', "write", false),
    ('q', "close buffer", false),
    ('d', "diagnostics", false),
    (',', "settings", false),
    ('g', "+git", true),
    ('l', "+lsp", true),
    ('a', "+agent", true),
];

pub const LEADER_AGENT: &[(char, &str, bool)] =
    &[('l', "outside changes", false), ('c', "clear change marks", false), ('p', "next proposal", false), ('n', "clear the note", false)];

pub const LEADER_GIT: &[(char, &str, bool)] =
    &[('n', "next change", false), ('p', "previous change", false), ('s', "status in tree", false)];
pub const LEADER_LSP: &[(char, &str, bool)] =
    &[('d', "definition", false), ('r', "references", false), ('h', "hover", false), ('n', "rename", false), ('a', "code actions", false)];

impl App {
    pub fn key(&mut self, mut k: KeyInput, tx: &mut Tx) {
        if let Screen::Setup(_) = self.screen {
            self.setup_key(k, tx);
            return;
        }
        if self.ed.keymap == Keymap::Vim && k.ctrl && k.code == KeyCode::Char('[') {
            k = KeyInput::plain(KeyCode::Esc);
        }
        match &self.overlay {
            Overlay::Leader(group) => {
                let group = *group;
                self.leader_key(group, k, tx);
                return;
            }
            Overlay::Pick(_) => {
                self.picker_key(k, tx);
                return;
            }
            Overlay::Menu(_) => {
                self.menu_key(k, tx);
                return;
            }
            Overlay::None => {}
        }
        // An agent's proposal takes y, n and scrolling while it shows.
        if self.ed.mode != Mode::Command && self.proposal_key(k) {
            return;
        }
        if self.note.is_some() && k.code == KeyCode::Esc && self.ed.mode != Mode::Insert {
            self.note = None;
        }
        if self.ed.mode == Mode::Command {
            let actions = editor::vim::cmdline_key(&mut self.ed, k);
            self.after_keymap(actions, tx);
            return;
        }
        if self.tree.focus && self.tree.show && self.tree_key(k, tx) {
            return;
        }
        if self.on_settings {
            self.settings_key(k, tx);
            return;
        }
        if !self.ed.has_doc() {
            self.docless_key(k, tx);
            return;
        }
        if self.popup_key(k, tx) {
            return;
        }
        // LSP keys every keymap shares: F2 renames, Ctrl+. (⌘. on macOS) lists code actions.
        if k.code == KeyCode::F(2) {
            self.start_rename();
            return;
        }
        if k.cmd && k.code == KeyCode::Char('.') && self.ed.keymap == Keymap::Classic {
            self.run_actions(vec![Action::CodeActions], tx);
            return;
        }
        let before = self.ed.buf().version();
        let actions = match self.ed.keymap {
            Keymap::Vim => editor::vim::handle_key(&mut self.ed, k),
            Keymap::Classic => editor::classic::handle_key(&mut self.ed, k),
        };
        if self.ed.has_doc() && self.ed.buf().version() != before {
            self.edited();
            self.after_typing(k);
        } else if !matches!(k.code, KeyCode::Char(_)) || k.ctrl {
            self.completion = None;
        }
        self.after_keymap(actions, tx);
    }

    fn after_keymap(&mut self, actions: Vec<Action>, tx: &mut Tx) {
        self.run_actions(actions, tx);
    }

    /// Keys with no file open: the leader, `:`, and the classic shortcuts that open things.
    fn docless_key(&mut self, k: KeyInput, tx: &mut Tx) {
        let actions = match (self.ed.keymap, k.code) {
            (Keymap::Vim, KeyCode::Char(c)) if c == self.ed.vim.leader && !k.ctrl => vec![Action::LeaderMenu],
            (Keymap::Vim, KeyCode::Char(':')) => {
                editor::vim::open_cmdline(&mut self.ed, ':', "");
                Vec::new()
            }
            (Keymap::Classic, KeyCode::Char('p')) if k.cmd && k.shift => vec![Action::Palette],
            (Keymap::Classic, KeyCode::Char('p')) if k.cmd => vec![Action::FindFile],
            (Keymap::Classic, KeyCode::Char('f')) if k.cmd && k.shift => vec![Action::GrepProject],
            (Keymap::Classic, KeyCode::Char('b')) if k.cmd => vec![Action::ToggleTree],
            (Keymap::Classic, KeyCode::Char(',')) if k.cmd => vec![Action::Settings],
            (Keymap::Classic, KeyCode::Char('q')) if k.cmd => vec![Action::QuitAll { force: false }],
            _ => Vec::new(),
        };
        self.run_actions(actions, tx);
    }

    fn leader_key(&mut self, group: Option<char>, k: KeyInput, tx: &mut Tx) {
        self.overlay = Overlay::None;
        let KeyCode::Char(c) = k.code else { return };
        let actions = match (group, c) {
            (None, 'f') => vec![Action::FindFile],
            (None, 'e') => {
                // Show and focus the tree; from the tree, hide it again.
                if self.tree.show && self.tree.focus {
                    self.tree.show = false;
                    self.tree.focus = false;
                } else {
                    self.tree.show = true;
                    self.tree.focus = true;
                    if self.tree.sel.is_none()
                        && self.ed.has_doc()
                        && let Some(p) = self.ed.buf().path.clone()
                    {
                        self.reveal_in_tree(&p);
                    }
                }
                return;
            }
            (None, '/') => vec![Action::GrepProject],
            (None, 'b') => vec![Action::PickBuffer],
            (None, 'w') => vec![Action::Save(None)],
            (None, 'q') => vec![Action::CloseBuffer],
            (None, 'd') => vec![Action::Diagnostics],
            (None, ',') => vec![Action::Settings],
            (None, 'g') | (None, 'l') | (None, 'a') => {
                self.overlay = Overlay::Leader(Some(c));
                return;
            }
            (Some('g'), 'n') => {
                self.jump_sign(true);
                return;
            }
            (Some('g'), 'p') => {
                self.jump_sign(false);
                return;
            }
            (Some('g'), 's') => vec![Action::FocusTree],
            (Some('l'), 'd') => vec![Action::GotoDefinition],
            (Some('l'), 'r') => {
                self.references();
                return;
            }
            (Some('l'), 'n') => {
                self.start_rename();
                return;
            }
            (Some('l'), 'a') => vec![Action::CodeActions],
            (Some('a'), 'l') => {
                self.show_outside();
                return;
            }
            (Some('a'), 'c') => {
                self.clear_outside(None);
                self.ed.message("change marks cleared");
                return;
            }
            (Some('a'), 'p') => {
                match self.proposals.first().map(|p| p.path.clone()) {
                    Some(p) => self.open_path(p, None),
                    None => self.ed.message("no proposals"),
                }
                return;
            }
            (Some('a'), 'n') => {
                self.note = None;
                return;
            }
            (Some('l'), 'h') => vec![Action::Hover],
            _ => Vec::new(),
        };
        self.run_actions(actions, tx);
    }

    /// Moves to the next (or previous) git change in the gutter.
    fn jump_sign(&mut self, forward: bool) {
        if !self.ed.has_doc() {
            return;
        }
        let Some(m) = self.ed.buf().path.as_ref().and_then(|p| self.meta.get(p)) else {
            self.ed.message("no changes against HEAD");
            return;
        };
        let line = self.ed.cursor().line;
        // A change is a run of lines with signs: jump between the first lines of runs.
        let lines: std::collections::BTreeSet<usize> = m.signs.iter().map(|s| s.line).collect();
        let starts: Vec<usize> = lines.iter().copied().filter(|&l| l == 0 || !lines.contains(&(l - 1))).collect();
        let target = if forward {
            starts.iter().find(|&&l| l > line).or(starts.first())
        } else {
            starts.iter().rev().find(|&&l| l < line).or(starts.last())
        };
        match target.copied() {
            Some(l) => self.ed.set_cursor(editor::buffer::Pos::new(l, 0)),
            None => self.ed.message("no changes against HEAD"),
        }
    }
}
