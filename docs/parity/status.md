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
  readings are ported, requests and the api's latency are the services' resting values; uptime is
  the average of the services' ninety days.
- The service tiles stand as the web's: a `Stat` tile each (`Stat::tile`, its dot, unit, note and
  `sparkline`), four to a row in `Columns`, pressed to open the service. The readings come from
  `live.rs`, the web's calm readings ported (a seeded generator, so egui's numbers are its own);
  each note says the service's uptime over ninety days. The tiles and the services table stand in
  the board's own order (`data::SERVICE_IDS`, as the web's) until the person drags the services
  into their own, which the tiles then follow.
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
- On call is the web's: a `Split` with the Now `Section` (each `PersonRow` with its chip, the
  escalation and next week in their status tones, `ChipTone::Status`), the Rotation section with
  its weeks, each week's person chosen in place, and the Handovers `Timeline` beside them. The page
  scrolls as the web's `main` does (`Page::scroll`); the listings that fill the page with a table
  (incidents, services, regions) keep `Page::show`, their tables scrolling inside it.
- A service's page has its head (`PageHeader::back`, the host as its code, `PageHeader::chip` for
  its state, Report incident, Schedule maintenance and Deploy), its figures, and its live charts
  in a `DashboardGrid`: Latency (p95 and p50) and Errors, each a `TimeSeriesPanel` from the charts
  add-on, half the row, the areas fading from each line, a clock time every 15th reading, the
  values written with their unit and the threshold steps dashed where the axis reaches them. Under them the Uptime panel (`DashboardPanel`) holds
  the `UptimeBar`: ninety days from the incident history (`history.rs`, the web's `uptimeDays`:
  each incident's time weighed by its severity, a stray minute lost on one day in twenty), the
  days with incidents in their worst state's color, the readout saying the uptime or the day
  pointed at, and a day pressed (`services/<id>/<date>`, as the web's route) listing its incidents
  in a `Timeline`. The figures the history gives match the web's to the hundredth: 99.91% over
  every service on the overview, 99.92% for api, each tile's note and the services table's
  column. The
  charts engine follows ECharts here for every chart: the series take the web's color order
  (`SERIES_ORDER`, from `chart-2`), a line's points sit on the axis' ends, and the value axis
  steps as ECharts picks (1, 2, 3 or 5 times a power of ten, about five apart). The dependency
  graph, deploys and maintenance follow.
- An incident's page (`incidents/<id>`, a row of the incidents table pressed, or a new incident
  opened) is the web's: the head with its code, back to the incidents, its state and severity
  chips and how long it has been open (`PageHeader` taking several chips and a `meta`), Resolve or
  Reopen and Status page; Post an update (the state after it, the words, Posted as ada, Post
  refused until something is written) over the updates on a `Timeline`, whose titles now wrap as
  the web's do; and beside them the details (a stacked `DescriptionList`: severity, assignee with
  Assign to the person on call, what it touches as `TagChips`, opened, resolved, on the status
  page), each changed in place. Still the web's and not egui's: the title edited in place
  (`InlineEdit`, which also stands the head 8 lower), the code crumb in the code face, and each
  update's author's avatar.
