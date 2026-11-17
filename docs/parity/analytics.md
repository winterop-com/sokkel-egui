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

## Next

- The frame (the rail with its pages, the top bar's trail and the stream's pause, the status line
  of MRR and live changes), the live stream, and the overview's arranged board: the key figure
  tiles, revenue over time, the sign-up funnel, revenue by country on the map, the live feed, top
  customers and revenue by region.
- Then Revenue (breakdowns and invoices with saved views), Customers and a customer, Funnels,
  Reports, Goals, Settings and the palette.
