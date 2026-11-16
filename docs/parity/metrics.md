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

## Next

- The app on its simulated source (`source.ts`, `data.ts`, `query.ts`): the three dashboards with
  the panels the web draws, then the editor, the alerts and the settings.
