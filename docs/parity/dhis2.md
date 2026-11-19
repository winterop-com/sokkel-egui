# DHIS2: sokkel-ui's dhis2 example in egui

sokkel-ui's dhis2 example (Tidevann's metadata workbench: indicators, validation rules and program
rules written in DHIS2's expression languages and checked against the server version they run on,
the data elements, the organization units and the functions each version knows, on a simulated
server), ported one to one as `examples/dhis2`. Its manifest is `parity/dhis2.json`; the web's
screens are shot into `parity/dhis2/web/` by `bun scripts/parity.ts web dhis2`.

## Where it stands

- The data is the web's to the value (`metadata.rs`, `values.rs`, `versions.rs`, `expression.rs`,
  `check.rs`, `program.rs`, `backend.rs`, and each page's words and logic in `ui.rs`, `app.rs` and
  `pages/`, checked in `tests/web.rs` against the web's own modules run under bun). The function and
  program variable names the latest server knows now come from the dhis2 add-on
  (`sokkel_egui_dhis2::names`), as the web's `versions.ts` reads them from
  `@winterop-com/sokkel-ui-dhis2/names`; `versions.rs` keeps which version added which, and hands
  the add-on a version's names (`Names::plugin_names`). The backend gained `restore`, the rows
  eframe's storage keeps put back on a new visit.
- The frame (`workbench.rs`): the rail with Indicators, Validation rules and Program rules, then
  Metadata (Data elements, Organization units, Functions), each with its description; the top bar's
  trail (`larkspur` in the code face, the place, the indicator or rule opened) with its links, the
  palette's button and the server menu (its state, version, facts, the versions to run as and What
  this version knows); the status line with the page's note (or where the changes stand while one is
  on its way) and the server's version. The address is the web's hash route: `indicators[/<id>]`,
  `validation-rules[/<id>]`, `program-rules[/<id>]`, `data-elements[/<id>]`, `org-units[/<id>]`,
  `functions`, and `settings` over the page; anything else is Not found. The backend's answers come
  on the app's clock, 150 ms after each call; its toasts and the places it goes to are followed
  each frame. The dhis2 add-on's languages are installed with the names the running version knows,
  and again whenever the version changes, so every editor colors with that version's names.
- The pages: the indicators as a searchable table, filtered by type and check, each with its
  formula in words (the joining `over` faint), its check and its value for Norway in the latest
  month, and New; an indicator with its name, type, description and two expressions edited and
  checked as typed beside Try it (a unit in the hierarchy and a month, the three tiles, the year as
  a line, and each unit under it, a row opening it), the save bar while anything is unsaved, and
  Delete (asked first). The validation rules with their importance, the rule in words (the operator
  in the code face), the check and the violations over every site and month, and New; a validation
  rule with its sides, missing value strategies and operator edited, and Run over the sites under a
  unit from one month to another, its tiles and the violations listed. The program's rules in their
  run order in a column (dragged into another order to number their priorities again, New at its
  head), the chosen rule's name, priority, description, condition and actions edited, and the form
  beside them in a split pane, filled in by hand and run through every rule as it changes: fields
  hidden and assigned, warnings and errors on their fields, what the rules did and those that cannot
  run. The data elements with their codes, disaggregation, value type, what reads them and their
  value, one opened in a sheet with its facts, category option combos and readers. The organization
  units in their tree (searched), the unit's facts with its path linked, its reporting rate over the
  year, and its data values for a month. The functions and program variables, filtered by kind and
  by whether the running version knows them.
- An expression in its field (`editor.rs`, the web's `ExpressionField`): the add-on's language
  colors it, the check's findings are underlined and listed on the field's issue mark (each with its
  line when it has several), the words the version knows are offered as it is typed (inside `#{` the
  data elements by name, after one and a dot its category option combos, inside `C{` and `R{` the
  constants and the data set, inside a rule's `#{` and `V{` its variables), a uid is named on hover,
  and an aggregate expression is read back with the names it stands for.
- Settings: Appearance with the carried themes, the scheme and the density; Server with the
  version as a segmented choice; Demo data's reset (asked first); each section's Reset to defaults.
  The palette: the pages, Run as each version, Settings and the look, and from two letters typed the
  indicators, rules, program rules, data elements and units it finds. The rows and the version are
  saved with eframe's storage.
- New in the library: the add-on `crates/dhis2` (`sokkel-egui-dhis2`, the counterpart of
  `@winterop-com/sokkel-ui-dhis2`): the names, the two languages read by one grammar as the web's
  stream parser reads them (`tokens.rs`, held against the web's `languages.test.ts` in
  `tests/languages.rs`) and `dhis2_plugin(names)`. In the core: plugins (`sokkel_egui::plugins`,
  after the web's `lib/plugins`: `register_plugins`, `plugin_language`, `language_for_media_type`),
  `CodeLanguage::Plugin(name)` and the public `CodeTag` and `CodeToken` a plugin's language reads a
  document into; `CodeEditor::complete`, the words offered worked out from the word typed and what
  stands before it; `Field::issues` (the web's `issues`: the worst tone edging the control and the
  issue mark at the field's end, its list opened from it), and `issues` on `YesNoField` and
  `NumberField`; `SaveBar`; `Cell::Runs`, a cell of words in their own inks and faces; `Page::scroll_in`
  and `Page::show_in`, a page in a pane inset as the shell's page is; `PeriodPicker::width`. Fixed:
  a linked trail's code crumb kept its code face; a page header's words wrap clear of its actions,
  and its description no longer runs past a narrow room; a section's description wraps clear of
  its actions. Stories: Dhis2/Expression editor, Primitives/Inputs/Field issues, Patterns/Save bar,
  Patterns/Data table runs.

## Open

- The server menu lists the versions as items under a Server version label, the one running
  checked, where the web opens them in a submenu of radio items: the core's menus have no
  submenus.
- The table of the units under an indicator's unit stays a table where the web's stacks each unit
  as a card of its labeled figures at that width.
- As in every example, a table's first column and its chevron column are not pinned with their
  rules, and the data elements' Value type column keeps its words on one line where the web's
  breaks them.
- The line charts' month labels are spaced by the charts add-on, which may show one month more or
  fewer than ECharts does.
