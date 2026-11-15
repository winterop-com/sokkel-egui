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
  `children`, and `IconButton::outline` and `IconButton::sm` (`icon-sm`), for the head's More.
- `DataTable::framed`, `Gap::Xs`, and a `Checkbox` whose label wraps.
- `Toast` and `Toaster`, after sokkel-ui's toast: kinds by icon, the newest in front and two
  peeking, apart while pointed at, a close button and a timeout that stops while pointed at.
  Story: Primitives/Overlays/Toast.
- In the charts add-on, `Series::color`: a series drawn in its own token, as a web series'
  `color` (Runs in `chart-1`, Failures in `status-failed`), its legend swatch with it
  (`legend_of`); without one a series takes its place in `SERIES_ORDER`.
- In the maps add-on, `MapPopup` and `MapPopupRow`, after sokkel-ui-maps' `MapPopup`, and a
  `MapView` whose markers are chosen by pressing them (`Marker::id`, `MapView::selected`).

## Next

- Toasts do not slide in and out yet, and are not swiped away.
- What the board shows apart from the figures:
  - The top bar: the web's has the Search field (its palette's trigger, ⌘K) at the right.
  - The to-do list: the web's rows stand 40 apart and wrap at their width; egui's are closer.
  - The worker log fills its panel downward on the web, from its first line; egui shows the
    latest lines at the foot.
  - Recent failures in a third-wide panel stacks as labeled cards on the web.
  - The account's avatar is a person icon on a muted disc on the web, initials in egui.
