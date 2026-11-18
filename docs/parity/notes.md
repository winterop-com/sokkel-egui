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

## Open

- A note's page (the editor, its preview, its tabs), the panel with its links and outline,
  Daily, Tags, Graph, Search and Settings.
- The dialogs that rename, move and delete, and New folder; the palette and the switcher.
- QuickAdd says nothing yet when a title is already in the vault (the web's `validate`).
