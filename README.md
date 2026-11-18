# truchet

Multi-scale Carlson Truchet tiles for fractal flames.

| Crate | What it is |
| --- | --- |
| `truchet-tiles` | The mosaic geometry. Fireheart's engine links it directly and ships `truchet_ms` as a builtin. |
| `truchet-plugin` | An optional WebAssembly guest that exposes the same variation to a host. |
| `truchet-apophysis` | The same variation as an Apophysis 7X plugin DLL, 32- and 64-bit. See [its README](crates/truchet-apophysis/README.md). |

Documentation: see `docs/` (published to GitHub Pages by CI), or run `scripts/build-docs.sh` and open `_site/index.html`.

## Tiles

Two tile sets, chosen by `style`:

- `style = 0`: Carlson's fifteen bands: arcs, bars, dots, cross, clover, T's, and fillets. Stroke half-width is `1/6`, corner radius is `1/2`. `types` is a bitmask picking the families: `1` dl, `2` dr, `4` bars, `8` dots, `16` cross, `32` clover, `64` T, `128` fillet. `3` is arcs only. `255` is all fifteen.
- `style = 1`: the dot set, 22 tiles with two ink dots on every edge. It joins itself but not the band set, so `types` is ignored. In Rust this is `TYPE_ROUND`.

A cell splits with probability `split`, up to `levels`. Odd levels swap ink and ground. Finer cells paint edge and corner wings so neighboring scales meet cleanly.

`mode`: `0` generates a fresh sample, `1` filters the incoming point, `2` snaps toward nearby ink (`snap` is the radius), `3` samples in the Poincaré disk (`grade` must be greater than 0).

`grade` greater than 0 shrinks tiles toward `y = 0`. The mosaic is the upper half-plane: x wraps, y does not, and `y <= 0` is ground.

`cargo run -p truchet-tiles --example sheet -- square|round` writes a PNG of every tile.

## Build

```sh
cargo test --workspace
cargo build --release --target wasm32-unknown-unknown -p truchet-plugin

# Apophysis DLLs (needs Docker; builds with MinGW and writes dist/apophysis/)
scripts/build-apophysis-dll.sh
scripts/smoke-apophysis-dll.sh
```

## CI/CD

- **CI** (`.github/workflows/ci.yml`) runs on every push and pull request: `rustfmt`, `clippy`, the tests, the wasm build, and, on Windows runners, an MSVC build of the Apophysis DLL for x86 and x64 that is loaded with `LoadLibrary` and driven through a session.
- **Release** (`.github/workflows/release.yml`) runs when a `v*` tag is pushed. It reruns CI and publishes a GitHub release with `truchet_ms-apophysis-<tag>-x64.zip`, `…-x86.zip`, the wasm guest, and checksums.
- **Pages** (`.github/workflows/pages.yml`) publishes `docs/`, generated tile sheets and the rustdoc API reference on every push to `main`. Set Settings > Pages > Source to "GitHub Actions" once.

To cut a release:

```sh
git tag v0.1.0
git push origin v0.1.0
```
