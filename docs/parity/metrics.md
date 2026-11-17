# Metrics: sokkel-ui's metrics example in egui

sokkel-ui's metrics example (live dashboards over a simulated source, arranged by drag, a panel
editor with a live preview, and alerts), to be ported one to one as `examples/metrics`. Its
manifest is `parity/metrics.json`; the web's screens are shot into `parity/metrics/web/` by
`bun scripts/parity.ts web metrics`.

## Where it stands

- The manifest and the web's screens, shot at sokkel-ui `c3ebc9d4`: the three dashboards
  (Services overview, Workers, Jobs), Dashboards, Alerts, a new panel in the editor, and Settings.
- 51 of its 62 parts have an egui counterpart from the earlier ports. Still without one, each to
  go into the library as the port reaches it: the time range picker (`dashboard/TimeRange`), the
  variables bar (`dashboard/VariablesBar`), `NumberField`, `UnsavedChanges`, `RailDrops`, the
  bar gauge (`BarGauge`) and heatmap (`HeatmapPanel`) panels, `Dot`, `WindowedPane`,
  `CopyButton`, and the toggle group.

- The simulated source is the web's to the last digit (`source.rs`, `query.rs`, checked against
  values bun reads from the web's own files): every metric, the incidents, the log lines, and a
  query's series over a range. `cargo run -p metrics` opens on the Services overview at 09:00:05,
  a scrape landing every second; its stat tiles, gauges, time series, bars, table and logs read
  the web's figures (388 req/s +3.7%, 0.28% +5.2%, 154 ms +2.9%, 51 -26.1%, 43%, 65%).

## Next

- Every panel kind is drawn now (the bar gauge and the heatmap are `sokkel-egui-charts::BarGauge` and `HeatmapPanel`); the time range (presets, or a custom span of days on two months, `RangeCalendar`), the
  refresh and the variables stand under the head (`TimeRangePicker`, `RefreshControl`,
  `variable_select`; the panels read at each refresh). A table's rows name each group after a dot in its last value's threshold color
  (`Cell::Marked`). A time series of more than three series has its legend as a table of
  Avg, Max and Current (`TimeSeriesPanel::legend_table`). A line's time axis picks its labels as the web's engine does (each minute here,
  the first and last always). The head edits the title in place and holds the web's More menu (Edit details, Duplicate
  dashboard, Delete dashboard) and Edit, which arranges the panels in the web's dense grid with the
  palette of the catalog's panels, in a draft Done keeps and Cancel throws away
  (`DashboardEditActions`, `DashboardArrange`).
- Each panel has the web's tools in its head (`PanelMenu`): a menu of View (but on a stat tile),
  Inspect data, Duplicate and Delete, and an expand button that opens the panel in its window over
  the page (`PanelWindow`), the panel saying "Shown in a window." in its place. A chart's tools
  show while the pointer is over it, as the web's `ChartFrame` shows them; a table's and a stat
  tile's always. Inspect data lists the panel's steps, newest first, or its log lines, and Download
  CSV writes the web's CSV into the reader's Downloads, numbering a second one as a browser does.
  Deleting asks first unless the board is being arranged, where Cancel brings the panel back.
- A table's sort arrow stands before the name of a column read from its end, and its bar holds the
  web's Columns menu (every column but the first can be hidden); the sort menu is for stacked rows.
- Left out: Share. The web's builds a link to the view in its URL, which a window has none of.
- The panel editor, from the head's Add panel, a panel's Edit, or an empty board's Add panel: the
  web's preview (the time controls in a pane's toolbar, the panel drawn taller with its legend a
  table, the line saying what it reads) beside its options in a `SplitPane` whose panes scroll,
  and a `FormBar` at the page's foot. The options are the web's: the title, the visualization and
  the width (`Segmented` with `fill`, wrapping its options onto even lines), the metric, its
  aggregation, the grouping, the value, the service and project, the unit, decimals, fill, stacking,
  the stat's color mode, a gauge's ends (`NumberField`), the color (`SwatchGroup`), and the
  thresholds (`Select` with `dots`). Leaving with changes asks first (`unsaved_changes`); saving
  brings the panel into view on the board.
- Dashboards, at the rail's head: the dashboards as a list (`TableVariant::List`, the title in
  `Cell::Strong`) dragged into the order the rail lists them, each opened by a press or its menu's
  Open, or deleted after asking; New opens the web's New dashboard dialog, empty or a copy of one's
  panels. Deleting the dashboard shown lands on Dashboards, as the web's does.
- Settings, from the rail's System or the account menu, over the page shown: the dashboards' range,
  auto refresh and time zone, the look, and resetting the demo's data. A default reaches the board
  while the reader has not picked their own range or refresh, as a link naming none follows the
  web's preferences. The zone is the computer's ("This computer" for the web's "This browser"),
  read as the app starts, or UTC, which the foot strip then names.
- Alerts, in the rail with the count firing: the web's five rules, read every second at the newest
  scrape (146 ms, 0.42%, 49, 75% and 332 req/s at 09:00:05, as the web's), and the 69 alerts they
  fired in the six hours before; a rule fires once it has crossed its threshold for its time,
  saying so in a toast, and resolves once back. The rules' list (a `Cell::Switch` turns one on or
  off), what fired (`Cell::Clock`, its clock and how long ago in the short words), the rule dialog
  with its live chart against the threshold, Clear resolved, and the Alerts section of Settings
  (evaluating, the toasts, how long fired alerts are kept).
- Open: Download CSV saves without a dialog asking where (a native save dialog needs a crate such
  as `rfd`); a stat tile in the preview keeps its own height where the web's stands 224.
- Open: the Services overview's request chart labels its axis every 90 seconds on the web where a
  fresh count of ECharts' `interval: 'auto'` gives 80, and the heatmap labels its axis more
  sparsely than the web's.
- Then the search palette (the top bar's Search and its commands).
