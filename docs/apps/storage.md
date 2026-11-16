# Storage: a browser for Tidevann's buckets

An app of sokkel-egui's own (`examples/storage`, `make storage`), not a port: Tidevann's
buckets as their jobs leave them, drawn with the library's public parts alone. A part it lacks
goes into the library, never into the app.

## What it does

- Three buckets, simulated the same every run (`files.rs`): harbor-exports, a folder a night of
  nightly-export's orders (CSV), its summary (JSON) and its log; juniper-inventory, the stock
  sync-inventory leaves and two shelf photos; larkspur-index, rebuild-index's archive and manifest
  and its schedule.
- The page: a `SplitPane`, the buckets as a `Tree` on the left (folders counting their files,
  files their size, a search by `/` keeping the matches and the folders above them), and on the
  right what is chosen: a folder's files as a list (name, size, modified, owner), a row pressed
  choosing that file; a file's facts in a `DescriptionList` over a `FilePreview` of what it holds.
- The top bar's trail is the chosen path; the status line counts the files and the bytes stored.

## What it brought into the library

- `Tree`, `TreeNode` and sokkel-ui's tree functions (`visible_rows`, `tree_matches`,
  `tree_ancestors`, `tree_node`): one node chosen, branches opening from the chevron, from Right or
  by being chosen, Left shutting or going to the parent, Up, Down, Home and End walking the rows.
  Its story is Patterns/Tree.
- `FilePreview`, `FileKind`, `format_size` and `parse_csv`: a CSV as a list of its first rows,
  JSON, text and code as numbered lines under the name, any other kind its icon and that it has no
  preview. Its story is Patterns/Files.
- A `DataTable` row is named for a screen reader by its cells' words, so it can be read and pressed.

## Next

- Not yet in the tree: several chosen, children loaded when a branch opens, a row's menu, moving
  rows by dragging, and the search's matches marked in the labels.
- Not yet in the preview: images (on a checkered ground), JSON as a tree to open (`JsonTree`),
  highlighting.
- sokkel-ui's Apps/Files story is the fuller app to port next: uploads, new folders, renames.
