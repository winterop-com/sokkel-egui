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

Ported from sokkel-ui `a4702e35` (0.25.0). 32 of the 34 library modules the example imports have an
egui counterpart.

| Screen | State |
|---|---|
| inbox, today, upcoming, project | Within a point or two of the web. Frame, rail, top bar, toolbar, all five columns, the grip, due tones, avatars, the row's action and the status line all line up. |
| inbox_dark | The same, in the dark scheme. |
| board | Columns, heads, counts and cards match. A card's title-to-date gap is about 6 taller than the web's. |
| settings | The dialog's size, head, column, search (filtering every section), group titles, rows and footer line up. The web blurs the page behind it; egui cannot blur, so ours only dims. |
| settings_appearance | `theme_settings` as the web's `themeSettings`: the theme select with each theme's sample, scheme, density and motion, the options as wide as the widest. |
| settings_demo | The section has no Reset to defaults (`SettingsSection::reset(false)`). |
| settings_reset | `ConfirmDialog` over the settings, its detail wrapping one word earlier than the web's. |

## What is left, largest first

1. **Editing in place** (`table-editing`): a title's text, a due date from a calendar, an assignee and
   a project from a select, each opened from its cell.
2. **The board's drag**: cards moved between columns and within one, and a card's menu (Move to,
   Delete).
3. **The empty list** (`primitives/empty`): its icon, title, the count of hidden done tasks, and Show
   done.
4. **Behavior the screens do not show**: the palette's groups as the web builds them
   (`placeCommands`), the account menu's scheme, keyboard shortcuts, and the drawer and stacked rows
   below 768.
