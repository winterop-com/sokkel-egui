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
- Open: the Playground waits for the code editor's spec, as `ApiPlayground` is built on it.
