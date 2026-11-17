# Plan

sokkel-egui draws sokkel-ui's design system with egui: the same tokens and themes, its components
as egui widgets, and its example apps ported one to one. It began as a question, does egui wear the
design faithfully when both read one source, and the ported apps are the answer so far.

## Where the design system lives

sokkel-ui writes every token once, as CSS: `oklch()` colors, `var()` aliases, `color-mix()` blends,
`calc`/`max`/`clamp`, rem lengths, and themes layered over tidevann's base. A browser settles all of
this; a native renderer cannot read it.

**The contract is sokkel-design.** It resolves sokkel-ui's tokens at a pinned commit into plain
values (`tokens/tokens.resolved.json`: every theme in light and dark, colors as sRGB hex, lengths in
px, text roles, density levels, and the parts' resolved paints), checks its resolved colors against
Chromium's in its CI, and specifies each part as it landed in sokkel-ui, with a per-renderer state
(`renderers.sokkel-egui`: missing, partial or built). Its site is
[winterop-com.github.io/sokkel-design](https://winterop-com.github.io/sokkel-design/).

Here, `make tokens` copies that file into `crates/core/tokens/tokens.resolved.json` and
`make tokens-check` fails when the copy is not sokkel-design's. A token egui cannot read is a
question for sokkel-design, not a resolver of egui's own. The resolver that began in this repository
moved there; `docs/proposals/sokkel-ui-resolved-tokens.patch` is the first form of that idea.

The renderers stay idiomatic and the contract decides, the same shape as sokkel-api: one contract,
an implementation per language, a check between them. sokkel-tui, a terminal renderer, reads the same
contract.

## What was found

**The colors.** Resolved colors match Chromium's to within 1/255 across every theme and scheme. Wide
gamut colors (mostly code-syntax colors in the contrast and neon themes) are clipped to sRGB, as an
sRGB canvas clips them; on a P3 display the web draws those few more vividly.

**Typography maps exactly.** Every text role (size, line height, weight, tracking) is one
`TextRole::rich` call on the same variable IBM Plex Sans the web uses, its weight on the face's
`wght` axis; mono is sized by x-height, as `font-size-adjust` sizes it on the web.

**What egui does not have.**
- Container queries: the page inset's `cqi` clamp is read at its wide and narrow ends.
- CSS transitions: egui animates with `animate_bool`.
- The web's subpixel layout; text lands on whole pixels.
- Glyphs no installed face has: since egui 0.37 a missing glyph is an error, so a sign the text faces
  lack ("⌘") is drawn from Lucide.

**Accessibility.** Since egui 0.37 the tests fail any widget without a name a screen reader can
say, so every part names itself: by its label, its placeholder, or, for an empty cell, its column.

## What is built

| Path | What it holds |
|---|---|
| `crates/core` (`sokkel-egui`) | Tokens, themes and text roles, and the primitives: buttons, chips, fields and inputs, selects, switches, menus, the command palette, dialogs and toasts, tables (sorting, columns, stacking on narrow screens, editing in place, paging, a toolbar and a foot), the filter bar and saved views, the app frame (rail, top bar, status line, page header), dashboards that are arranged by drag, stat tiles, timelines, calendars and more. |
| `crates/charts` (`sokkel-egui-charts`) | Every chart drawn with egui's painter, one engine as in sokkel-ui: lines (stacked, dashed, with bands and thresholds), bars, gauges, bar gauges, heatmaps, funnels, donuts, waterfalls and sparklines, and their dashboard panels. |
| `crates/maps` (`sokkel-egui-maps`) | Maps on walkers with OpenFreeMap's vector street map styled from the tokens: markers sized and colored by a scale, routes, legends, popups and the web's controls. |
| `crates/graph` (`sokkel-egui-graph`) | Graphs of node boxes and edges, laid out in ranks with routed edges. |
| `examples/tasks`, `status`, `dashboard`, `metrics`, `analytics` | sokkel-ui's examples ported one to one, each checked against the web's own screens (`docs/parity`). |
| `examples/runs`, `logs`, `planner`, `storage` | Apps of egui's own, built only from the library's public parts (`docs/apps`). |
| `examples/storybook` | Every part in every theme, grouped as sokkel-ui's Storybook groups them. |

Every screen the tests record is kept as a snapshot and published in the site's gallery,
[winterop-com.github.io/sokkel-egui](https://winterop-com.github.io/sokkel-egui/).

## Next

1. Finish the analytics port: Reports, Goals, Settings and the palette.
2. Port the rest of sokkel-ui's examples: records, runs, notes and studio, then the media apps.
3. The parts sokkel-design specifies that egui lacks: run-timeline, clip-timeline, emoji-picker,
   reaction-bar, composer and message-composer, from their specs.
4. Keep `renderers.sokkel-egui` true: each part's state is reported to sokkel-design as it is built.
5. Box shadows as resolved values; they still come through as words.
