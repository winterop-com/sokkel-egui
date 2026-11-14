# Dashboard: sokkel-ui's dashboard example in egui

sokkel-ui's dashboard example (Tidevann's dashboards: widgets a person arranges, configures and
names), ported one to one as `examples/dashboard` (`make dashboard`). Its screens are drawn by
`examples/dashboard/tests/screens.rs` (overview, operations, team) and
`tests/behavior.rs` (the list of dashboards).

## Comparing a live board

The readings are drawn from a seeded generator (`live.rs`, mulberry32 seeded as the web's), with
the clock starting at Wed 2026-11-18 09:00. Egui's draws are its own, so judge the screens by
their structure, never by their figures.

## Where it stands

- The frame: the rail with All dashboards and each dashboard, Settings under System, the top
  bar's New and Live buttons, and the status line (paused or live, running, sites in trouble).
- The three dashboards Tidevann starts with (Overview, Operations, Team) and all fourteen kinds
  of widget: stat, trend, bars, gauge, runs, activity, log, map, note, to-do list, upcoming,
  uptime, on call and projects (`widgets.rs`).
- Edit arranges a dashboard on `DashboardArrange`: drag to move, the palette to add, the head's
  tools to resize, configure or remove, the keyboard to step; Cancel asks before throwing edits
  away (`guard_cancel`).
- Configure (`dialogs.rs`): a widget's title and what its kind reads. New and Rename: a
  dashboard's name, checked against the others, and what a new one starts as.
- The list of dashboards: reordered by their grips, each row's menu to open, rename, duplicate or
  delete (asked first). The rail follows the list's order.
- Settings: live updates and their pace, editing, appearance, and resetting the demo.
- A toast says what was done, as the web's `actions.ts`: a dashboard made, its copy made, or
  deleted.
- The sites map draws its points alone, as the web's point layer; a point pressed opens its card
  (`MapPopup`): the city, its country, its state and its round trip.
- The uptime bars show the latest days that fit their width, as sokkel-ui's `UptimeBar` does, so
  in a third-wide panel the first day reads "81 days ago", not 89.

## What it brought into the library

- `StatPanel`, `Threshold`, `format_value`, `threshold_color`; and in the charts add-on,
  `GaugePanel`, `BarPanel`, `Gauge` and `BarChart::horizontal`. Story: Charts/Dashboard panels.
- `ActivityList`, `BarList`, `LogView` and `Markdown` (headings, paragraphs, lists, bold, italic,
  code). Stories: Patterns/Activity list, Bar list, Log view, Markdown.
- `ArrangeItem`, so `DashboardArrange` and `DashboardEdit` take any app's widgets.
- `DataTable::framed`, `Gap::Xs`, and a `Checkbox` whose label wraps.
- `Toast` and `Toaster`, after sokkel-ui's toast: kinds by icon, the newest in front and two
  peeking, apart while pointed at, a close button and a timeout that stops while pointed at.
  Story: Primitives/Overlays/Toast.
- In the maps add-on, `MapPopup` and `MapPopupRow`, after sokkel-ui-maps' `MapPopup`, and a
  `MapView` whose markers are chosen by pressing them (`Marker::id`, `MapView::selected`).

## Next

- Toasts do not slide in and out yet, and are not swiped away.
- The list's Holds column is narrower than the web's.
- A parity board against the web's screens, as tasks and status have.
