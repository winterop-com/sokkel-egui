# sokkel-design's open questions, decided

The owner asked for these to be settled on 2026-11-14 by this session and a second opinion from
Codex, agreeing on each. They answer `sokkel-design/open-questions.md` as of sokkel-design `d86b0ae`:
D1 to D10 are its Decisions, F0 to F24 the rows of "Findings that need a call" in table order (F0
the DataTable row). sokkel-ui and sokkel-design take them up in their own repositories; this
repository follows them in the egui parts (see "In egui" at the end).

## Decisions

- **D1.** Specs now, tokens at 1.0: `tokens.json` stays in sokkel-ui, sokkel-design reads it at a
  pinned sokkel-ui version and fails a spec that names a token that version lacks.
- **D2.** The resolver moves to sokkel-design as it is, in TypeScript, beside the specs whose paints
  it resolves; `verify-colors` runs in sokkel-design's CI against a pinned sokkel-ui.
- **D3.** Lockstep with sokkel-ui before 1.0; egui states the sokkel-ui version whose contract it
  meets. Revisit at 1.0.
- **D4.** No version per part: one contract version, named parts, a `draft`/`stable` status, a
  changelog.
- **D5.** The shell now; other patterns when an egui app needs them.
- **D6.** One app shape as data is the plan (places, route ids, settings sections, commands,
  shortcuts, words, appearance keys), with starters in each renderer's `examples/`. Its schema and
  fixtures come when a second app is ported, sokkel-ui's tasks example the first to read it. No web
  pages inside an egui app, no scaffolder yet.
- **D7.** The listed minimum is the gate before an egui app follows the shell, behavior and
  accessibility included: the rail's resize, the top bar's search field, Section, keyboard
  navigation, focus returned, the shortcuts registry, a back stack, the appearance kept.
- **D8.** An egui story copies the data of the sokkel-ui story at its path, the 500-run tables
  included; the pairs are a CI artifact until the format settles.
- **D9.** sokkel-ui keeps its words as functions; the check samples them (zero, one, many, the
  interpolations).
- **D10.** Every color and ring a spec paints is drawn from the resolved paints, with a check that
  fails local mixing on those paths; sizes stay in `sizes`; colors an app supplies at run time stay
  free.

## Findings

| | Decided |
|---|---|
| F0 | Sorting by keys: Up from the first row enters the head row, Left and Right choose a column, Enter or Space cycles ascending, descending, as given; Down returns to the rows. The table keeps one tab stop. |
| F1 | One keyboard focus look for the frame's controls, the button's own: a 2px outline in `ring`, 2 off, on keyboard focus only. |
| F2 | The top bar's menu button looks like the rail's collapse control: `hover` ground and `foreground` ink under the pointer, F1's focus; a finger square below 768. |
| F3 | `StatusLine` owns its strip (`spacing-shell-foot` tall, `page-foot-ground`, a 1px `page-foot-line` rule over it, 12 across); `Shell` gives it the room and draws no strip of its own. |
| F4 | Actions last: the search, the trail, the palette's button, then the screen's actions; 12 between those groups, 6 between actions. |
| F5 | No rule under the rail's head, in the frame or the drawer; `Shell`'s doc comment is corrected. |
| F6 | Inside a `Shell` the page header draws no trail (`trailInTop` by default); outside one it keeps it. |
| F7 | Unframed means no ground and no edge in every tone. |
| F8 | A section's boxed head keeps its title's own case; it can hold names such as job names. |
| F9 | A count beside a title looks the same in both heads: `text.help`, the body face, tabular figures, `muted-foreground`, 8 after the title. The code face is for codes. |
| F10 | Every select list pads its rows 4, in a group or not. |
| F11 | A one-line select or menu row is `density.control` tall (32, 28 compact), a finger tall below 768. |
| F12 | 8 between a lead (icon or swatch) and its label, in the trigger and the list. |
| F13 | A swatch is a general option lead of `SelectField`, drawn by the same part as the theme picker's. |
| F14 | One tooltip, the inverted one: `foreground` ground, `background` ink, 12 by 6, `radius-md`, a 10 arrow, no edge or shadow. |
| F15 | A tooltip waits 500 ms under the pointer, from the primitive's own default; on keyboard focus it shows at once. |
| F16 | Every popup (select list, menu, submenu, the account menu) wears `popover` and `popover-foreground`, `radius-surface`, a 1px `border-strong` edge and `elevation-pop`; no Tailwind shadow, no look of its own. |
| F17 | One line, 768: below it a dialog is a full-screen sheet and settings a list of sections to go into; a ConfirmDialog stays a box sized to its words. |
| F18 | No blur: both renderers draw the same backdrop. |
| F19 | A boxed dialog, the settings dialog and the palette wear `elevation-dialog` (at first the same as `elevation-pop`) and a 1px `border-strong` edge; a full-screen sheet has no shadow. |
| F20 | One highlight for every list of choices, the palette's active row included: `accent` ground, `accent-foreground` ink for its icon, label and meta. |
| F21 | Every list of choices (palette, menu, select) stops at its first and last row; Home and End reach them. |
| F22 | cmdk's vim keys are off; Mod+K toggles the palette, in a `Shell` or not. |
| F23 | A palette whose command is running holds open (Escape, a press outside, its close and Mod+K included) until it settles; a refusal stays shown. |
| F24 | A meta is `muted-foreground` on one line or two, `accent-foreground` on the active row; a status tone stays. |

## Tokens

Adopt now: the control heights (landed), the focus ring (2 wide, 2 off; invalid 3 wide in
`critical` at 20 percent light and 40 dark), `disabled-opacity` 0.5, motion (`motion-fast` 100,
`motion-base` 200, `motion-ease` ease-out, stilled when motion is reduced), the layers
(`layer-drawer`, `layer-dialog`, `layer-popover`, `layer-toast`), structured elevation (layers of
x, y, blur, spread, color, with `elevation-dialog` added), the inks' fallbacks (`page-title-ink` and
`section-head-ink` to `foreground`), the breakpoints (`breakpoint-md` 768, `breakpoint-lg` 1024),
and the scroll bar (10 wide, a solid track that keeps its room).

Later: a named spacing scale (the specs keep exact part spacings meanwhile) and the state mixes
(`secondary-hover`, `row-critical`, `row-warning`), resolved as paints until a theme needs them.

## In egui

What this repository changes for them, in the parts sokkel-egui draws: F0 (the table's head by
keys), F1 and F2 (the frame's focus and the menu button), F9 (the count), F11 (row heights), F12,
F14 and F15 (the tooltip), F16 and F19 (popups and dialogs, from a resolved `elevation-pop`), F17
(sheets below 768), F20, F21, F22 and F24 (the lists of choices). F3 to F8, F10, F13 and F18 need
nothing here: egui already draws them so. F23's dialog half is in (a busy `ConfirmDialog` releases after `busy-release` with "Stop waiting");
and so is its palette half: `CommandPalette::running` holds the palette while the app's command
runs (its row saying Running), and after `busy-release` a Stop waiting row and Escape let it go
(`PaletteOutcome::stop_waiting`, where the app stops the work, as the web's abort signal does).
0.28.1's `searching` too: `CommandPalette::searching` says Searching after `wait-shown` where
nothing is listed, or dims and passes over the earlier rows. D10 is under way: egui reads the
resolved paints (`Tokens::paint`) for the button, the text field's edge, the popup and dialog
surfaces with the dialog's shade and footer, the tooltip, and the menu's and select's highlighted
row, and `crates/core/tests/paints.rs` fails a paint egui reads that a theme or scheme lacks;
none of these drew differently from its paint. D7's list is egui's work after these.
