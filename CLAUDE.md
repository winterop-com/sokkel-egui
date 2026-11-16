# sokkel-egui

sokkel-ui's design system drawn with egui: the same tokens and themes, and primitives after
sokkel-ui's components. Research stage; `docs/plan.md` says what was learned and what comes next.

## Layout

- A cargo workspace shaped like sokkel-ui's `packages/`. `crates/core` is the package
  `sokkel-egui`, the counterpart of `@winterop-com/sokkel-ui`: tokens, themes, text roles and
  primitives. Add-ons come later as crates of their own (`sokkel-egui-charts`, `-maps`, ...).
  `crates/charts` (`sokkel-egui-charts`) is the first add-on: every chart drawn with the
  painter, one engine as in sokkel-ui.
  The same dependency lines hold as in sokkel-ui: an add-on depends on the core by name, the core
  never on an add-on, and no add-on on another. No empty placeholder crates.
- `examples/tasks` is sokkel-ui's tasks example ported one to one: the reference for everything
  else. `make parity` checks it against the web example as GitHub has it and lays both sides'
  screens beside each other (`docs/parity/tasks.md`); a difference is fixed in the library, never
  in the example alone.
- `examples/runs` is a working app on the frame (Tidevann's runs, simulated): it uses only the
  library's public parts; a part it lacks goes into the library.
- `examples/storybook` is its own package that depends on the core by name, as an app would: the
  egui counterpart of sokkel-ui's Storybook, its stories grouped as sokkel-ui groups them
  (Foundations, Themes, Primitives/<group>, Patterns). A story is one `Story` in
  `src/stories/mod.rs`; its kittest snapshots are in `tests/snapshots/`.

## Tokens

- `crates/core/tokens/tokens.resolved.json` is sokkel-design's (`tokens/tokens.resolved.json`, made
  there from the sokkel-ui commit it pins, its resolver and `verify-colors` run in its CI), copied
  in by `make tokens` (`scripts/design.ts`) from `.parity/sokkel-design`, a clone fetched on each
  run (`SOKKEL_DESIGN_REF` pins a commit, `SOKKEL_DESIGN_OFFLINE` skips the fetch). Never edit it.
  `make tokens-check` fails when it is not sokkel-design's.
- After sokkel-design's contract moves, run `make tokens`. A token egui cannot read is a question
  for sokkel-design, not a resolver of egui's own.
- sokkel-ui is read from `.parity/sokkel-ui` (`scripts/sokkel.ts`; `SOKKEL_UI_REF`,
  `SOKKEL_UI_OFFLINE`) by the parity scripts.
- sokkel-ui and sokkel-design are read, never written: they are under heavy development by its own sessions. Questions
  about its tokens or parts go to the sokkel-ui session; a change it might want later is kept as a
  proposal in `docs/proposals/`.
- Every color, corner, size and text style a widget draws comes from `tokens(ctx)`: never a color
  or size literal for something a token names. Text goes through `TextRole`, never a size and a
  weight of its own; weights are the variable face's `wght` axis.
- Tailwind's `bg-x/NN` is `tokens.color_at("x", 0.NN)`.

## Storage

- Nothing here needs a database yet. If something does (a story's data, a cache), use Turso, the
  `turso` crate (`turso = { version = "0.8", default-features = false }`), never rusqlite.

## Rules carried over from sokkel-ui

- Never write the name of an app or product that uses the library. Examples use the fictional
  Tidevann workspace only (jobs nightly-export, sync-inventory, ...; run states; people ada, grace,
  linus, margaret; projects harbor, juniper, larkspur).
- Words a widget says are fields with English defaults. American English. No emojis.
- Fix the widget, never the example.
- Fonts and other carried code are listed in `THIRD_PARTY_NOTICES.md` in the same change.

## Gates

`make all` (fmt-check, clippy with warnings denied, every test). After a visual change run
`make test` first: every snapshot that differs fails and leaves `*.new.png` and `*.diff.png` beside
it. Look at each of those, and only then `make snapshots` to accept them. A story's snapshot draws
its canvas alone (`Storybook::show_story`), so adding a story changes no other picture; only
`storybook_frame.png` shows the sidebar. A new story's first picture is recorded by
`make snapshots` and looked at before it is kept. Commit each finished piece of work
locally on main (the owner asked for one commit per iteration), staging named paths, conventional
commits, no attribution lines. Never push: there is no remote.
