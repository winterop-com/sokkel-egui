# Parity: the tasks example

sokkel-ui's `examples/tasks` ported one to one as `examples/tasks`: every screen drawn as the web
draws it, every behavior doing what the web's does. The web example is the reference; when they
differ, egui changes, in the library, never in the example alone.

## Keeping up

`make parity` checks the port against sokkel-ui as GitHub has it, and lays the screens side by side.
1. It fetches sokkel-ui into `.parity/sokkel-ui`, a clone of its own (`scripts/sokkel.ts`).
2. It compares each file of `examples/tasks/web/src` with the blob it was ported from
   (`parity/tasks.json`, `ported`).
   - A changed file is listed, with the `git diff` command that shows the change to port.
   - A new or removed file is listed too.
3. It lists every library module the example imports with no egui counterpart in the manifest's
   `parts`, and fails on one the manifest does not know.
4. It writes `.parity/tasks.html`: each screen from the web beside the same screen in egui.

`make parity-web` rebuilds the web example in the clone at the same commit, and shoots its screens
into `parity/tasks/web/`, keeping a picture that differs only by antialiasing (no channel by more
than 2), and records the commit it shot at (`shot`). The example's own files can stay still while
the library moves under it, so `make parity` also lists the library files the example imports, and
the tokens and styles, changed since that commit: the cue to shoot again and look. Those screens are committed, so git shows how the web side changed. The
egui screens are `examples/tasks/tests/snapshots/`, from `tests/screens.rs`. Both are 1280 by 800,
in the System scheme (light, or dark when named so), on 2026-11-16, from fresh data.

After porting a change, record the new blobs in `parity/tasks.json` (`ported.commit` and
`ported.files`) and shoot the web again.

## Where it stands

Ported from sokkel-ui `a4702e35` (0.25.0). All 34 library modules the example imports have an egui
counterpart.

| Screen | State |
|---|---|
| inbox, today, upcoming, project | Within a point or two of the web; the frame follows sokkel-design's rail, top bar and status line specs (814e3af), and the page header its spec (689452c): the count on the title's baseline. Frame, rail, top bar, toolbar, all five columns, the grip, due tones, avatars, the row's action and the status line all line up. |
| today_empty | `Empty` in the table's frame without its heads: the list's icon, "Nothing in Today", the count of hidden done tasks in balanced lines, and Show done. |
| inbox_dark | The same, in the dark scheme. |
| board | Columns, heads, counts and cards match, the cards' feet included. egui breaks a title only at spaces, where the web also breaks after a hyphen ("send-" / "digest"). |
| board_menu | A card's menu from its button, shown with the grip while the card is hovered: Move to with each column's icon (the card's own disabled), then Delete. Cards drag between and within columns (`Board`); where the web's other cards slide aside, ours show a line where the card lands. |
| edit_title, edit_due, edit_due_calendar, edit_assignee | Editing in place (`Column::editable`): the text editor over its cell with its line under it, the date typed with Clear and a calendar under the cell, and the select's list over the cell with the chosen option where the value was. Selected text keeps its ink as the browser's does, on our token ground rather than the browser's own blue (sokkel-ui has no selection token). Rows are known by the task's id (`DataTable::keys`, the web's `rowKey`): an open editor, the active cell and a refused draft stay with their task when the rows move under them. |
| palette | The palette a third down the window, 384 wide, its search on the input's tint, its rows rounded as radius-lg, and the web's groups: Tasks, Lists and Go to from the rail (`place_commands`), Scheme, Density and Theme with the one in use hinted (`appearance_commands`), and Demo. |
| account | The account menu from the rail's foot: the account with where it is signed in, Settings, and the scheme with each one's icon, the one worn checked. |
| settings | The dialog's size (896 by 704, at most the window less 32 across and 64 down), head, column on the dialog's own ground, search (filtering every section), group titles 16 under the search, rows ruled only between each other, and footer line up, as sokkel-design's settings-dialog spec says. The web blurs the page behind it; egui cannot blur, so ours only dims. |
| settings_appearance | `theme_settings` as the web's `themeSettings`: the theme select with each theme's sample, scheme, density and motion, the options as wide as the widest. |
| settings_demo | The section has no Reset to defaults (`SettingsSection::reset(false)`). |
| settings_reset | `ConfirmDialog` over the settings, 384 wide (`sm`; a `Dialog` is 512 by default, as the dialog spec says) with its 65 footer, its detail wrapping one word earlier than the web's. |

## What is left, largest first

1. **A save's spinner** while editing in place: an egui save is done when the app's handler
   returns, so there is nothing to wait on yet; a typed character, Backspace and Delete open a
   cell, and the calendar moves by its keys, as the web's do.
2. **Behavior the screens do not show**: the drawer and stacked rows below 768. The one shortcut
   the example binds, the shell's Mod+K for the palette, is answered as the web's is: Cmd or Ctrl
   on macOS (not both), Ctrl elsewhere, and in a field too; whether it closes an open palette
   waits on sokkel-design's question 22.
3. **The live dot's pulse** in the status line (sokkel-design's status line spec); ours is still.
