#!/usr/bin/env bash
# Build the site: the pages in docs/ (landing and reference), the tile sheets
# generated from the crate, and the rustdoc API reference.
#
#   scripts/build-docs.sh [output-dir]      (default: _site)
#   REPO_URL=https://github.com/you/fork scripts/build-docs.sh
#
# CI runs this and publishes the result to GitHub Pages.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
out="${1:-$root/_site}"
repo_url="${REPO_URL:-https://github.com/Fluffy9/TruchetMS}"

cd "$root"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$root/target}"
rm -rf "$out"
mkdir -p "$out"
cp -r docs/. "$out/"
mkdir -p "$out/assets"

cargo run --quiet --release -p truchet-tiles --example sheet -- square > "$out/assets/square-tiles.png"
cargo run --quiet --release -p truchet-tiles --example sheet -- round  > "$out/assets/round-tiles.png"

cargo doc --no-deps -p truchet-tiles
cp -r "$CARGO_TARGET_DIR/doc" "$out/api"
printf '<!doctype html><meta http-equiv="refresh" content="0; url=truchet_tiles/index.html">\n' > "$out/api/index.html"

# Fill in the repository link on every page.
for page in "$out"/*.html; do
  sed "s#__REPO_URL__#${repo_url}#g" "$page" > "$page.tmp"
  mv "$page.tmp" "$page"
done
touch "$out/.nojekyll"

echo "docs written to $out"
