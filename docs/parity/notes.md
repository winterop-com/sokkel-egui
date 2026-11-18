# Notes: sokkel-ui's notes example in egui

sokkel-ui's notes example (Tidevann's vault of linked markdown notes: every note, a folder's,
a note's page, the daily notes, tags, the graph of links and search, on a simulated backend),
ported one to one as `examples/notes`. Its manifest is `parity/notes.json`; the web's screens are
shot into `parity/notes/web/` by `bun scripts/parity.ts web notes`.

## Where it stands

- The data is the web's to the value (`model.rs`, `seed.rs`, `vault.rs`, `wiki_links.rs`,
  `search.rs`, `backend.rs`, checked in their tests against the TypeScript run under bun): the
  vault, its links, tags, mentions and outline, the tree's order, search, and the backend's
  notes, folders, drafts and tabs.
- The frame: the rail's places with the vault's body under them (`RailBody`: its title, the
  filter with New note and New folder beside it, the tree), the account at its foot, the top bar
  with the search field at its start (`TopBar::start`, new in the library) and the palette's
  button as the web draws it beside a field, only its keys in the code face, and the status line
  with the save, the words, the links in and the notes.
- The tree: a menu on each row (`Tree::actions`, new in the library), seen while the row is
  pointed at, its room kept so a long name is cut short as the web's is: a folder's New note, New
  folder, Rename, Move to and Delete; a note's Open, Rename, Move to and Delete.
- All notes and a folder: the header with the folder's buttons, the field that makes a note, and
  the notes newest change first in a list table (`Cell::IconName`, new in the library: a note's
  kind as its icon, then its name), sorted, reordered by hand when the vault's order is manual,
  each row's menu.
- A note: the open notes as tabs over it (`DocumentTabs`, with Recent and New note at the strip's
  end), its name renamed in place in a pane's head (`PaneHeader`, new in the library, and
  `InlineEdit::weight`), Edit, Both or Read, and its markdown in the code editor beside the page it
  makes (`EditPreview`, new in the library, on `SplitPane::only`), the links followed and the tags
  opened. Mod-E steps the view and Mod-S saves.
- Beside a note, in the shell's panel: its linked and unlinked mentions, each line drawn as
  markdown with the name marked, Link to turn a mention into a link, the links out (Make for a
  name with no note yet), its tags, and its outline (`Item`, `item_group` and `TableOfContents`,
  new in the library). A side panel's sections stand right under its head, each heading on a 20
  row, as the web's.
- Daily: today's note started or opened, a month to jump to any day with the days written dotted
  (`MiniMonth`, new in the library), the template as it reads, and the days written.
- Tags: every tag with how many notes carry it, filtered, the most used as bars (`BarList` takes
  a closure to write a value), and one tag's notes (`Cell::TagName`, new in the library). A list's
  chevron column keeps the column gap before it, as the web's grid.
- Search: every note holding the words, in its name or its text, with the line each says them in,
  narrowed by kind and folder (`SearchResults`, new in the library, with the search's shapes as
  the web's `lib/search`: `SearchResult`, `SearchGroup`, `SearchFacet`, `SearchChosen`); the
  address follows the query.
- Settings over the page, at the section the address names: the editor's text size, wrap, line
  numbers and the view a note opens in; the daily notes' folder and template (kept once typing
  rests half a second); the look with the app's carried themes; and the vault's order, its tabs,
  the recent notes and the demo's data put back after asking.

## Open

- Graph.
- The dialogs that rename, move and delete, and New folder; the palette and the switcher.
- QuickAdd says nothing yet when a title is already in the vault (the web's `validate`).
- The web's note editor draws in the sans face at c95b8911; that is a sokkel-ui bug (its lean
  stylesheet dropped `--font-mono`), so the egui editor keeps the mono face.
