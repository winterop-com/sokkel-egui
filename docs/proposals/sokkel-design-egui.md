# sokkel-design: the egui side

Research notes for the shared design system, `sokkel-design`, from building sokkel-ui-egui. They
answer the questions put by the sokkel-ui side. Nothing here changes either repository. The owner
decides.

## (a) What egui needs first, in order toward an app that follows the shell

The goal is an egui app built the way a web, Tauri or Electron app on sokkel-ui is built. That makes
the frame the first need, then the parts every page of that frame uses.

| Order | Part | Why it comes at this point |
|---|---|---|
| 1 | Shell, Rail (groups, collapse, resize), TopBar (trail, search, actions), StatusLine foot | The frame every page sits in. The storybook's own sidebar and toolbar are a first draft of Rail and TopBar. |
| 2 | PageHeader, Section, page states (empty, loading, error) | What every page starts with. |
| 3 | DataTable, list variant first | One table for every listing. Needs resizable columns, sort, select, a row height from `density.row`, and drawing only the visible rows. It is the largest single part. |
| 4 | Select, Dialog with SettingsDialog, DropdownMenu, Tooltip, CommandPalette | Overlays. egui's `Popup` and `Modal` exist, but focus and keyboard behavior need care. |
| 5 | The rest of the field family | Number, date and select fields. Input, TextField, Checkbox, Switch, Segmented and Tabs exist already. |
| 6 | Toast, Callout and Banner | Short messages. |
| 7 | Add-ons | Charts and maps have started; they grow as the patterns ask for them. |

## (b) Tokens that do not map to egui, or are missing

The resolver (`scripts/resolve-tokens.ts`, run by `make tokens`) settles every token but these. All
3910 colors it resolves match Chromium within 2/255 (`make verify-colors`).

| Token or value | Status for egui |
|---|---|
| `density.page-inset` | A `cqi` clamp, so CSS-only. egui has its wide and narrow ends and picks one by the available width. |
| `scrollbar` | CSS-only. |
| Parts tokens set to `currentcolor` (`section-head-ink`, `page-title-ink` in some themes) | No value to resolve. egui falls back to the color the part drew before. The spec should name that fallback per part. |
| `elevation-*` | Box-shadow strings with several layers. egui's `Shadow` is one layer, so the strings pass through unparsed. A structured value (x, y, blur, spread, color) per layer would be better. |
| `font-size-adjust` (mono by x-height) | Computed as a scale factor in egui, so it works, but only as derived arithmetic. |
| 154 of the 1119 distinct oklch values | Outside sRGB (mostly code colors in contrast and neon). They are clipped for egui, while a P3 screen shows the web more vivid. |

Missing as tokens, so each renderer guesses:

- **Control heights per size.** Button xs/sm/default/lg are 24/28/32/36 and Input is 32, fixed in the components. `density.control` exists, but Button and Input do not read it.
- **Focus ring.** Outline 2px in `ring` with a 2px offset; the invalid ring is 3px at 20 or 40 percent.
- **Disabled opacity** (0.5).
- **Motion durations and easing.** There are none; a renderer can use none of them.
- **Z-order** of overlays.
- **The spacing steps** components use (Tailwind's 0.25rem scale: `px-2.5`, `gap-1.5`). They are not in `tokens.json`.

The arbitrary hover mixes in the components, such as a secondary button's hover at
`color-mix(in oklch, secondary, foreground 5%)`, live in the components, not the tokens.

## (c) A spec format Rust can read

JSON per part, plus an index and a JSON Schema. serde reads it into typed structs, or a build step
turns it into Rust.

- **Refer to tokens by the resolved names** (`primary`, `radius-control`, `density.control`), never by CSS.
- **Give sizes as a token name or as px**, never as a class.
- **Write states as a map:** `state → { ground, edge, ink, ring }`, each a token name with an optional alpha, such as `"primary/80"`.
- **Write variants as the same shape keyed by name.**
- **Write words as `key → English default`**, the same keys on both sides.

One file per part keeps a change to one part to one diff. The index lists the parts and the
version.

## (d) Where it lives

A separate `sokkel-design` is the right home: tokens, resolver, principles, part specs and the
specimen list sit above both renderers. Each renderer generates from it and has its own drift check.

- **The resolver** (TypeScript today) can move there and publish `tokens.resolved.json` beside `tokens.json`.
- **egui would depend on it** as a crate `sokkel-design` that embeds the resolved file and the specs, or reads the published npm file.
- **The cost:** sokkel-ui is changing fast, and a token change becomes a change in two places, plus a version bump. Releasing sokkel-design and sokkel-ui at one version keeps that bearable.

## (e) What does not fit immediate-mode egui as written

1. **Styling that depends on context** (`:has()`, `in-data-[slot=button-group]`, a card with a head bar) is implicit in CSS. In egui every one must be an explicit option, so the spec should list these contexts as options of the part.
2. **egui wraps a row only before a child whose size it knows.** A part that grows to its content cannot wrap in a flowing row. Specs should give fixed sizes where a part sits in such a row (tiles, chips, badges).
3. **egui hands a child its parent's layout.** Every part that stacks sets its own layout. The renderer handles this; the spec need not mention it.
4. **Transitions and `animate-pulse`** become explicit animation in egui. Without motion tokens the durations are guesses.
5. **Focus.** CSS draws the ring on `:focus-visible` (keyboard only). egui draws it on any focus unless the renderer tracks how focus arrived.
6. **Responsive breakpoints** (`max-md:min-h-finger`) work in egui from the available width. The spec should state the breakpoints as values (768 and the finger size 44).
7. **Fonts apply one frame after they are set.** An egui-only constraint, already handled.

Density, the radius ladder and the parts tokens fit well once resolved. They are plain numbers and
colors per theme, scheme and density.

## Specimens

egui already renders one: `examples/storybook/tests/snapshots/`, every story in tidevann light, the
foundations in tidevann dark, the sample in steel, and every theme's colors in both schemes. A fixed
specimen list from sokkel-design maps onto those tests one to one, so the side-by-side comparison
needs only the Storybook side.
