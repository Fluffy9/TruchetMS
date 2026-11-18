# truchet-apophysis

`truchet_ms` as an Apophysis 7X plugin DLL.

## Install

1. Download `truchet_ms.dll` from a release: the `x64` zip for 64-bit Apophysis, the `x86` zip for 32-bit.
2. Copy it into Apophysis's `Plugins` folder and restart Apophysis.
3. Use the `truchet_ms` variation.

Needs Windows 10 or newer. The DLL has no other dependencies (the C runtime is linked in).

## Variables

The variables are named like the flame attributes, so a flame written by Fireheart loads unchanged.

| Variable | Default | Range | |
| --- | --- | --- | --- |
| `truchet_ms_seed` | 1 | 0 – 2147483647 | integer |
| `truchet_ms_width` | 6 | 1 – 64 | |
| `truchet_ms_height` | 6 | 1 – 64 | |
| `truchet_ms_levels` | 4 | 0 – 8 | integer |
| `truchet_ms_split` | 0.5 | 0 – 1 | |
| `truchet_ms_types` | 3 | 0 – 255 | integer bitmask, square set |
| `truchet_ms_style` | 0 | 0 – 1 | integer: `0` square tiles, `1` rounded tiles |
| `truchet_ms_noise` | 0.45 | 0 – 10 | |
| `truchet_ms_scale` | 1 | 1e-6 – 1e6 | |
| `truchet_ms_mode` | 0 | 0 – 3 | integer: generate, filter, snap, disk |
| `truchet_ms_snap` | 0.4 | 0 – 100 | |
| `truchet_ms_grade` | 0 | 0 – 64 | |

See the [docs](../../docs/index.html) for what each one does.

## Use mode 0

Apophysis lets a variation add to the output point but not discard it. In filter and snap modes a point that misses the ink therefore adds nothing, which leaves a hole where the host expected a sample. Generate mode (`truchet_ms_mode = 0`) always produces a point on the ink and is the mode to use. Disk mode (`3`) needs `truchet_ms_grade` above 0 and is meant to be followed by a hyperbolic variation.

## Build

On Windows, with the MSVC toolchain:

```sh
cargo build --release -p truchet-apophysis --target x86_64-pc-windows-msvc
cargo build --release -p truchet-apophysis --target i686-pc-windows-msvc
```

From Linux or macOS, with Docker (MinGW cross-compile, writes `dist/apophysis/{x64,x86}/truchet_ms.dll`):

```sh
scripts/build-apophysis-dll.sh
scripts/smoke-apophysis-dll.sh   # loads both DLLs under Wine
```

## Test

`cargo test -p truchet-apophysis` exercises the exported ABI through the same functions the host calls. `smoke/smoke.c` loads a built DLL with `LoadLibrary`/`GetProcAddress` and drives it through a session; CI runs it for both architectures.

## ABI notes

The DLL exports the cdecl `PluginVar*` functions from the Apophysis plugin SDK. The host calls `Create`, sets variables by name, calls `Init`, `Init3D` or `InitDC` with pointers to its point and transformed point, then `Prepare`, then `Calc` once per iteration. `Calc` reads the transformed point and adds its result to the output point. Integer variables are rounded to the nearest integer and clamped to their range.
