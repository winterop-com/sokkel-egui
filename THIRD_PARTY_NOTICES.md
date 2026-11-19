# Third-party notices

- IBM Plex Sans (variable, Latin and Latin Extended subsets as `@fontsource-variable/ibm-plex-sans`
  ships them, converted from WOFF2 to TTF), IBM Plex Mono Regular, and IBM Plex Mono Medium and SemiBold
  (Latin, as `@fontsource/ibm-plex-mono` 5.3.0 ships its 500 and 600, converted from WOFF2 to TTF), Copyright 2019 IBM Corp.,
  under the SIL Open Font License 1.1: `crates/core/assets/fonts/IBMPlexSans-OFL.txt` and
  `IBMPlexMono-OFL.txt`.
- Noto Emoji (monochrome), Copyright 2013 Google LLC, under the SIL Open Font License 1.1, and the
  emoji icon font (MIT), through egui's `monochrome_emoji_fonts` (`epaint_default_fonts`): the
  emoji a reaction is drawn with, behind the text faces.
- Lucide icons, through the `lucide-icons` crate and the icon font it ships (MIT and ISC,
  Copyright Lucide Contributors and Cole Bemis), the same set sokkel-ui draws with lucide-react.
- The basemap previews in `crates/maps/assets/previews/`, as sokkel-ui-maps ships them: the light
  and dark street maps from OpenFreeMap (© OpenMapTiles, © OpenStreetMap contributors, under the
  Open Database License), and the satellite from NASA's Blue Marble (public domain).
- The regions in `examples/records/data/regions.json`: Liberia's counties from Natural Earth's
  1:10m Admin 1 (v5.1.2), public domain, as sokkel-ui's examples carry them. The sites in
  `places.json` are generated.
- d3-force 3.0.1 and d3-quadtree 3.0.2, Copyright 2010-2026 Mike Bostock, under the ISC license:
  ported to Rust in `crates/graph/src/force.rs` (the simulation, its link, many-body, collide and
  centering forces, its seeded random source, and the quadtree they search), the force layout
  sokkel-ui-graph's `NetworkGraph` runs on.
- The JSON Schema 2020-12 meta-schema in `crates/core/src/widgets/json_schema_2020_12.json`, as
  sokkel-ui's `json-schema.ts` flattens it (its seven vocabulary meta-schemas merged into one
  object, every reference inside it), Copyright (c) 2022 JSON Schema Specification Authors, under
  the BSD 3-Clause License.
