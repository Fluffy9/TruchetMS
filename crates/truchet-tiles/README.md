# truchet-tiles

Multi-scale Carlson Truchet mosaic used by the `truchet_ms` builtin in fh-engine and by the Apophysis plugin.

Two tile sets. The square set is Carlson's 15 bands, stroke half-width `1/6`, corner radius `1/2`. The rounded set (`TYPE_ROUND`, `style = 1` in flames) is 22 tiles with two ink ports on every edge; it joins itself but not the square set, so setting it replaces the other `types` bits.

Cells split with probability `split`, up to `levels`. Odd levels swap ink and ground. Finer leaves paint edge/corner wings so scales join without square notches.

`types` bitmask: `1` dl, `2` dr, `4` bars, `8` dots, `16` cross, `32` clover, `64` T, `128` fillet. `3` = arcs. `255` = all fifteen. `256` = `TYPE_ROUND`.

`mode`: `0` generate, `1` filter, `2` snap (`snap` radius), `3` Poincare disk (needs `grade` > 0).

`grade` `> 0` shrinks tiles toward `y = 0` (upper half-plane; `y <= 0` is ground).

Examples: `cargo run -p truchet-tiles --example sheet -- square|round` draws every tile; `mosaic`, `gasket`, `diskchain`, `compare` and `tiles` are smaller demos.
