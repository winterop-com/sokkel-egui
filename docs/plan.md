# Plan

sokkel-ui-egui draws sokkel-ui's design system with egui: the same tokens and themes, and
primitives built after sokkel-ui's components. This is the first, deliberately small step: does egui
wear the design faithfully when both read one source?

## The question first: where the design system lives

sokkel-ui writes every token once, in `packages/core/tokens/tokens.json`. Its values are CSS:
1739 `oklch()` colors, 281 `var()` aliases, 62 `color-mix()` blends, `calc`/`max`/`clamp`, rem
lengths. A theme is a layering of maps:
- light is tidevann's light, then the derived surfaces, then the theme's own;
- dark adds tidevann's dark and the theme's dark.

A browser settles all of this. A native renderer cannot read it at all.

**Recommended: the tokens stay the one source, a resolver turns them into plain values, every
renderer reads the result.**
- While sokkel-ui is under heavy development, the resolver lives here and only reads it:
  `make tokens` runs `scripts/resolve.ts` over a sokkel-ui checkout's `tokens.json` and writes
  `crates/core/tokens/tokens.resolved.json`. That file holds every theme in light and dark, its
  colors as sRGB hex, its lengths in px, its numbers, words and text roles, and every density level.
  `make tokens-check` fails when the file is not what sokkel-ui's tokens make.
- When sokkel-ui wants it, the resolver moves there and its token build writes the file beside its
  CSS, exported as a package entry: `docs/proposals/sokkel-ui-resolved-tokens.patch` is that change,
  tried once on a branch and set aside.

The two renderers stay idiomatic and the tokens decide. It is the same shape as sokkel-api: one
contract, an implementation per language, and a check between them.

The same resolved data is what a later design system for decks and mockups would be built from,
with the component screenshots.

## What was found

**The resolver.**
- A small evaluator with no dependency handles what the tokens use:
  - oklch, hex, `transparent`, `var()`;
  - `color-mix` in oklab and oklch, with premultiplied alpha;
  - px, rem and em, `calc` and `max`.
- It also emits the color names Tailwind classes paint with that no theme sets directly. For
  example, `good-ink` is `var(--status-succeeded-ink)`.
- Checked against Chromium: all 3910 colors a page computes (13 themes × 2 schemes) come out
  within 1/255 of the browser's own, drawn into an sRGB canvas. `make verify-colors`
  (`scripts/verify-colors.ts`) reruns that comparison from sokkel-ui's own generated CSS.
- `make tokens` and `make tokens-check` fail when a token is left unresolved that is not known to be
  CSS-only (`scrollbar`, the `cqi` page inset): a value sokkel-ui starts writing in a form the
  evaluator does not know fails there, not later as a widget asking for a missing token.

**Wide gamut.**
- 154 of the 1119 distinct oklch values lie visibly outside sRGB (ΔE > 0.01). Most are code-syntax
  colors in the contrast and neon themes.
- The resolver clips them, as an sRGB canvas does. On a P3 display, Storybook draws those few more
  vividly than egui can, so a small difference there is expected, not a bug.

**Typography maps exactly.**
- egui 0.36 has no font weights in `FontId`, but `RichText` takes `.variation("wght", w)`,
  `.line_height()` and `.extra_letter_spacing()`.
- Every text role (size, line height, weight, tracking) is one `TextRole::rich` call on the same
  variable IBM Plex Sans the web uses (converted from the fontsource WOFF2).
- Mono is sized by x-height through `mono-x-height`, as `font-size-adjust` does on the web.

**What egui does not have.**
- Container queries: `density.page-inset` is a `cqi` clamp, so only its wide and narrow ends are
  emitted.
- CSS transitions: egui animates with `animate_bool`.
- The web's subpixel layout.
- Box shadows come through as words (`0 1px 2px #23333e14, ...`), not yet parsed.

**What the tokens lack, for any second renderer.**
- No motion or z-order tokens; components use Tailwind's defaults.
- No single control-size scale: Button and Input use fixed 24/28/32/36, while composites read
  `density.control`. sokkel-ui's roadmap already has "one size scale" for 1.0.
- These sizes live in the components, so here they live in the widgets too.

## What is built

