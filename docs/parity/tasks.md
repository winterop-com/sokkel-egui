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
| phone | Below 768 (`is_phone`), as sokkel-design's shell pattern says: no rail in the frame, the top bar across the window with the menu button at its start, its last crumb alone and the palette's button a magnifier on a finger square, and the status line across the window. The page lines up with the web's: `Page` takes the narrow inset (16) under 640, `toolbar` puts the add field across the row and its end on a row of its own under 448, `Segmented` options are a finger tall, and the list stacks (`StackLayout::Compact`, the box a `Stack::Lead`) as the title over a quiet line of due, assignee and project, the menu at the row's end. The last row the list cuts is cut plainly, as the web's: `install` turns egui's scroll fade off, and only the palette's list fades its foot, painted as the web's mask fades it. |
| phone_drawer | The menu button opens the rail as a drawer over a scrim (`Shell` holds it): expanded, without its edge, finger-tall entries, the close at its head's end and the account at its foot. A place chosen in it, Escape, the close or a press on the scrim closes it. The menu button's hover and focus wait on sokkel-design's question 2. |
| phone_edit_title | A stacked row edits in place as the web's compact rows do: a press on the title opens its editor as a field of its own (rounded, its edge in the ring's color), a finger tall from the title's line over the facts, the row 8 taller; a press on a fact (due, assignee, project) opens its editor after the column's name in the facts line's place. The field keeps the title's start in view where the web's scrolls to its end. |
| board | Columns, heads, counts and cards match, the cards' feet included. A title breaks where the browser breaks it, after a space or a hyphen ("send-" / "digest"), where egui alone would break after a hyphen only on a line with no space. |
| board_menu | A card's menu from its button, shown with the grip while the card is hovered: Move to with each column's icon (the card's own disabled), then Delete. Cards drag between and within columns (`Board`); where the web's other cards slide aside, ours show a line where the card lands. |
| edit_title, edit_due, edit_due_calendar, edit_assignee | Editing in place (`Column::editable`): the text editor over its cell with its line under it, the date typed with Clear and a calendar under the cell (its buttons' icons in the foreground's ink under the pointer; the web's shot keeps the pointer over the calendar button, ours takes it away), and the select's list over the cell with the chosen option where the value was. Selected text keeps its ink as the browser's does, on our token ground rather than the browser's own blue (sokkel-ui has no selection token). Rows are known by the task's id (`DataTable::keys`, the web's `rowKey`): an open editor, the active cell and a refused draft stay with their task when the rows move under them. |
| palette | The palette a third down the window, 384 wide, its search on the input's tint, its rows rounded as radius-lg, its list fading at its foot while more is below (shadcn's `scroll-fade-b`, painted over the list), and the web's groups: Tasks, Lists and Go to from the rail (`place_commands`), Scheme, Density and Theme with the one in use hinted (`appearance_commands`), and Demo. |
| account | The account menu from the rail's foot, 4 in from the trigger's start (`alignOffset`), its rows' labels 10 after their icons and a finger tall on a phone: the account with where it is signed in, Settings, and the scheme with each one's icon, the one worn checked. |
| settings | The search takes the focus as the dialog opens from 640 (on a phone nothing does), as the spec says. The dialog's size (896 by 704, at most the window less 32 across and 64 down), head, column on the dialog's own ground, search (filtering every section), group titles 16 under the search, rows ruled only between each other, and footer line up, as sokkel-design's settings-dialog spec says. |
| settings_appearance | `theme_settings` as the web's `themeSettings`: the theme select with each theme's sample, scheme, density and motion, the options as wide as the widest. |
| settings_demo | The section has no Reset to defaults (`SettingsSection::reset(false)`). |
| settings_reset | `ConfirmDialog` over the settings, 384 wide (`sm`; a `Dialog` is 512 by default, as the dialog spec says) with its 65 footer, its detail wrapping one word earlier than the web's. |

## Decided ahead of the web

sokkel-design's open questions were decided on 2026-11-14 (`docs/proposals/open-questions-decided.md`,
F0 to F24). egui follows them now and the web when sokkel-ui lands them, so on these the two sides
differ on purpose until then; a difference listed here is not drift to undo:

- F16, F19: every popup wears a `border-strong` edge and `elevation-pop`, read from the token (the
  web's select list and menus still wear Tailwind's `shadow-md` and a faint edge); dialogs, the
  settings dialog and the palette wear `elevation-dialog` (at first `elevation-pop`) and the same
  edge, where the web's dialogs are flat. The drawer wears `elevation-pop`.
- F11, F12, F20, F21, F22, F24: every list of choices (menus, the select's list, the table's
  select, the account menu) has rows a control tall (32, 28 compact; a finger on a phone) and 8
  from a lead to its label; the palette's active row is lit in `accent` with `accent-foreground`
  ink, as the menus' are (the web's is `muted`); the palette's arrows stop at its ends, Home and
  End reach them, and Mod+K closes an open palette.
- F1, F2 (focus_rail, phone_menu_focus, egui's own shots): the frame's controls (rail entries, the
  account trigger, the drawer's close, the top bar's menu button) take the button's keyboard focus,
  a 2px `ring` outline 2 off, where the web's rail takes the browser's outline in `ring/50`; the
  menu button and the drawer's close turn `foreground` under the pointer.
- F18: no blur behind a dialog, on either side once the web drops its own.

## What is left, largest first

1. **A save's spinner** while editing in place: an egui save is done when the app's handler
   returns, so there is nothing to wait on yet; a typed character, Backspace and Delete open a
   cell, and the calendar moves by its keys, as the web's do.
2. **Behavior the screens do not show**: The one shortcut
   the example binds, the shell's Mod+K for the palette, is answered as the web's is: Cmd or Ctrl
   on macOS (not both), Ctrl elsewhere, and in a field too; whether it closes an open palette
   waits on sokkel-design's question 22.
