# Dataviz: sokkel-ui's dataviz example in egui

sokkel-ui's dataviz example (Tidevann's Visualizer: datasets imported from a file or pasted, charts
built from their columns with a live preview and views suggested for them, saved charts, boards of
charts filtered together and presented, and charts shared as links that carry their data), ported
one to one as `examples/dataviz`. Its manifest is `parity/dataviz.json`; the web's screens are shot
into `parity/dataviz/web/` by `bun scripts/parity.ts web dataviz`.

## Where it stands

- The data is the web's to the value (`model.rs`, `seed.rs`, `parse.rs`, `shape.rs`, `views.rs`,
  `share.rs`, `backend.rs` and each page's and dialog's words and logic in `visualizer.rs`,
  `chart_view.rs`, `dialogs.rs`, `import_dialog.rs`, `inputs.rs` and `pages/`, checked in
  `tests/web.rs` against the web's own modules run under bun). The figures are shaped with the
  charts add-on's data module (`sokkel_egui_charts::data`: cells, rows, the pivot, the quantile and
  the ways figures combine), which replaced the example's own copy (`chart_data.rs`); the regions
  are `examples/records/data/regions.json`, shared rather than copied. The backend gained
  `restore`, the rows eframe's storage keeps put back on a new visit.
- The frame (`app.rs`): the rail with Datasets, Charts and Boards over their descriptions and
  counts, the boards in the person's order, and Settings; the top bar's trail with its links and
  the palette's button; the status line with where the changes stand and the three counts. The
  address is the web's hash route: `datasets[/<id>]`, `build[/<id>]`, `charts[/<id>]`,
  `boards[/<id>[/present]]`, `shared/<payload>`, and `settings[/<section>]` over the page; anything
  else is a page that leads nowhere. The backend's answers come on the app's clock, 150 ms after
  each call, as the web's simulated latency; its toasts (with Undo and Open) and the places it goes
  to are followed each frame.
- The pages (`screens.rs`): the datasets as a searchable table with each one's columns, size,
  charts, source and age, New chart, Download CSV and Delete (asked first); a dataset with its
  title and description edited in place, More (Download CSV, Duplicate, Delete) and New chart, and
  the Rows, Columns and Charts tabs: rows searched, filtered, selected, edited in place by type,
  added, duplicated and deleted (their menu shown on hover); columns renamed, retyped, added and
  deleted (asked first); the charts drawn from it. The builder: Back, Download (data as CSV), Share
  and Add to board (held until the chart is saved), the live preview of the view in its frame, the
  note under it, the suggested views as cards with their charts drawn small, and the options
  (Chart, View, Columns, Figures, Look) in a split pane; the form bar with Discard, Save as new and
  Save. The saved charts as cards (the default) or a table, sorted by when they were saved. The
  boards in the person's order, dragged by their grips. A board with its title edited in place,
  Present, Edit (a draft kept by Done or thrown away by Cancel, charts dragged on from the
  palette), Add chart and More; its filter bar across every chart whose dataset has the column,
  kept with the board; its panels each drawn as their view with Edit chart, Download CSV, Edit the
  board and Remove from board (with Undo). A shared chart rebuilt from its link, kept here with
  Keep a copy.
- Each view is drawn as the web's `ChartView` (`draw.rs`): bar, line, area, scatter, pie and
  heatmap by the charts add-on's `ChartPanel` from the web's chart spec; the map as the regions
  shaded by a sequential scale with its legend (Less, More, No figure) and the picked region's
  popup; the figures as a table; one figure as a `StatPanel` with its course and change.
- Settings (`overlays.rs`): Charts (the palette with its swatches, how figures combine, suggested
  views), Numbers (format and decimals, with the sample written as set), Appearance with the
  carried themes, and the demo's data reset (asked first); each section's Reset to defaults. The
  dialogs: Import data (a file dropped on the window, or rows pasted, the columns' names and types
  editable before anything is kept), New board and Board details, Add a chart (its width and the
  saved chart), and Share with the link and Copy link. The palette: the page's own commands (This
  chart, or the board's), the places, the actions, every dataset, chart and board, and the look.
  Everything kept is saved with eframe's storage.
- New in the library: in the core, `ItemCard::icon`, `ItemCard::media` and `ItemCard::title_lines`;
  `DataTable::cards` (the rows as a grid of cards, with the bar's table and grid toggle,
  `TableState::grid`, the sort menu in the grid and a row's actions at its card's corner) and
  `DataTable::actions_on_hover`; a table's row actions standing before its chevron rather than over
  it; `Select::dot_rows` (a palette's swatches before each option); `PageHeader::edit_description`
  and the count beside an editable title; a primary button's trailing icon in its own ink. In the
  charts add-on, a value axis up the side of a chart under 140 tall takes two steps rather than
  five, as the web's `EChart` sets `splitNumber`.

## Open

- A table's first column and its end column are not pinned with their rules, as the web's are
  where a table has a least width: the core's `DataTable` draws neither rule, in every example.
- The dataset's description edited in place is in the foreground ink, where the web's sits muted
  under the title: the core's `InlineEdit` has one ink at rest.
- Download offers the data as CSV; the chart as PNG is listed but held, as there is no picture of
  a chart to export. The shared page's PNG button is left out for the same reason.
- Import reads a file dropped on the window rather than one picked from a dialog: the core has no
  file field. Pasted rows work as the web's.
- A board's map stands in the middle of its panel; the web's frames it a little to the right of the
  legend's corner. A board presented full screen (`boards/<id>/present`) shows the board in its page
  rather than in a window over everything.
- The selection bar of the dataset's rows has Clear but not Delete N: the core's selection bar has
  no actions of the app's.
- The rows' heads `Rainfall · number` show whole where the web's narrower number columns cut them
  short.
