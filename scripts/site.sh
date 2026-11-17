#!/usr/bin/env bash
# The project's site, as GitHub Pages serves it: the crates' API docs (from `cargo doc`, run
# first) under api/, and a gallery of every screen the tests record, by example, under shots/.
#
#   scripts/site.sh <out>
set -euo pipefail

out="${1:?usage: scripts/site.sh <out>}"
root="$(cd "$(dirname "$0")/.." && pwd)"
rm -rf "$out"
mkdir -p "$out/shots"
cp -R "$root/target/doc" "$out/api"

section() {
    local name="$1" dir="$2" note="$3"
    local pictures=("$dir"/*.png)
    [ -e "${pictures[0]}" ] || return 0
    mkdir -p "$out/shots/$name"
    cp "$dir"/*.png "$out/shots/$name/"
    printf '<section id="%s"><h2>%s</h2><p>%s</p><div class="grid">\n' "$name" "$name" "$note"
    for picture in "${pictures[@]}"; do
        local file
        file="$(basename "$picture")"
        printf '<a href="shots/%s/%s"><img loading="lazy" src="shots/%s/%s" alt="%s"><span>%s</span></a>\n' \
            "$name" "$file" "$name" "$file" "${file%.png}" "${file%.png}"
    done
    printf '</div></section>\n'
}

description() {
    sed -n 's/^description = "\(.*\)"$/\1/p' "$1/Cargo.toml" | head -1
}

{
    cat <<'HEAD'
<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>sokkel-egui</title>
<style>
:root { --ground: #f8fafc; --card: #ffffff; --ink: #1d2329; --muted: #5d6874; --rule: #dce2e8; --accent: #006aa0; }
@media (prefers-color-scheme: dark) {
  :root { --ground: #14181c; --card: #1b2026; --ink: #e6eaee; --muted: #9aa5b0; --rule: #2c333a; --accent: #5cb5e6; }
}
* { box-sizing: border-box; }
body { margin: 0; background: var(--ground); color: var(--ink); font: 15px/1.5 system-ui, sans-serif; }
main { max-width: 1200px; margin: 0 auto; padding: 32px 16px 64px; }
h1 { margin: 0 0 4px; font-size: 28px; }
h2 { margin: 40px 0 4px; font-size: 20px; text-transform: capitalize; }
p { margin: 0 0 12px; color: var(--muted); max-width: 72ch; }
a { color: var(--accent); }
nav { display: flex; flex-wrap: wrap; gap: 8px 16px; margin: 16px 0 8px; }
.grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr)); gap: 16px; }
.grid a { display: block; background: var(--card); border: 1px solid var(--rule); border-radius: 8px; overflow: hidden; color: var(--muted); text-decoration: none; font-size: 13px; }
.grid img { display: block; width: 100%; height: 180px; object-fit: cover; object-position: top left; border-bottom: 1px solid var(--rule); }
.grid span { display: block; padding: 6px 10px; overflow-wrap: anywhere; }
</style>
</head>
<body>
<main>
<h1>sokkel-egui</h1>
<p>sokkel-ui's design system drawn with egui: the same tokens and themes, its components as egui
widgets, and its example apps ported one to one. Every screen below is one the tests record.</p>
<nav>
<a href="https://github.com/winterop-com/sokkel-egui">Source</a>
<a href="api/sokkel_egui/index.html">sokkel-egui API</a>
<a href="api/sokkel_egui_charts/index.html">charts API</a>
<a href="api/sokkel_egui_maps/index.html">maps API</a>
<a href="api/sokkel_egui_graph/index.html">graph API</a>
<a href="https://winterop-com.github.io/sokkel-design/">sokkel-design</a>
</nav>
HEAD
    for dir in "$root"/examples/*/; do
        name="$(basename "$dir")"
        section "$name" "$dir/tests/snapshots" "$(description "$dir")"
    done
    for dir in "$root"/crates/*/; do
        name="$(basename "$dir")"
        section "$name" "$dir/tests/snapshots" "$(description "$dir")"
    done
    printf '</main>\n</body>\n</html>\n'
} > "$out/index.html"