| Path | What it holds |
|---|---|
| `crates/core` (`sokkel-ui-egui`) | `Appearance` (theme, scheme, density), `install(ctx, &appearance)`, `tokens(ctx)`, `TextRole`, and `Button` (6 variants, 4 sizes, busy, a disabling reason), `Chip` (tones, selected), `TextField` (label, help, error), `Switch`. |
| `crates/charts` (`sokkel-ui-egui-charts`) | The first add-on, on the core by name: `LineChart` (series, an area, a shaded band), `BarChart` (side by side or stacked), `Sparkline` and `legend`, drawn with the painter in the `chart-*` tokens, with a guide and a tooltip under the pointer. |
| `crates/maps` (`sokkel-ui-egui-maps`) | The second add-on: `MapView` on walkers 0.60 with OpenStreetMap tiles (an identifying user agent and a tile cache), `Marker` and `Route` in the chart colors, zoom buttons, the OSM credit; without tiles it draws the `canvas` ground and a graticule, which the snapshots show. |
| `examples/storybook` | The storybook: a sidebar of stories grouped as sokkel-ui's are, a toolbar for theme, scheme and density, and the story's canvas. Foundations (Colors, Typography, Corners, Density, Icons, Code colors), Themes (All themes, Sample), Actions/Button, Display (Alert, Badge, Card, Separator, Skeleton), Inputs (Checkbox, Field, Input, Switch), Navigation/Tabs, Patterns (Chip, Segmented, Shell), Charts (Line, Bars, Sparkline), Maps/Map. |
| `examples/storybook/tests/snapshots/` | Every story in tidevann light, the foundations in tidevann dark, the sample in steel; and a test that draws every color of every theme in both schemes. |

**Theme scopes.** `scope(ui, theme, dark, |ui| ...)` draws a part in another theme or scheme, as
sokkel-ui's `data-theme-scope` themes one element: the widgets inside read the scope's tokens and
egui's own visuals follow them. Themes/All themes draws every theme this way, in both schemes.
egui wraps a row only before a child whose size it knows, so each tile is given its size first.

**Icons.** Lucide, as sokkel-ui uses lucide-react, from the `lucide-icons` crate's font in a
family of its own (`icons::paint`, `Button::icon`). 154 of the 159 icons sokkel-ui's packages import
are in the font; the other five are lucide-react aliases (`check-circle-2`, `code-2`, ...). egui
applies fonts a frame after they are set, so `icons::paint` skips the first frame instead of
panicking on a missing family.

**Layout inside parts.** egui hands a child the layout of the row around it, so a part that stacks
(Card, Alert, a theme tile) sets its own top-down layout rather than trusting its parent's. A frame
part a theme leaves as `currentcolor` (`section-head-ink` in tidevann) has no resolved color;
`Tokens::color_or` takes the color the part drew before.

**Seeds from egui-demos.** Two pieces were reshaped to read only tokens:
- the palette-to-`Visuals` mapping (`src/theme.rs`);
- the shadcn-style widget painting (`examples/ui_kit_gallery/widgets.rs`).

**Known differences from the React components:**
- TextField shows its error as a line under the field. sokkel-ui's `Field` puts it behind an
  issue mark.
- Hover on a secondary button blends in gamma space rather than oklch.
- There are no icons yet.

## Next

1. More primitives, chosen by what the frame needs: Select, Section, Tooltip, Dialog, Menu,
   Callout.
2. The frame: Shell, Rail, TopBar and StatusLine are in (`Shell::show` asks one closure for each part, so an app's state is borrowed once); next are PageHeader, page states, the rail's resize and collapse by drag, and a port of the `tasks` example's screens.
- The add-on crates after the core, one engine each as in sokkel-ui:
  - `sokkel-ui-egui-maps`: started (markers, routes, zoom, tiles); next are areas (GeoJSON
    polygons) and a legend.
  - `sokkel-ui-egui-charts`: started (line, bars, sparkline); next are a donut, a heatmap and
    the dashboard's chart panels. sokkel-ui's charts take rows and fields (`x`, `y`, `series`);
    these take categories and series for now.
  A prior egui dashboard on egui 0.36 already has both (see the session memory for where).
3. A renderer-neutral spec per part in sokkel-ui (anatomy, sizes, color roles per variant and state,
   by token name), so both renderers build from it rather than from each other.
4. Side-by-side pages of each snapshot beside its Storybook story, per theme and scheme.
5. Shadows and focus rings as resolved values, and the token additions above proposed in sokkel-ui.
