# Keys

editor has two keymaps. Setup asks which one you want. To switch later, run
`:set keymap=classic` or `:set keymap=neovim`, or use Settings › keys & mouse.

## Neovim keymap

Space is the leader by default. To use backslash, as in stock Neovim, set `keys.leader` to
`backslash`. Setup can also read the leader from `~/.config/nvim`.

### Leader menu

Press the leader alone to see these keys in a popup.

| Keys | Does |
| --- | --- |
| `SPC f` | Find a file |
| `SPC /` | Search the project (ripgrep) |
| `SPC b` | Pick an open buffer |
| `SPC e` | Focus the file tree. Again, hide it |
| `SPC w` | Write |
| `SPC q` | Close the buffer |
| `SPC d` | Diagnostics of this file |
| `SPC ,` | Settings |
| `SPC g n`, `SPC g p` | Next, previous git change |
| `SPC g s` | Focus the tree, with git status |
| `SPC l d` | Go to definition |
| `SPC l r` | References |
| `SPC l h` | Hover |
| `SPC l n` | Rename symbol |
| `SPC l a` | Code actions |
| `SPC a l` | Changes made on disk from outside the editor |
| `SPC a c` | Clear the outside-change marks |
| `SPC a p` | Show the next agent proposal |
| `SPC a n` | Clear the agent's note |

### Motions

All motions take a count.

| Keys | Moves to |
| --- | --- |
| `h` `j` `k` `l`, arrows | Left, down, up, right |
| `w` `b` `e` `ge` | Word starts and ends |
| `W` `B` `E` `gE` | WORD starts and ends |
| `0` `^` `$` `g_` | Line start, first non-blank, line end, last non-blank |
| `gg` `G` `{N}G` | First line, last line, line N |
| `f` `F` `t` `T` `;` `,` | A character on the line, and repeats |
| `%` | The matching bracket |
| `{` `}` | Paragraphs |
| `(` `)` | Sentences |
| `H` `M` `L` | Top, middle, bottom of the screen |
| `*` `#` `n` `N` | The word under the cursor, the next and previous match |
| `]f` `[f`, `]F` `[F` | Next, previous function start, end (tree-sitter) |
| `]c` `[c`, `]C` `[C` | Next, previous class start, end (tree-sitter) |
| `]a` `[a` | Next, previous parameter (tree-sitter) |
| `Ctrl d` `Ctrl u` `Ctrl f` `Ctrl b` | Half and full pages |
| `Ctrl e` `Ctrl y`, `zz` `zt` `zb` | Scroll the view |
| `Ctrl o` `Ctrl i`, `''` | The jump list, across files |
| `` `a `` `'a` | Mark `a`. `A` to `Z` jump across files |
| `gd`, `gf` | Definition (LSP), the file under the cursor |

### Operators and text objects

| Operator | Does |
| --- | --- |
| `d` `c` `y` | Delete, change, yank |
| `>` `<` `=` | Indent, outdent, reindent |
| `gu` `gU` `g~` | Lower, upper, toggle case |
| `gc` | Toggle comments |

Double an operator for the whole line: `dd`, `cc`, `yy`, `>>`, `gcc`, `gUU`. An operator also
takes a search: `d/pat<Enter>` deletes up to the next match.

| Object | Around | Inner |
| --- | --- | --- |
| Word, WORD | `aw` `aW` | `iw` `iW` |
| Sentence, paragraph | `as` `ap` | `is` `ip` |
| Brackets | `a(` `a[` `a{` `a<` (`ab`, `aB`) | `i(` `i[` `i{` `i<` |
| Quotes | ``a" a' a` `` | ``i" i' i` `` |
| HTML tag | `at` | `it` |
| Function (tree-sitter) | `af` | `if` |
| Class, struct, impl (tree-sitter) | `ac` | `ic` |
| Parameter (tree-sitter) | `aa` | `ia` |
| Comment (tree-sitter) | `a/` | `i/` |

Counts work on objects too: `d2af` deletes the function around the one you are in.

### Other Normal mode keys

| Keys | Does |
| --- | --- |
| `i` `a` `I` `A` `o` `O` `gi` | Insert |
| `R` | Replace mode: type over text. Backspace puts the old text back |
| `x` `X` `s` `S` `D` `C` `Y` | Short forms of delete, change and yank |
| `p` `P` | Paste after, before. The unnamed, `+` and `*` registers use the system clipboard |
| `J` `gJ` | Join lines |
| `r{c}` `~` | Replace one char, toggle case |
| `u` `Ctrl r` `U` | Undo, redo, undo the changes on one line |
| `.` | Repeat the last change, Visual changes included |
| `Ctrl a` `Ctrl x` | Add to, subtract from the number under the cursor |
| `"a` | Use register `a`. `"A` appends |
| `qa` … `q`, `@a`, `@@` | Record and play macros |
| `ma` | Set mark `a` |
| `gn` `cgn` | Select, change the next search match. `.` repeats |
| `gt` `gT` | Next, previous tab |
| `K` | Hover (LSP) |
| `ZZ` `ZQ` | Write and close, close without writing |

### Insert mode

