# Records: sokkel-ui's records example in egui

sokkel-ui's records example (Tidevann's survey records: forms filled in sections and filed to a
simulated server, the responses it holds, the households it follows, the places it covers, the
API and the metadata's health), ported one to one as `examples/records`. Its manifest is
`parity/records.json`; the web's screens are shot into `parity/records/web/` by
`bun scripts/parity.ts web records`.

## Where it stands

- The manifest and the web's screens, shot at sokkel-ui `8eb137fa`: the overview, Forms, three
  forms being filled and a form's layout, Responses, Households and a household, Places, Server,
  Metadata health and Settings. 48 of its 80 parts have an egui counterpart from the earlier ports.
- The data is the web's to the digit (`forms.rs`, `geo.rs`, `backend.rs`, checked in their tests):
  the seven forms with their sections, the monthly grid's eight cells banded by time of day, the
  answers made up from a seed with JavaScript's generator and rounding, the receipts' ids as the
  web's `receiptId` writes them, the households and responses seeded around the day, and the
  server's checks, filing, sending on and withdrawing. The reporting periods went into the library
  (`sokkel_egui::periods`, after `lib/periods`).
- The frame and the overview: the rail with the waiting count, the trail with `juniper` in the code
  face, the server menu (`ServerMenu`, new in the library: the state's dot, the name, the version,
  and a menu of the state, the server's facts, Check again and the app's items), and the status
  line each page fills. The overview: Send on, the responses by state as tiles of equal height
  (`StatTiles`) with the waiting one larger and edged, the forms as cards (`ItemCard`) in the
  person's order, dragged into a new one (`SortableGrid`), and the server's facts in two columns
  (`DescriptionList::columns`). Flat sections (`Section::flat`) and kind chips (`ChipTone::Kind`)
  are new in the library too; a page header's description now wraps at the web's `max-w-prose`.
- Forms: the periodic reports and single events as cards with their kind, questions and period,
  and the program as a list of its forms, each a name over what it records (`Cell::Lines` in rows
  of `row_height`) and a chevron at the row's end (`DataTable::chevron`); the request behind the
  page as the web's `ApiLink` (new in the library: the method in its ink, the path, and a copy
  button).
- A form being filled, for the sectioned forms: Filed for (the place from the hierarchy in a
  `TreeSelect`, the visit's day, the household, the identifier the server gives), each section's
  questions lined up across their columns as the web's `Columns fields` lines them (a `Field`'s
  label, description, control and help each one height across the row), and the bar at the foot
  with Clear, Fill with test data and its seed, the count of answers, the errors (`IssueCount`)
  and Submit, which files to the backend and opens the receipt. New in the library for it:
  `Field::required`, `hint` and `error` (an issue mark at the control's end), `NumberField` over
  `Field` with an optional value, `ComputedField`, `YesNoField`, `TimeField`, `RepeatedGroup`,
  `PeriodPicker`, `TreeSelect`, `Select::codes`, a read-only `Input`, and `FormBar::show_with`.
  The monthly report's count grid is the web's: questions in rows or in columns, a search and
  Unfilled only over a long run, the rows in groups that fold with their count, and each row's cells
  as fields named by their band and cut where the columns do not fit (`EntryGrid`, new in the
  library; a banded table of number inputs where they do). An input is now 32 tall with its edge,
  as the web's, in every app.
- A form's Layout: its sections and questions in a tree dragged into the order every fill follows
  and kept on the server (`Tree::movable`, `Tree::choosing`, `move_tree_node`, new in the
  library), Reset back to the form's own order, and the refusal for a grid of counts. Open: the
  web also moves the focused row with Alt and the arrows.
- Responses: the states as a segmented choice with their counts and dots, the list (search, the
  Form filter as the web's `FilterChip`, each row's menu to open the page, copy the receipt, send
  it on or withdraw it), and the response chosen open in a sheet beside the list (`DetailSheet`,
  new in the library: the code over the title, the chips and actions, the body, the foot, and an
  edge that drags it wider). A response's own page and the sheet share its body: received, filed
  for (with links as the web's `TextLink`), its answers (a monthly report's as a read-only
  `EntryGrid`), and its history; Withdraw asks first. A table asked to show all its rows now does
  so inside a page that scrolls. Open: the web's Raw record opens the record in a `SourceSheet`,
  a code view, which waits for the code editor.
- Households: the list searched by identifier (`DataTable::search_placeholder`) and filtered by
  region, water source and state, each row opening its household; a household's page with the
  banner over a second registration of its identifier, its identifier, what it told the survey,
  its program, and its visits, each unfolding to its answers (`Unfoldable`, new in the library),
  Record a visit and Exit interview opening those forms for it, and Remove after asking. A page
  header's meta words now stand 12 apart, as the web's `gap-x-3`, or 6 where the app gives them as
  one row (`PageHeader::meta_row`, as status' incident and analytics' customer do). Open: Raw
  record waits for the code editor, and the banner names the other household without linking it.
- Places: the hierarchy in a column of its own (`SubNav`, new in the library) with its search and
  tree, the map with no basemap (`MapView::bare`) of the regions' boundaries (`Area`, cut into
  triangles once and filled and edged by its `Emphasis`, chosen by a press, new in the maps crate)
  and the sites as points, the chosen region lit with its sites, the notes under the map, and the
  place's facts in an `Inspector` (new in the library): its code to copy, its path, the forms
  reported there, and what was recorded there. Open: the web folds the map's legend into a
  button.
- Server: what the server is, its kinds of record and its declared operations, in lists whose rows
  grow to their wrapped words (`DataTable::auto_rows`) and open into the search parameters a kind
  reads (`DataTable::detail`, the row's chevron and a press opening it, its state's `expanded`),
  with codes in cells at the body's size (`Cell::Code`, `Cell::CodeLines`), outline tags
  (`Cell::OutlineTags`) and terms in the code face (`DescriptionList::mono_terms`), all new in the
  library. Its web screen predates the column heads of contract 0.30.1, so the heads differ.
- Open: the Playground waits for the code editor's spec, as `ApiPlayground` is built on it.
