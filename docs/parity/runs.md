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
- Open: the overview, a run's page, a job's page, the settings, the
  palette and the sign-in page; the bulk actions on the selected runs; a table narrower than its
  least width scrolls sideways on the web and is cut short here; the log's wrapping.