| Keys | Does |
| --- | --- |
| `Esc`, `Ctrl [` | Back to Normal |
| `Ctrl w` `Ctrl u` | Delete a word, delete to the line start |
| `Ctrl t` `Ctrl d` | Indent, outdent the line |
| `Ctrl r a` | Insert register `a` |
| `Ctrl o` | Run one Normal command |
| `Ctrl n` `Ctrl p`, `Tab` `Enter` | Move in the completion list, take an item |

### Visual mode

`v`, `V` and `Ctrl v` start character, line and block selection. All motions and text objects
extend it, and `o` swaps its ends. `d c y > < ~ u U J gc r p` and `:` act on it. In block mode,
`I` and `A` type on every line.

### Ex commands

| Command | Does |
| --- | --- |
| `:w` `:w path` `:wa` | Write, write as, write all |
| `:q` `:q!` `:wq` `:x` `:qa` `:qa!` | Close and quit |
| `:e path` `:e!` | Open, reload from disk |
| `:{N}` | Go to line N |
| `:s/pat/rep/gi` `:%s` `:'<,'>s` | Substitute. The `c` flag asks before each change |
| `:g/pat/d` `:v/pat/d` | Delete matching, non-matching lines |
| `:d` `:y` `:m` `:t` `:>` `:<` `:j` `:sort` `:normal` | Line commands, with ranges |
| `:set opt`, `:set noopt`, `:set opt=val` | Options: `keymap`, `wrap`, `number`, `relativenumber`, `tabstop`, `expandtab`, `scrolloff`, `list`, `cursorline`, `colorcolumn`, `mouse`, `theme` |
| `:noh` | Hide search highlights |
| `:marks` `:delmarks` | List, delete marks |
| `:'<,'>!cmd` | Filter the lines through a shell command |
| `:!cmd` | Run a shell command in the project folder |
| `:Rename name` | Rename the symbol under the cursor (LSP) |
| `:CodeAction` | Code actions (LSP) |
| `:Settings` `:Find` `:Grep` | Settings, the file finder, project search |

## Classic keymap

There are no modes: you type at once. `Ctrl` is `⌘` on macOS.

| Keys | Does |
| --- | --- |
| `Ctrl S` | Save |
| `Ctrl P` | Go to file |
| `Ctrl ⇧ P` | Command palette |
| `Ctrl F`, `Alt Ctrl F` | Find in file, replace in file |
| `Ctrl ⇧ F` | Find in project |
| `Ctrl Z`, `Ctrl ⇧ Z` or `Ctrl Y` | Undo, redo |
| `Ctrl C` `Ctrl X` `Ctrl V` | Copy, cut, paste. With no selection, the whole line |
| `Ctrl A` | Select all |
| `Ctrl ⇧ K` | Delete the line |
| `Alt ↑` `Alt ↓` | Move lines |
| `Ctrl /` | Toggle comments |
| `Ctrl ]` `Ctrl [`, `Tab` `⇧ Tab` | Indent, outdent |
| `F12`, `F2`, `Ctrl .` | Definition, rename, code actions |
| `Ctrl B` | Toggle the file tree |
| `Ctrl Tab` `Ctrl ⇧ Tab` | Next, previous tab |
| `Ctrl W` | Close the tab |
| `Ctrl ,` | Settings |

Shift with arrows, Home and End selects. Word moves are `Ctrl` arrows on Linux and Windows,
and `Alt` arrows on macOS.

### Multiple cursors

| Keys | Does |
| --- | --- |
| `Ctrl D` | Select the word, then add a cursor at each next match |
| `Ctrl ⇧ L` | Select every match |
| `Ctrl Alt ↑` `Ctrl Alt ↓` | Add a cursor above, below |
| Alt-click | Add a cursor, or remove the one under the pointer |
| `Esc` | Back to one cursor |

Typing, deleting, moving, cut, copy and paste act at every cursor, as one undo step. A paste
with one line per cursor puts one line at each.

## Everywhere

| Keys | Where | Does |
| --- | --- | --- |
| `j` `k` `l` `h`, `Enter`, `Esc` | File tree | Move, open, close a folder, leave |
| `y`, `n`, `]c` `[c`, `j` `k` | An agent's proposal | Accept, reject, next and previous change, scroll |
| `Ctrl n` `Ctrl p`, arrows, `Tab` | Pickers | Move |
| `Ctrl w` `Ctrl u` | Pickers | Delete a word, clear the query |
| `j` `k`, `h` `l`, `1`–`5`, `Tab` | Settings | Move, change, pick a section |
| `Tab` `⇧ Tab`, `j` `k`, `Enter` | Setup | Next step, back, move, pick |

## Mouse

Turn the mouse on or off in setup or Settings. When it is on:

- Click places the cursor, drag selects (Visual mode in the Neovim keymap), double-click selects
  a word, and triple-click or a click on a line number selects a line.
- The wheel scrolls. With Shift, it scrolls sideways.
- Right-click opens a menu, when `mouse.right_click` is on.
- A middle-click on a tab closes it, unless it has unsaved changes.
- Drag the file tree's edge to resize it.
