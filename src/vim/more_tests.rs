//! Tests for Replace mode, sentences, tags, operator-pending search, `gn`, `U`, Visual dot
//! repeat, `:s///c`, marks, the jump list across files, `gf` and the tree-sitter keys.

use std::path::PathBuf;

use super::tests::{at, cur, ed, feed, text};
use super::*;
use crate::buffer::{Buffer, Pos};
use crate::editor::{Action, Doc, Mode};

#[test]
fn replace_mode_overtypes_and_backspace_restores() {
    let mut e = ed("xyz");
    feed(&mut e, "Rabc");
    assert!(replacing(&e.vim));
    assert_eq!(e.mode, Mode::Insert);
    assert_eq!(text(&e), "abc");
    feed(&mut e, "<bs><bs>");
    assert_eq!(text(&e), "ayz", "Backspace brings back what each char covered");
    feed(&mut e, "<esc>");
    assert!(!replacing(&e.vim));
    assert_eq!(cur(&e), (0, 0));
    feed(&mut e, "u");
    assert_eq!(text(&e), "xyz", "one undo step");
}

#[test]
fn replace_mode_appends_past_the_end_and_splits_lines() {
    let mut e = ed("ab");
    feed(&mut e, "Rxyz<bs><esc>");
    assert_eq!(text(&e), "xy");
    let mut e = ed("abcd");
    feed(&mut e, "lR<cr><esc>");
    assert_eq!(text(&e), "a\nbcd", "Enter splits the line without eating a char");
}

#[test]
fn replace_mode_counts_and_dot() {
    let mut e = ed("abcdefgh");
    feed(&mut e, "3Rxy<esc>");
    assert_eq!(text(&e), "xyxyxygh");
    let mut e = ed("aaaa\naaaa");
    feed(&mut e, "Rxy<esc>j0.");
    assert_eq!(text(&e), "xyaa\nxyaa");
}

#[test]
fn sentence_motions_and_objects() {
    let s = "Hello there. How are you? Fine.";
    let mut e = ed(s);
    feed(&mut e, ")");
    assert_eq!(cur(&e), (0, 13));
    feed(&mut e, ")");
    assert_eq!(cur(&e), (0, 26));
    feed(&mut e, "(");
    assert_eq!(cur(&e), (0, 13));
    feed(&mut e, "dis");
    assert_eq!(text(&e), "Hello there.  Fine.");
    let mut e = ed(s);
    at(&mut e, 0, 15);
    feed(&mut e, "das");
    assert_eq!(text(&e), "Hello there. Fine.");
    let mut e = ed(s);
    at(&mut e, 0, 28);
    feed(&mut e, "das");
    assert_eq!(text(&e), "Hello there. How are you?", "the last sentence takes the white space before it");
}

#[test]
fn sentences_stop_at_empty_lines() {
    let mut e = ed("One. Two\n\nThree.");
    feed(&mut e, "))");
    assert_eq!(cur(&e), (1, 0));
    feed(&mut e, ")");
    assert_eq!(cur(&e), (2, 0));
}

#[test]
fn tag_objects_nest_and_count() {
    let s = "<div><p>hi <b>x</b></p></div>";
    let mut e = ed(s);
    at(&mut e, 0, 8);
    feed(&mut e, "dit");
    assert_eq!(text(&e), "<div><p></p></div>");
    let mut e = ed(s);
    at(&mut e, 0, 14);
    feed(&mut e, "dat");
    assert_eq!(text(&e), "<div><p>hi </p></div>");
    let mut e = ed(s);
    at(&mut e, 0, 14);
    feed(&mut e, "3dit");
    assert_eq!(text(&e), "<div></div>");
    let mut e = ed("<ul>\n  <li>a</li>\n  <br/>\n</ul>");
    at(&mut e, 1, 6);
    feed(&mut e, "2dit");
    assert_eq!(text(&e), "<ul></ul>", "self-closing tags don't count");
}

#[test]
fn operators_take_a_search_as_their_motion() {
    let mut e = ed("one two three four");
    feed(&mut e, "d/thr<cr>");
    assert_eq!(text(&e), "three four");
    assert_eq!(e.mode, Mode::Normal);
    let mut e = ed("one two three");
    at(&mut e, 0, 8);
    feed(&mut e, "c?one<cr>X<esc>");
    assert_eq!(text(&e), "Xthree");
    let mut e = ed("one two");
    feed(&mut e, "d/<esc>w");
    assert_eq!(text(&e), "one two", "Esc drops the operator");
    assert_eq!(cur(&e), (0, 4));
    assert!(e.vim.op_search.is_none());
}

