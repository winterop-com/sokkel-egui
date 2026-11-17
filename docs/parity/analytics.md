# Analytics: sokkel-ui's analytics example in egui

sokkel-ui's analytics example (Tidevann's live sales dashboard: revenue, customers, funnels,
reports and goals over a simulated history), to be ported one to one as `examples/analytics`. Its
manifest is `parity/analytics.json`; the web's screens are shot into `parity/analytics/web/` by
`bun scripts/parity.ts web analytics`.

## Where it stands

- The manifest and the web's screens, shot at sokkel-ui `e1777df2`: the overview, Revenue and its
  invoices, Customers and a customer, Funnels and a funnel, Reports and a report, Goals, and
  Settings. 54 of its 64 parts have an egui counterpart from the earlier ports. Still without one,
  each to go into the library as the port reaches it: `ChartPanel` (the funnel, donut, waterfall
  and cohort heatmap kinds), `FilterBar` and `lib/filters`, `SavedViews` and `lib/list-views`,
  `DateField`, the native select, `NotFound`, `ErrorBoundary` and `lib/download`.
- The simulated world is the web's to the last digit (`seed.rs`, `world.rs`, `metrics.rs`,
  `view.rs`, checked in `tests/figures.rs` against values bun reads from the web's own files at
  2026-11-18 09:00 UTC): two years of 13,665 teams, their 21,363 invoices and the latest changes;
  the key figures of any range and segment with their trends, the flows, the funnels, revenue by
  plan, country and region, the top customers, the cohorts and a team's usage. Its random source,
  hash and rounding are JavaScript's bit for bit, and its days are read at the zone the app is
  given, as the browser reads its own.
- The frame and the overview: the rail with each page's description wrapped as the web's
  `text-pretty` wraps it (a last line of one word takes the word before it), the trail, the
  stream's pause, and the status line of MRR and live changes with the live dot. The live stream
  is the web's to the moment (`stream.rs`, `tests/stream.rs`): its random source seeded from when
  the app starts, each event's wait starting once the backend has kept the one before, the ids
  written in base 36, and the notes a big order or a team leaving leave in the corner. The
  overview's arranged board, drawn at 09:00:05 as the web's is shot: the six key figure tiles
  (`StatPanel::format`, `info`, `note`), revenue over time stacked by plan with the period before
  dashed (`Series::stack`, `dashed`, a chart's `format` writing its ticks short as the web's
  `compactWritten`), the sign-up funnel (`FunnelPanel`), the live feed, top customers
  (`Cell::Toned`) and revenue by region, under the bar of plan, region, Compare
  (`variable_toggle`, the small `Switch`) and the range. Their figures are the web's: $554.8K
  +7.2%, $577.2K +5.6%, 6,063 +6.1%, 811 +7.1%, 11.8% -3.0 pts, 2.0% +0.5 pts.
- Revenue by country on the map: each country a point sized by its revenue's square root (4 to
  20) and colored by its class of a sequential scale from `chart-2` (`Marker::radius`,
  `sequential_colors`, `class_of`, `point_radius`), the larger drawn under the smaller, the
  legend's stepped ramp from Less to More (`MapLegend::ramp`), a country's popup with its change
  and share, and the map framed on the points with the web's 24 of margin, framed again while the
  grid settles its width.
- Open: the web map keeps clear of its legend as it frames the points; the egui map frames them
  without it, so it centers a little higher.
- Open: the web example's styles never emit `min-h-28`, so its stat tiles stand at their content's
  94 rather than the library's 112, which the egui tile keeps (a question for sokkel-ui).

## Next

- Revenue (breakdowns and invoices with saved views), Customers and a customer, Funnels,
  Reports, Goals, Settings and the palette.
