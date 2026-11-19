# FHIR: sokkel-ui's fhir example in egui

sokkel-ui's fhir example (Tidevann's FHIR workbench: a FHIR server's resources read and edited as
JSON, checked against their types and kept in versions, FHIRPath evaluated against a resource or
the whole server, CQL libraries in versions with the diff between any two, and search by
parameters, on a simulated server), ported one to one as `examples/fhir`. Its manifest is
`parity/fhir.json`; the web's screens are shot into `parity/fhir/web/` by
`bun scripts/parity.ts web fhir`.

## Where it stands

- The data is the web's to the value (`model.rs`, `types.rs`, `seed.rs`, `validate.rs`,
  `fhirpath.rs`, `evaluate.rs`, `search.rs`, `server.rs`, and each page's words and logic in `ui.rs`,
  `app.rs`, `editing.rs` and `pages.rs`, checked in `tests/web.rs` against the web's own modules run
  under bun). The line diff a version is drawn against the editor with is now the core's
  (`sokkel_egui::diff_lines`, `fold_diff`), and FHIRPath's `matches` tests with the core's patterns
  (`sokkel_egui::regex`), where the example carried copies of both. The server gained `restore`,
  the rows eframe's storage keeps put back on a new visit, and its rows, the kept values and the
  drafts are serializable for it.
- The frame (`workbench.rs`): the rail with Overview and Resources, the resource types under
  Resources with their counts (open while one is shown), then Tools (FHIRPath, CQL with the
  libraries' count, Search), each with its description; the account menu with the server's
  address; the top bar's trail (`harbor` in the code face, the place, the type and the resource or
  library opened, an id in the code face) with its links and the palette's button; the status line
  with the page's note and `unsaved` while an edit is. The address is the web's hash route:
  `overview`, `resources[/<Type>[/<id>|new]]`, `fhirpath[/<Type>/<id>]`, `cql[/<id>]`, `search`,
  and `settings` over the page; anything else is Not found. The server's answers come on the app's
  clock, 150 ms after each call; its toasts are said each frame. The fhir add-on's languages are
  installed once.
- The pages: the overview with the four counts as tiles (each opening its type or the libraries),
  the vital signs measured per month as stacked bars, and the latest ten changes (each opening its
  resource or library); a type's resources as a searchable table with the types as a segmented
  choice with their counts, the type's own columns, the state, the version and the last change
  (sorted by it, newest first), New and the API link; a resource as its JSON in the code editor,
  checked against its type's schema as it is typed, the server's last answer and a conflict marked
  on the values they are about, folding, bracket colors and find, with Save (Mod-s), Revert,
  Delete (asked first), FHIRPath on it and the PUT link; its side panel (400 wide) with Problems
  (each going to its line), History (the versions, the one chosen drawn against the editor),
  References (what it points at and what points at it) and, for a patient, Vitals (body weight,
  heart rate and systolic pressure over the year, each axis fitted); FHIRPath with the subject
  chosen, the expression in the add-on's language with its completions and the words described on
  hover, its error marked where it stops reading, the result or the refusal, the subject's JSON, the
  examples and Evaluate when not live; the CQL libraries and New; a library in the add-on's language
  with its live marks (no declaration, an active version changed), Save, Publish, Revert and its
  Library resource, beside its versions and the one compared drawn against the editor; and search,
  the query in one line with its completions, run with Enter, its matches as a table or the
  searchset Bundle a page at a time, beside the searches kept (run, dragged into another order,
  removed) or the type's parameters (each added to the query), with Save (named in a dialog) and the
  examples.
- Settings: Appearance with the carried themes, the scheme, the density and motion; Editors (line
  numbers, wrap, text size); FHIRPath and search (evaluate as typed, matches per page); Demo
  server's reset (asked first); each section's Reset to defaults. The palette: the pages and the
  types, New of each type, Settings and the look, and from two letters typed the resources it finds.
  The server's rows, the kept values and the unsaved edits are saved with eframe's storage.
- New in the library: the add-on `crates/fhir` (`sokkel-egui-fhir`, the counterpart of
  `@winterop-com/sokkel-ui-fhir`): FHIRPath and CQL read by one grammar as the web's stream parser
  reads them (`program.rs`, held against the web's `languages.test.ts` in `tests/languages.rs`) and
  `fhir()`, installed with `register_plugins` as the dhis2 add-on is. In the core: `DiffView` with
  `diff_lines`, `fold_diff` and `diff_counts` (after sokkel-ui's `DiffView`: the counts at the top
  end or beside a title, the far unchanged lines folded into a row that opens them);
  `sokkel_egui::regex` made public; a rail entry's own `entries`, folded under a chevron and open
  while one of them is shown, the parent set apart over them, and `place_commands` listing them;
  `Shell::panel_width`, the side panel's first width. In the charts add-on: `fit` fits a line's
  value axis to its values, as the web's `fit`. Fixed: a page header's actions wrap onto a row of
  their own under the words when the words would have less than their basis beside them (16rem
  with a description or meta, else 8rem), as the web's wrapping row, where they overlapped the
  meta line; a stacked list's text lead (a version's `v3`) is as wide as its words where it was
  cut to a mark's 16. Stories: Fhir/FHIRPath and CQL, Patterns/Diff view, Patterns/Rail nested
  places.

## Open

- A date and time reads "Nov 3, 2026 at 9:05 AM" where the web's screens, shot in Chrome, read
  "Nov 3, 2026, 9:05 AM": `ui::when` is held to the web's modules as bun's Intl writes them.
- FHIRPath's result says "in 0.0 ms" where the web measured 9.0: the data layer holds the time an
  evaluation takes still, so the screens and the tests stay the same.
- A problem in the side panel says where it is as its line; the web names a schema finding by its
  path (`Patient.name[0].family`), which the core's `CodeProblem` does not carry.
- Collapsed, the rail marks a parent while one of its places is shown, where the web opens them in
  a flyout beside it.
- As in every example, a table's first column and its chevron column are not ruled off from the
  others as the web's are.