#[test]
fn y_slash_yanks_up_to_the_match() {
    let mut e = ed("abc def ghi");
    feed(&mut e, "y/gh<cr>");
    assert_eq!(e.regs.unnamed.text, "abc def ");
    assert_eq!(text(&e), "abc def ghi");
}

#[test]
fn cgn_and_dot_change_match_after_match() {
    let mut e = ed("foo bar foo bar foo");
    feed(&mut e, "/foo<cr>");
    assert_eq!(cur(&e), (0, 8));
    feed(&mut e, "cgnX<esc>");
    assert_eq!(text(&e), "foo bar X bar foo");
    feed(&mut e, ".");
    assert_eq!(text(&e), "foo bar X bar X");
    feed(&mut e, ".");
    assert_eq!(text(&e), "X bar X bar X");
}

#[test]
fn gn_selects_the_match_and_dgn_deletes_it() {
    let mut e = ed("a foo b");
    feed(&mut e, "/foo<cr>0gn");
    assert_eq!(e.mode, Mode::Visual(crate::editor::VisualKind::Char));
    assert_eq!(e.doc().view.anchor, Some(Pos::new(0, 2)));
    assert_eq!(cur(&e), (0, 4));
    feed(&mut e, "<esc>0dgn");
    assert_eq!(text(&e), "a  b");
}

#[test]
fn big_u_restores_the_last_changed_line() {
    let mut e = ed("hello world\nnext");
    feed(&mut e, "xx");
    assert_eq!(text(&e), "llo world\nnext");
    feed(&mut e, "U");
    assert_eq!(text(&e), "hello world\nnext");
    feed(&mut e, "u");
    assert_eq!(text(&e), "llo world\nnext", "U is a change that u undoes");
}

#[test]
fn dot_repeats_visual_commands_on_the_same_size() {
    let mut e = ed("a\nb\nc\nd\ne");
    feed(&mut e, "Vjd");
    assert_eq!(text(&e), "c\nd\ne");
    feed(&mut e, ".");
    assert_eq!(text(&e), "e");
    let mut e = ed("abcdef abcdef");
    feed(&mut e, "vlld");
    assert_eq!(text(&e), "def abcdef");
    feed(&mut e, "w.");
    assert_eq!(text(&e), "def def");
    let mut e = ed("foo foo");
    feed(&mut e, "vecX<esc>");
    assert_eq!(text(&e), "X foo");
    feed(&mut e, "w.");
    assert_eq!(text(&e), "X X", "a Visual change repeats with its text");
    let mut e = ed("abcd");
    feed(&mut e, "vl~ll.");
    assert_eq!(text(&e), "ABCD");
    let mut e = ed("ab\ncd\nef");
    feed(&mut e, "<c-v>jrxj.");
    assert_eq!(text(&e), "xb\nxd\nxf");
}

#[test]
fn substitute_confirm_asks_for_each_match() {
    let mut e = ed("a a\na");
    feed(&mut e, ":%s/a/b/gc<cr>");
    assert!(confirming(&e.vim));
    assert!(e.msg.as_ref().is_some_and(|m| m.text.starts_with("replace with b (y/n/a/q/l")));
    assert_eq!(e.doc().view.anchor, Some(Pos::new(0, 0)), "the match is selected");
    feed(&mut e, "yny");
    assert!(!confirming(&e.vim));
    assert_eq!(text(&e), "b a\nb");
    assert_eq!(e.msg.as_ref().map(|m| m.text.as_str()), Some("2 substitutions on 2 lines"));
    assert_eq!(e.mode, Mode::Normal);
    feed(&mut e, "u");
    assert_eq!(text(&e), "a a\na", "the whole session is one undo step");
}

#[test]
fn substitute_confirm_all_quit_and_last() {
    let mut e = ed("a a\na");
    feed(&mut e, ":%s/a/b/gc<cr>a");
    assert_eq!(text(&e), "b b\nb");
    let mut e = ed("a a\na");
    feed(&mut e, ":%s/a/b/gc<cr>yq");
    assert_eq!(text(&e), "b a\na");
    let mut e = ed("a a\na");
    feed(&mut e, ":%s/a/b/gc<cr>nl");
    assert_eq!(text(&e), "a b\na");
    let mut e = ed("a a\na");
    feed(&mut e, ":%s/a/b/c<cr>yy");
    assert_eq!(text(&e), "b a\nb", "without g, one match a line");
}

