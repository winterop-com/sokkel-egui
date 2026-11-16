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
  `variable_select`; the panels read at each refresh). Still to come: the threshold dot before a
  table row's name, and the head's Share, Edit and Add panel.
- Then Dashboards, the editor, Alerts and Settings, each in the rail once it works.
