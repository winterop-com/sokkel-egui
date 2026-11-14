# Status: sokkel-ui's status example in egui

sokkel-ui's status example (a live board of services, regions and incidents, on call, and the
public status page) is the next to port one to one, after tasks. Nothing is ported yet: this page
holds what the port starts from.

- **Manifest:** `parity/status.json`: the example's 70 library parts, 32 with an egui counterpart,
  and eight screens (overview, incidents, services, one service, regions, on call, the public page,
  settings), shot into `parity/status/web/` by `bun scripts/parity.ts web status`.
- **A live example in shots:** the shooter seeds `Math.random` in every page, and for a manifest
  marked `live` it pauses the page's clock and runs the same 5 s of its own time before the shot.
  Incidents, services, one service, regions, on call and the public page come out the same on every
  shot; the overview (and settings, drawn over it) still draws different readings from shot to
  shot, so something there reads `Math.random` outside the clock's timers. Settle that before its
  egui side is compared.
- **Parts to build first** (`bun scripts/parity.ts check status` lists every one): the status
  provider and `lib/status` (states and their tones), `UptimeBar`, `Stat`, `Banner`, the toast,
  `Section`, `StatusChip`, `DescriptionList`, `Timeline`, `RelativeTime`, `ListRow`, `SplitPane`,
  the dashboard (`DashboardGrid`, `DashboardArrange`, `DashboardEdit`) and its `TimeSeriesPanel`
  (charts), the maps add-on's layers, legend and popup, and the graph add-on (`GraphCanvas`).