#[test]
fn marks_follow_edits_above_them() {
    let mut e = ed("a\nb\nc");
    feed(&mut e, "jjma");
    feed(&mut e, "ggOnew<esc>");
    feed(&mut e, "gg'a");
    assert_eq!(cur(&e), (3, 0));
    feed(&mut e, ":marks<cr>");
    assert!(e.msg.as_ref().is_some_and(|m| m.text.contains("a 4:0")));
    feed(&mut e, ":delmarks a<cr>'a");
    assert!(e.msg.as_ref().is_some_and(|m| m.error && m.text.starts_with("E20")));
}

fn two_files() -> crate::editor::Editor {
    let mut e = ed("one\ntwo\nthree");
    e.docs[0].buf.path = Some(PathBuf::from("/tmp/project/a.rs"));
    let mut b = Buffer::from_str("x\ny");
    b.path = Some(PathBuf::from("/tmp/project/b.rs"));
    e.docs.push(Doc::new(b));
    e
}

#[test]
fn uppercase_marks_open_their_file() {
    let mut e = two_files();
    feed(&mut e, "jmA");
    e.active = 1;
    assert_eq!(feed(&mut e, "'A"), vec![Action::OpenAt(PathBuf::from("/tmp/project/a.rs"), Pos::new(1, 0))]);
    e.active = 0;
    feed(&mut e, "G'A");
    assert_eq!(cur(&e), (1, 0), "in its own file it is a plain jump");
}

#[test]
fn ctrl_o_returns_to_another_file() {
    let mut e = two_files();
    feed(&mut e, "G");
    e.active = 1;
    assert_eq!(feed(&mut e, "<c-o>"), vec![Action::OpenAt(PathBuf::from("/tmp/project/a.rs"), Pos::new(0, 0))]);
}

#[test]
fn gf_opens_the_file_under_the_cursor() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("x.txt"), "hi").unwrap();
    let mut e = ed("see x.txt here");
    e.docs[0].buf.path = Some(dir.path().join("main.rs"));
    at(&mut e, 0, 5);
    assert_eq!(feed(&mut e, "gf"), vec![Action::Open(dir.path().join("x.txt"))]);
    at(&mut e, 0, 11);
    assert!(feed(&mut e, "gf").is_empty());
    assert!(e.msg.as_ref().is_some_and(|m| m.text.starts_with("E447")));
}

#[test]
fn rename_and_code_action_commands() {
    let mut e = ed("x");
    assert_eq!(feed(&mut e, ":Rename new_name<cr>"), vec![Action::Rename("new_name".into())]);
    let acts = feed(&mut e, ":Rename<cr>");
    assert!(matches!(acts.as_slice(), [Action::Error(m)] if m.starts_with("E471")));
    assert_eq!(feed(&mut e, ":CodeAction<cr>"), vec![Action::CodeActions]);
}

#[test]
fn code_objects_and_motions_say_why_they_found_nothing() {
    // The buffer has no path, so no grammar covers it, with or without tree-sitter.
    let mut e = ed("fn main() {\n    let x = 1;\n}");
    at(&mut e, 1, 4);
    feed(&mut e, "daf");
    assert_eq!(text(&e), "fn main() {\n    let x = 1;\n}");
    assert!(e.msg.as_ref().is_some_and(|m| m.error && m.text.starts_with("E: no")));
    e.msg = None;
    feed(&mut e, "]f");
    assert_eq!(cur(&e), (1, 4));
    assert!(e.msg.as_ref().is_some_and(|m| m.error));
    feed(&mut e, "vic");
    assert_eq!(e.mode, Mode::Visual(crate::editor::VisualKind::Char), "Visual stays as it was");
    assert_eq!(e.doc().view.anchor, Some(Pos::new(1, 4)));
    feed(&mut e, "<esc>[a]F[C");
    assert_eq!(cur(&e), (1, 4));
}

#[test]
fn vim_keys_drop_extra_cursors() {
    let mut e = ed("ab\ncd");
    e.docs[0].extra.push(crate::editor::Cursor { pos: Pos::new(1, 0), anchor: None });
    feed(&mut e, "l");
    assert!(e.doc().extra.is_empty());
}
