#!/usr/bin/env bash
# Build the example apps in release mode and package each as a standalone app in dist/apps/: a
# macOS .app bundle on macOS, a plain binary elsewhere. Each .app gets an icon
# (Contents/Resources/AppIcon.icns) from scripts/icon.py: the app's initials on a rounded square in
# the base theme's primary color (needs uv for Pillow), and an ad-hoc signature so Gatekeeper
# takes it for a local build.
# Usage: scripts/bundle.sh [app ...]   (every app in examples/ when none is named)
set -euo pipefail
cd "$(dirname "$0")/.."

if [ "$#" -gt 0 ]; then
  names=("$@")
else
  names=()
  for d in examples/*/; do names+=("$(basename "$d")"); done
fi

args=()
for n in "${names[@]}"; do args+=(-p "$n"); done
cargo build --release "${args[@]}"

version=$(awk -F'"' '/^version/ { print $2; exit }' Cargo.toml)
mkdir -p dist/apps
for n in "${names[@]}"; do
  bin="target/release/$n"
  if [ "$(uname)" != "Darwin" ]; then
    cp "$bin" "dist/apps/$n"; echo "dist/apps/$n"; continue
  fi
  # "tasks" -> "Tasks"
  title="$(echo "${n:0:1}" | tr '[:lower:]' '[:upper:]')${n:1}"
  app="dist/apps/$title.app"
  rm -rf "$app"
  mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
  cp "$bin" "$app/Contents/MacOS/$n"
  icon=""
  if command -v uv >/dev/null && uv run -q scripts/icon.py "$title" "$app/Contents/Resources/AppIcon.icns"; then
    icon="<key>CFBundleIconFile</key><string>AppIcon</string>"
  else
    echo "warning: no icon for $n (uv missing or scripts/icon.py failed)" >&2
  fi
  cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>$title</string>
  <key>CFBundleDisplayName</key><string>$title</string>
  <key>CFBundleIdentifier</key><string>dev.sokkel-egui.$n</string>
  <key>CFBundleExecutable</key><string>$n</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleVersion</key><string>$version</string>
  <key>CFBundleShortVersionString</key><string>$version</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
  $icon
</dict>
</plist>
PLIST
  plutil -lint -s "$app/Contents/Info.plist" || { echo "error: bad Info.plist for $n" >&2; exit 1; }
  codesign --force --sign - "$app" >/dev/null 2>&1 || true
  echo "$app"
done
