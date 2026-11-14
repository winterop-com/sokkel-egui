# Status: sokkel-ui's status example in egui

sokkel-ui's status example (a live board of services, regions and incidents, on call, and the
public status page), ported one to one as `examples/status` (`make status`). Its screens are
drawn by `examples/status/tests/screens.rs` and laid beside the web's by
`bun scripts/parity.ts board status`.

## Comparing a live board

Its readings are random draws, and the web and egui draw from different streams (the web's
overview even differs from shot to shot), so the board can only show layout and look: sizes,
places, spacing, colors. Judge the status screens by their structure, never by their figures,
and never chase a difference made of numbers.

## Where it stands

- The frame lines up with the web's: the rail with each place's description under its label
  (`RailEntry::description`), the Customers group, the top bar's Live button, and the status line
  live with operational, open and on call.
- Each page has its head (title, count or description, actions), the actions level with the
  title block's foot as the page-header spec says.
- The overview's figures stand in a `KpiRow` of `Stat`s, at the web's places; until the live
  readings are ported, requests and the api's latency are the services' resting values and uptime
  reads "-".
- The service tiles stand as the web's: a `Stat` tile each (`Stat::tile`, its dot, unit, note and
  `sparkline`), four to a row in `Columns`, pressed to open the service. The readings come from
  `live.rs`, the web's calm readings ported (a seeded generator, so egui's numbers are its own);
  the notes say "no data over 90 days" until the incident history uptime is read from is ported.
- The incidents are the web's seed (`data::seed_incidents`: INC-41 open, eight resolved) and the
  incidents page has its filter: `Segmented` with each option's count and state dot, the page's
  parts a section's gap apart (`Stack`, `Gap::Section`).
- The incidents table stacks as the web's labeled cards (`StackLayout::Labeled`): the title and
  its menu, a line each for Touches (`Cell::Tags`), Assignee and Lasted, and the facts (code, state
  and severity chips, `relative_time`). Its bar is the web's (`DataTable::search`): the search, reached by "/" (`SearchField::keys`), narrowing by every word, the rows' count, and the sort menu, newest first by Opened (`Cell::Relative`).
- The services table is the web's: labeled cards with a grip each (dragged to reorder, Reset order
  putting the board's back), the description as a primary value over up to two lines
  (`StackPlace::Primary`), the facts (state, p95, errors, uptime), the bar with the count and
  Sort, and the latency sparkline (`Cell::Trend`) for when it does not stack.
- Next, largest first:
  the dashboard under them; the incidents table (a stacked, labeled list under a bar with its search, row count and sort; `StatusChip`, `TagChips`, `RelativeTime`); the map of sites; on call's
  rotation; the public page; settings; the live simulation behind all of them.

## Groundwork

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