- Under the overview's tiles stands the web's board as it comes, in a `DashboardGrid`: Sites (two
  thirds, the map of the sites in their states with its legend), Open incidents (a third, the
  compact incident table and its All, `DashboardPanel::actions`), and Uptime (the whole row,
  each service's name, what it is and its state over its `UptimeBar`, a day pressed opening that
  day on the service's page). Arranging the board (Edit, `DashboardArrange`) and the other
  panels (latency, requests, errors, on call, maintenance, deploys) are still to come.
- A place chosen inside a page (All, a service's tile, a day) now stays chosen: the shell went
  back to the place the rail started the frame on whenever the page moved elsewhere.
- The public status page is the web's, a page of its own outside the board's frame: an `AppBar`
  (the mark, the page's title, Subscribe and the board's `IconButton`), a `Banner` saying how
  everything is, then in a column 768 wide the incidents going on (a `Section` with its state's
  chip in `Section::actions`, its updates on a `Timeline` with dots in their states' colors), the
  maintenance ahead (`data::seed_maintenance`, the web's windows), every service's ninety days,
  and the last seven days, each day's incidents as `TimelineEvent::content`. Get notified is the
  web's form: the address (a `TextField`, which now fills its column as the web's `w-full`), every
  service or some (`ChoiceList`), the services ticked (`checkbox_list` in four `Columns`), Subscribe
  with its note (`form_bar`), the web's three refusals, and the address said back with Unsubscribe
  and Subscribe another.
- Regions is the web's: the head counting the sites operational, a `ChipBar` of the regions
  (choosing one frames its sites with 6 degrees about them, All frames every site), and a
  `SplitPane` (60% to the map, 35 to 75, the rule dragged or double-clicked back) of the map and
  the sites table. The map (`MapView::framed(false)`, OpenFreeMap's street map in the tokens'
  colors) draws each site in its state's color with the `MapLegend` in its corner (Operational,
  Degraded, Outage, each counted); the table stacks as labeled cards (city, state, country, round
  trip now), its menu showing a site on the map. The sites' round trips are live (`live.rs`, the
  web's `siteHealthOf`). Still to come: the map's locate, full screen and basemap buttons, the
  site's popup, and Simulate trouble here, which waits on the scenarios.
- A busy confirm lets go after a wait (F23 as the owner refined it, in sokkel-design 0.28.0's
  dialog spec): after `busy-release` (5 s) Cancel says "Stop waiting" and, as Escape does, lets
  the work go (`ConfirmAnswer::StopWaiting`); the shade still holds. The palette's own way out
  waits for an egui command that runs long.
- Next, largest first:
  the dashboard under them; the incidents table (a stacked, labeled list under a bar with its search, row count and sort; `StatusChip`, `TagChips`, `RelativeTime`); the map of sites; the public page; settings; the live simulation behind all of them.

## What does not work yet

sokkel-ui's examples keep one rule: every button does what it says. Edit on the overview stands
there so the screen matches the web's, but nothing answers it yet, until the arranged board
(`DashboardArrange`) is ported. Everything else works:

- Report incident (the overview, a service's page) and New (the incidents page) open the new
  incident dialog, the web's `NewIncidentDialog`, opened on its own page.
- Schedule maintenance opens the web's `MaintenanceDialog`: what the work is, and when it starts
  and ends in two `DateTimeField`s (typed as `2026-11-17 02:00`, or picked on a calendar beside
  the day's times every 15 minutes; a moment that cannot be read, or before the earliest, said
  under the field), tomorrow from two in the night by default, Schedule refused in the web's
  words. The window goes on the status page's maintenance ahead.
- Deploy opens the web's `DeployDialog`: the version, one patch on by default, and what it
  changes, refused until the version is written as one and new. A deploy runs 8 s of the
  board's clock, then succeeds, and is the version the service's figures say it runs
  (`data::seed_deploys` gives every service its last three, as the web's). The service page has
  its Deploys panel (each deploy's version, state, what it changed, who and when; Roll back on the
  version running) and its Maintenance panel (Schedule at its head; Start now for two minutes,
  Cancel, End now or Remove by where the window stands), their tables stacking below 420
  (`DataTable::stack_below`, the web's `stackBelow`). A table that fits its room (stacking
  rather than scrolling) shrinks its columns in proportion where they run past it, as the web's
  `minWidth="auto"`; a dashboard's cells measure their panels with unbounded room, so a taller
  table is never cut.
- Resolve, Reopen and Assign to the person on call in an incident's menu and on its page;
  Subscribe on the public page brings the form into view.

sokkel-ui 0.28.1 is met: a tap by touch on a tooltip's trigger (a shut button's reason
included) holds the tooltip for `tip-hold` or until the next press, a mouse press leaving it as
it was; the palette holds while an app's command runs (Stop waiting after `busy-release`) and
says Searching after `wait-shown`; and the parts egui draws read their resolved paints, checked
in `crates/core/tests/paints.rs`.

## Last checked

Against sokkel-ui `4f2cf282` (0.28.0): every web screen shot again, 45 of the example's 70 parts
with an egui counterpart. The rail's untitled first group now stands 4 lower on the web, as egui
already draws it; the Customers group's described entries stand 54 apart on the web and 58 in
egui, a difference older than this check, still to settle.

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
