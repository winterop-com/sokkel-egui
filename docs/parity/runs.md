# Runs: sokkel-ui's runs example in egui

sokkel-ui's runs example (Tidevann's jobs console: an overview, runs going live, a run's page,
jobs, schedules and settings, on a simulated backend), ported one to one as `examples/runs`,
in place of the earlier app of the same name. Its manifest is `parity/runs.json`; the web's
screens are shot into `parity/runs/web/` by `bun scripts/parity.ts web runs`.

## Where it stands

- The data is the web's to the value (`model.rs`, `seed.rs`, `schedule.rs`, `timeline.rs`,
  `logs.rs`, `backend.rs`, checked in their tests against the TypeScript run under bun): the
  jobs, the 48 hours of seeded history, each schedule's fires, a run's steps and log at any
  moment, and the backend's engine that starts, settles, cancels and retries runs.
- The frame: the rail with the runs going, the trail, Settings in the top bar, the status line
  with the next fire (or the scheduler off), the runs going and the failures today, and a run
  beside the page in the shell's side panel (`Shell::panel` and `SidePanel`, new in the library).
- Runs: every run newest first, searched, filtered (`FilterBar`) and kept as saved views, each
  row's menu (Open, Open page, Retry, Copy run ID, Cancel run, the cancel asked first), and Run a
  job with the jobs in the code face (`MenuItem::code`, new in the library). A run's step is in
  the code face, muted (`Cell::MutedCode`, new). A selectable table's check column is now the
  web's 40, and a row's menu stands pinned at the row's very end, as the web's end column of a
  table with a least width (runs, invoices, responses); a table that fits its room, as the web's
  `minWidth="auto"` (`DataTable::fits`, new in the library), keeps the row's padding after it.
- A run beside the page: Details (its buttons, a failure or a skip as a `Problem`, its facts, its
  inputs and outputs as a `JsonTree`, both new in the library), its Steps as a timeline, and its
  Log.
- Jobs: every job with its schedule (or paused), its next run, its last run as a chip and when
  (`Cell::ChipNote`, new in the library), its runs over two days as a line and its success, each
  named over its code (`Cell::NamedCode` at the body's weight), searched and filtered by
  project, each row's menu (Open, Run now, Pause or Resume, Delete after asking), and New: a job
  made from a starter with its code in the code face (`Input::mono`, new in the library), its
  project, owner, steps and schedule. Open: the web's line over two days is 16 tall.
- Schedules: the fires ahead over 24 hours, 3 days or a week, the scheduler's banner while it
  is off, the runs and the fires on a `RunTimeline` (new in the library, after sokkel-design's
  `run-timeline` spec: the legend, the ruler on the clock, a lane per job, the chosen lane's
  upcoming runs dragged sideways as one, snapped, and its schedule moved), the jobs not on a
  schedule, and the fires by day (a name and its code on one line, `Cell::NameThenCode`, new),
  filtered by job. A table's toolbar with no search stands in the search's place, as the web's.
- Settings over the page: how a run opens and how times read, the notes to hear, the log, the
  look with the app's carried themes alone (`theme_settings_of`, new in the library), the
  scheduler, the account with Sign out, and the demo's data put back after asking; the palette with
  the pages, the jobs, a job to run now, the look, the demo and signing out.
- A run's page: its job, code, state and where it stands, Open job, Retry and Cancel, its failure
  or skip as a `Problem`, its steps as a graph (a step's duration on its box, a failed step's box
  edged in its state, the steps not reached dashed, an edge in its source's hue and dashed into a
  skipped step, all new in the graph crate), a chosen step's facts, its log as tall as its lines
  (`LogView::fit`, new), its facts, inputs and outputs, and its steps as a timeline. A `Columns`
  cell now stacks what it holds, as a grid's cell does.
- The sign-in page after signing out: the brand pane with the flow-lines art (`FlowLines`, its
  seeded graph to the value, as the web's `flowGraph`), the mark, the name and the facts, and the
  form; ada, grace, linus or margaret sign in with "tide" (`SignIn`, new in the library). Its
  fields draw their own icons and the password's show button; the web's other ways in (a server,
  a token) are not ported, the example asks for none.
- A job's page: its name edited in place, its schedule, owner, project and runs, Run now, Pause
  and Delete, its steps as a graph and a chosen step's facts, its schedule and settings each saved
  as it is changed (`InlineEdit::display`, `::placeholder` and `::multiline`, new in the library),
  and its recent runs. A description list's term stands level with its value's first line. Open:
  the time, day and number editors are typed as text with the value checked, not the web's
  pickers; the recent runs' last column fills the row, where the web leaves the room empty.
- The overview: the panels the person arranges and keeps (Edit, the palette of the others), with
  the refresh: the figures of the last 24 hours as stat panels, the runs per hour by state,
  failures and runs by job and by trigger, each run's time against its job's median as a box plot
  (`BoxPlotPanel` with the web's `spreadOf`, new in the charts crate), and the lists of runs in
  trouble, going, latest and next up. A donut now holds its legend inside its body and grows into
  a taller row, as the web's chart does (`legend_height`, and a chart body's room in a stretched
  row, new in the charts crate). Open: a chart panel's body keeps 8 over and under it where the
  web's has none, so the overview's rows stand some 24 taller.
- A table narrower than its least width scrolls sideways as the web's (the runs with a run open
  beside them): its first column and its end column stay pinned, each with the web's 1px line
  beside it on every table that is not a list, the columns between scrolling under them by a
  sideways wheel or the thin bar along the rows' foot. A chevron alone stands in the middle of
  its end column.
- Open: the bulk actions on the selected runs; the log's wrapping.
