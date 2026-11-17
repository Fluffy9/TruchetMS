# truchet

Multi-scale Truchet variation for Fireheart. This is not the old `Truchet` arc plugin.

Tiles are Carlson corner bands: radius `1/2`, stroke half-width `1/6`, centred on the cell. A cell splits with probability `split`, up to `levels`. That chance does not depend on where the cell sits. Warp the space with another variation (a log, for example) if the detail should grade across the picture. Odd levels swap ink and ground. Samples on the ground are dropped.

`types` is a bitmask: `1` diagonal `\`, `2` diagonal `/`, `3` both.

## Parameters

| Suffix | Default | |
|---|---|---|
| `seed` | 1 | |
| `width`, `height` | 6 | mosaic size, in level-0 tiles |
| `levels` | 4 | deepest a cell may go |
| `split` | 0.5 | chance a cell splits, anywhere in the mosaic |
| `types` | 3 | tile bitmask |
| `noise` | 0.45 | nudges the split roll. `0` is a hard chance |
| `scale` | 1 | multiplies the incoming point before it hits the mosaic |

The pre-affine is already applied. `scale` 1 means one mosaic spans `width` by `height` units of that space.

## Build

```bash
cargo test
cargo build --release --target wasm32-unknown-unknown -p truchet-plugin
```

The guest is `target/wasm32-unknown-unknown/release/truchet_plugin.wasm`. Import that module in Fireheart. The variation name is `truchet_ms`.
