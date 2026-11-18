#!/usr/bin/env bash
# Checks docs/assets/truchet.js against the Rust crate: renders the same
# windows with both and compares every pixel. Needs cargo and node.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$root/target}"
cargo build --quiet --release -p truchet-tiles --example dump
node scripts/check-js.js "$CARGO_TARGET_DIR/release/examples/dump" "$root/docs/assets/truchet.js"
