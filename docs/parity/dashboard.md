# Dashboard: sokkel-ui's dashboard example in egui

sokkel-ui's dashboard example (Tidevann's dashboards: widgets a person arranges, configures and
names), ported one to one as `examples/dashboard` (`make dashboard`). Its screens are drawn by
`examples/dashboard/tests/screens.rs` (overview, operations, team) and
`tests/behavior.rs` (the list of dashboards), and laid beside the web's by
`bun scripts/parity.ts board dashboard` (the manifest is `parity/dashboard.json`; the web's
screens are shot at 1440 by 1500, as egui's, by `bun scripts/parity.ts web dashboard`).

## Comparing a live board

The readings are drawn from a seeded generator (`live.rs`, mulberry32 seeded as the web's), with
the clock starting at Wed 2026-11-18 09:00 UTC. Egui's draws are its own, so judge the screens by
their structure, never by their figures. The web prints times in the browser's zone and egui in
UTC, so the shots are taken in UTC: shot in Oslo, the schedules' times stood an hour apart.

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
- The command palette, from the top bar's Search or ⌘K, as the web's: New dashboard, Rename
  and Duplicate this dashboard, Pause or Follow live data, the rail's places, and the look.
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
- `Column::fill_share` (filling columns share the room as `fr` tracks) and `Cell::Muted`, for
  the list's Holds column.
- `DashboardEditActions::show_with`, the page's other actions before Edit as the web's
  `children`, and `IconButton::outline`, for the head's More.
- `IconButton` is `icon-sm` unless asked otherwise, as the web's (`IconButton::md` for `icon`);
  the top bar's New and Live, the toast's close and the planner's week buttons stand as the web's.
- `Checkbox::field`, as sokkel-ui's `CheckboxField`: the row the label, 8 in and 6 down, the
  box 10 before a label of 20 tall lines, the hover ground; the to-do list's rows stand as the
  web's. Story: Primitives/Inputs/Checkbox.
- A list given `stack_below` stacks below it or once its columns no longer fit, whichever is
  wider, as the web's `minWidth="auto"` (a table keeps its own `stack_below`; status's Deploys
  list now stacks too, where it squeezed its columns); a stacked list draws plain divided rows, not cards; the
  title is the title alone (never a primary value too) and keeps a code's face. A listed row is
  padded 10 with 4 between its parts, as the web's `py-2.5 gap-1`, so its lines stand 24 apart. Recent failures in
  a third-wide panel stacks as the web's.
- `LogView::grow`, as the worker log's `grow` over `h-72` (sokkel-ui #600): 288 alone, as tall as
  its row beside a taller panel, scrolling past that. A `DashboardPanel` stretched to its row
  offers the room to its body (`body_room`), and a part that takes it says so (`took_room`), so the
  panel's own height, which sizes the row, leaves it out.
- `AccountMenu::local`, as the web's `local`: no one signed in, the avatar a person's outline on
  the muted ground.
- `DataTable::framed`, `Gap::Xs`, and a `Checkbox` whose label wraps.
- `Toast` and `Toaster`, after sokkel-ui's toast: kinds by icon, the newest in front and two
  peeking, apart while pointed at, a close button and a timeout that stops while pointed at. They
  move as the web's, over 500 ms on its `cubic-bezier(0.22, 1, 0.36, 1)`: a new one rises from
  150% of its height below, one closed or timed out sinks away, the rest close up and spread;
  dragged down or right past 40 (base-ui's swipe) one goes that way, let go sooner it springs
  back. The close button is `icon-sm` at the content's right edge, as the web's `ToastClose`.
  Story: Primitives/Overlays/Toast.
- In the charts add-on, `Series::color`: a series drawn in its own token, as a web series'
  `color` (Runs in `chart-1`, Failures in `status-failed`), its legend swatch with it
  (`legend_of`); without one a series takes its place in `SERIES_ORDER`.
- In the maps add-on, `MapPopup` and `MapPopupRow`, after sokkel-ui-maps' `MapPopup`, and a
  `MapView` whose markers are chosen by pressing them (`Marker::id`, `MapView::selected`).

## Next

- Shoot the web's screens again once the sokkel-ui pin moves past #600 (`daa906c6`), which fixed
  the web's worker log: it grew to its lines, its `h-72` lost to `flex-1`.
