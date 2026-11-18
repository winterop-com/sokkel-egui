# Studio: sokkel-ui's studio example in egui

sokkel-ui's studio example (Tidevann's workspace of pipelines, schemas and queries, the runs of its
pipelines and the tables its queries read, on a simulated backend), ported one to one as
`examples/studio`. Its manifest is `parity/studio.json`; the web's screens are shot into
`parity/studio/web/` by `bun scripts/parity.ts web studio`.

## Where it stands

- The data is the web's to the value (`model.rs`, `seed.rs`, `tables.rs`, `sql.rs`, `yaml.rs`,
  `pipeline.rs`, `pipeline_edit.rs`, `simulate.rs`, `backend.rs`, checked in `tests/web.rs`
  against the web's own modules run under bun): the workspace's files, three weeks of tables, the
  SQL engine its queries run on, pipelines read from their YAML and edited back into it, and runs
  simulated step by step with their logs.
- The frame: the rail's places over the workspace's tree (`RailBody::places_above`, new in the
  library; a place's empty count hidden, as the web's), each project's files by kind with a dot on
  one not saved (`TreeNode::dot`, new), and the tables with their rows; each row's menu (New in a
  project or a group; Open, Rename, Save, Move to, Delete on a file); the trail with the file's
  name in the code face; Save all and New file; and the status line.
- The work area: the open files and tables as tabs with their menu, over the editor the tab names.
- A pipeline: its YAML checked against the pipeline schema with the `${...}` references left to
  the app (`CodeEditor::deferred`, new in the library), completed and described from the
  pipeline's own words, beside its steps on a graph that edits them (the graph add-on's
  `PipelineGraph` with its toolbar, picker, notices and undo), the chosen step's settings as a form
  drawn from the schema (`SchemaForm`), the problems, and its last run's graph and log. Text, Both
  or Graph; Run and Cancel; the editor pops out into a window with the reference beside it
  (`WindowedPane`).
- A schema: checked against JSON Schema 2020-12 (`json_schema_2020_12`, new in the library, BSD 3),
  beside a value tried against it, with the problems of either.
- A query: SQL run over the tables with Run or Mod-Enter, its rows in a table under it, a refusal
  marked where it is; table and column names completed and described.
- A table: its columns and what each holds over its rows, searched (`SplitPane::down`, new).
- Runs and a run's page: every run newest first, searched; a run's state, its failure as a
  `Problem`, its steps live on a `RunGraph`, and its log with the failing step marked.
- The dialogs: a new file's kind, project and name; a rename; a delete; and closing tabs with
  changes not saved (`DialogShell::other`, new in the library: Save, Don't save, Cancel).
- Settings over the page (the editor's text and its aids, the look with the studio's carried
  themes, the workspace's tabs and the demo's data), the palette and the file switcher (Mod-P);
  Mod-Shift-S saves all. A key cap's modifier symbols draw as their icons (`⇧`, `⌥`, `⌃` beside
  `⌘`). A problem's kind in a list is a dot (`Cell::Dot`), and a list may leave out its head row
  (`DataTable::heads`), both new in the library.

## Open

- One undo for the text and the graph: the graph's undo holds every change of the text, but the
  editor's own Mod-Z undoes its own typing only.
- A run's page header says its state in words; the web shows it as a chip in its meta line.
- A table's first column is not pinned with its rule as the web's table variant pins it, and a
  table's bar with no search stands about 6 taller than the web's.
- The tried value is kept as it is typed; the web keeps it half a second after typing stops.
