//! Apophysis 7X plugin for the `truchet_ms` variation.
//!
//! Apophysis loads every DLL in its `Plugins` folder and calls a fixed set of
//! `PluginVar*` exports (the interface of `apoplugin.h`). This crate implements
//! that interface in Rust on top of `truchet-tiles`; no C header is needed.
//!
//! How the host drives a plugin:
//!
//! * `PluginVarCreate` makes an instance. The host sets variables by name with
//!   `PluginVarSetVariable`, then calls `PluginVarInit*` (pointers into the
//!   iterator state and the variation weight) and `PluginVarPrepare`.
//! * For every iteration it calls `PluginVarCalc`. The incoming point is
//!   `FT` (after the transform's affine); the plugin **adds** its result to `FP`,
//!   which the host zeroes before the first variation of a transform runs.
//!
//! Apophysis has no way for a variation to drop a sample. A miss in filter or
//! snap mode (and a failed throw in generate or disk mode) therefore adds
//! nothing. Generate mode (`truchet_ms_mode = 0`) is the one that never leaves
//! gaps, and is what you want in most flames.
//!
//! Variable names are the flame attribute names (`truchet_ms_seed`, ...), so
//! flames written by Fireheart load unchanged.

use std::ffi::{c_char, c_int, c_void, CStr};
use std::ptr;
use std::sync::atomic::{AtomicU32, Ordering};

use truchet_tiles::{disk_sample, filter_point, sample_ink, snap_point, Mosaic, TYPE_ROUND};

const TRUE: c_int = 1;
const FALSE: c_int = 0;

/// Values `PluginVarGetArchitecture` reports (`PLUGIN_ARCH_*` in the SDK).
const ARCH_X86: c_int = 0x20;
const ARCH_X64: c_int = 0x40;

/// One plugin variable. Integers are stored as rounded doubles.
pub struct VarInfo {
    pub name: &'static CStr,
    pub integer: bool,
    pub min: f64,
    pub max: f64,
    pub default: f64,
}

const fn real(name: &'static CStr, min: f64, max: f64, default: f64) -> VarInfo {
    VarInfo {
        name,
        integer: false,
        min,
        max,
        default,
    }
}

const fn int(name: &'static CStr, min: f64, max: f64, default: f64) -> VarInfo {
    VarInfo {
        name,
        integer: true,
        min,
        max,
        default,
    }
}

const SEED: usize = 0;
const WIDTH: usize = 1;
const HEIGHT: usize = 2;
const LEVELS: usize = 3;
const SPLIT: usize = 4;
const TYPES: usize = 5;
const STYLE: usize = 6;
const NOISE: usize = 7;
const SCALE: usize = 8;
const MODE: usize = 9;
const SNAP: usize = 10;
const GRADE: usize = 11;

/// The variables, in the order the host sees them.
pub const VARS: [VarInfo; 12] = [
    int(c"truchet_ms_seed", 0.0, 2_147_483_647.0, 1.0),
    real(c"truchet_ms_width", 1.0, 64.0, 6.0),
    real(c"truchet_ms_height", 1.0, 64.0, 6.0),
    int(c"truchet_ms_levels", 0.0, 8.0, 4.0),
    real(c"truchet_ms_split", 0.0, 1.0, 0.5),
    // Bitmask: 1 dl, 2 dr, 4 bars, 8 dots, 16 cross, 32 clover, 64 T, 128 fillet.
    int(c"truchet_ms_types", 0.0, 255.0, 3.0),
    // 0 = Carlson's square set (picked by `types`), 1 = the rounded set.
    int(c"truchet_ms_style", 0.0, 1.0, 0.0),
    real(c"truchet_ms_noise", 0.0, 10.0, 0.45),
    real(c"truchet_ms_scale", 1.0e-6, 1.0e6, 1.0),
    // 0 generate, 1 filter, 2 snap, 3 disk (needs grade > 0).
    int(c"truchet_ms_mode", 0.0, 3.0, 0.0),
    real(c"truchet_ms_snap", 0.0, 100.0, 0.4),
    real(c"truchet_ms_grade", 0.0, 64.0, 0.0),
];

/// Variation name the host registers (the `truchet_ms="1"` weight attribute).
const NAME: &CStr = c"truchet_ms";

static NEXT_STREAM: AtomicU32 = AtomicU32::new(1);

/// xorshift32. Every instance gets its own stream, so the threads of a render
/// do not repeat each other.
struct Rng(u32);

impl Rng {
    const FALLBACK: u32 = 0xA341_316C;

    fn new() -> Self {
        let stream = NEXT_STREAM.fetch_add(1, Ordering::Relaxed);
        let seed = Self::FALLBACK ^ stream.wrapping_mul(0x9E37_79B1);
        Rng(if seed == 0 { Self::FALLBACK } else { seed })
    }

    /// Uniform in `[0, 1)`.
    fn unit(&mut self) -> f32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        if x == 0 {
            x = Self::FALLBACK;
        }
        self.0 = x;
        (x >> 8) as f32 / 16777216.0
    }
}

/// One plugin instance. The host only ever holds it as a `void*`.
pub struct Variation {
    vvar: f64,
    ft: [*mut f64; 3],
    fp: [*mut f64; 3],
    color: *mut f64,
    /// Targets for the pointers a 2D `PluginVarInit` does not supply.
    dummy: [f64; 3],
    values: [f64; VARS.len()],
    mosaic: Mosaic,
    scale: f32,
    mode: i32,
    snap: f32,
    rng: Rng,
}

impl Variation {
    fn new() -> Self {
        let mut values = [0.0; VARS.len()];
        for (slot, info) in values.iter_mut().zip(VARS.iter()) {
            *slot = info.default;
        }
        let mut v = Variation {
            vvar: 0.0,
            ft: [ptr::null_mut(); 3],
            fp: [ptr::null_mut(); 3],
            color: ptr::null_mut(),
            dummy: [0.0; 3],
            values,
            mosaic: Mosaic::default(),
            scale: 1.0,
            mode: 0,
            snap: 0.4,
            rng: Rng::new(),
        };
        v.prepare();
        v
    }

    fn prepare(&mut self) {
        let g = |i: usize| self.values[i];
        let style = g(STYLE) >= 0.5;
        self.mosaic = Mosaic {
            seed: g(SEED).round().max(0.0) as u32,
            width: g(WIDTH) as f32,
            height: g(HEIGHT) as f32,
            levels: g(LEVELS).round() as i32,
            split: g(SPLIT) as f32,
            types: if style {
                TYPE_ROUND
            } else {
                g(TYPES).round().max(0.0) as u32
            },
            noise: g(NOISE) as f32,
            grade: g(GRADE) as f32,
        };
        self.scale = g(SCALE) as f32;
        self.mode = g(MODE).round() as i32;
        self.snap = g(SNAP) as f32;
    }

    /// Point the optional pointers at private storage, as the SDK does with its
    /// dummy globals, so a 2D host never leaves them dangling.
    fn use_dummies(&mut self) {
        self.dummy = [0.0; 3];
        let base = self.dummy.as_mut_ptr();
        // SAFETY: `dummy` has three elements and lives as long as the instance.
        unsafe {
            self.ft[2] = base;
            self.fp[2] = base.add(1);
            self.color = base.add(2);
        }
    }

    fn lookup(name: *const c_char) -> Option<usize> {
        if name.is_null() {
            return None;
        }
        // SAFETY: the host passes a NUL-terminated name.
        let name = unsafe { CStr::from_ptr(name) };
        VARS.iter().position(|info| info.name == name)
    }

    fn set(&mut self, index: usize, value: f64) {
        let info = &VARS[index];
        let value = if value.is_nan() { info.default } else { value };
        self.values[index] = if info.integer {
            (value + 0.5).floor().clamp(info.min, info.max)
        } else {
            value.clamp(info.min, info.max)
        };
    }
}

// ----- identity -------------------------------------------------------------

#[no_mangle]
pub extern "C" fn PluginVarGetName() -> *const c_char {
    NAME.as_ptr()
}

#[no_mangle]
pub extern "C" fn PluginVarGetNrVariables() -> c_int {
    VARS.len() as c_int
}

#[no_mangle]
pub extern "C" fn PluginVarGetVariableNameAt(index: c_int) -> *const c_char {
    match usize::try_from(index).ok().and_then(|i| VARS.get(i)) {
        Some(info) => info.name.as_ptr(),
        None => c"".as_ptr(),
    }
}

#[no_mangle]
pub extern "C" fn PluginVarGetArchitecture() -> c_int {
    if cfg!(target_pointer_width = "64") {
        ARCH_X64
    } else {
        ARCH_X86
    }
}

// ----- lifetime -------------------------------------------------------------

#[no_mangle]
pub extern "C" fn PluginVarCreate() -> *mut c_void {
    Box::into_raw(Box::new(Variation::new())).cast()
}

/// # Safety
/// `vpp` must be null or point to a pointer returned by [`PluginVarCreate`].
#[no_mangle]
pub unsafe extern "C" fn PluginVarDestroy(vpp: *mut *mut c_void) -> c_int {
    if vpp.is_null() || (*vpp).is_null() {
        return FALSE;
    }
    drop(Box::from_raw((*vpp).cast::<Variation>()));
    *vpp = ptr::null_mut();
    TRUE
}

unsafe fn instance<'a>(varptr: *mut c_void) -> Option<&'a mut Variation> {
    varptr.cast::<Variation>().as_mut()
}

// ----- initialisation -------------------------------------------------------

/// # Safety
/// The pointers must stay valid until the instance is destroyed.
#[no_mangle]
pub unsafe extern "C" fn PluginVarInit(
    varptr: *mut c_void,
    fpx: *mut c_void,
    fpy: *mut c_void,
    ftx: *mut c_void,
    fty: *mut c_void,
    vvar: f64,
) -> c_int {
    let Some(v) = instance(varptr) else {
        return FALSE;
    };
    v.use_dummies();
    v.fp[0] = fpx.cast();
    v.fp[1] = fpy.cast();
    v.ft[0] = ftx.cast();
    v.ft[1] = fty.cast();
    v.vvar = vvar;
    TRUE
}

/// # Safety
/// The pointers must stay valid until the instance is destroyed.
#[no_mangle]
pub unsafe extern "C" fn PluginVarInit3D(
    varptr: *mut c_void,
    fpx: *mut c_void,
    fpy: *mut c_void,
    fpz: *mut c_void,
    ftx: *mut c_void,
    fty: *mut c_void,
    ftz: *mut c_void,
    vvar: f64,
) -> c_int {
    let Some(v) = instance(varptr) else {
        return FALSE;
    };
    v.use_dummies();
    v.fp = [fpx.cast(), fpy.cast(), fpz.cast()];
    v.ft = [ftx.cast(), fty.cast(), ftz.cast()];
    v.vvar = vvar;
    TRUE
}

/// DirectColor entry point of Apophysis 7X. The colour and the transform matrix
/// are not used, so this is [`PluginVarInit3D`] plus the colour pointer.
///
/// # Safety
/// The pointers must stay valid until the instance is destroyed.
#[no_mangle]
pub unsafe extern "C" fn PluginVarInitDC(
    varptr: *mut c_void,
    fpx: *mut c_void,
    fpy: *mut c_void,
    fpz: *mut c_void,
    ftx: *mut c_void,
    fty: *mut c_void,
    ftz: *mut c_void,
    color: *mut c_void,
    vvar: f64,
    _a: f64,
    _b: f64,
    _c: f64,
    _d: f64,
    _e: f64,
    _f: f64,
) -> c_int {
    let ok = PluginVarInit3D(varptr, fpx, fpy, fpz, ftx, fty, ftz, vvar);
    if let Some(v) = instance(varptr) {
        v.color = color.cast();
    }
    ok
}

// ----- variables ------------------------------------------------------------

/// # Safety
/// `name` must be a NUL-terminated string and `value` a valid pointer.
#[no_mangle]
pub unsafe extern "C" fn PluginVarGetVariable(
    varptr: *mut c_void,
    name: *const c_char,
    value: *mut f64,
) -> c_int {
    let (Some(v), Some(i)) = (instance(varptr), Variation::lookup(name)) else {
        return FALSE;
    };
    if value.is_null() {
        return FALSE;
    }
    *value = v.values[i];
    TRUE
}

/// # Safety
/// `name` must be a NUL-terminated string and `value` a valid pointer.
#[no_mangle]
pub unsafe extern "C" fn PluginVarSetVariable(
    varptr: *mut c_void,
    name: *const c_char,
    value: *mut f64,
) -> c_int {
    let (Some(v), Some(i)) = (instance(varptr), Variation::lookup(name)) else {
        return FALSE;
    };
    if value.is_null() {
        return FALSE;
    }
    v.set(i, *value);
    TRUE
}

/// # Safety
/// `name` must be a NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn PluginVarResetVariable(varptr: *mut c_void, name: *const c_char) -> c_int {
    let (Some(v), Some(i)) = (instance(varptr), Variation::lookup(name)) else {
        return FALSE;
    };
    v.values[i] = VARS[i].default;
    TRUE
}

// ----- iteration ------------------------------------------------------------

/// # Safety
/// `varptr` must come from [`PluginVarCreate`].
#[no_mangle]
pub unsafe extern "C" fn PluginVarPrepare(varptr: *mut c_void) -> c_int {
    let Some(v) = instance(varptr) else {
        return FALSE;
    };
    v.prepare();
    TRUE
}

/// # Safety
/// `varptr` must come from [`PluginVarCreate`] and have been initialised.
#[no_mangle]
pub unsafe extern "C" fn PluginVarCalc(varptr: *mut c_void) -> c_int {
    let Some(v) = instance(varptr) else {
        return FALSE;
    };
    if v.ft.iter().chain(v.fp.iter()).any(|p| p.is_null()) {
        return FALSE;
    }
    let (x, y, z) = (*v.ft[0], *v.ft[1], *v.ft[2]);
    if !(x.is_finite() && y.is_finite() && z.is_finite()) {
        return TRUE;
    }
    let (x, y, z) = (x as f32, y as f32, z as f32);
    let weight = v.vvar as f32;
    let (scale, snap) = (v.scale, v.snap);
    let rng = &mut v.rng;
    let mut unit = || rng.unit();
    let out = match v.mode {
        1 => filter_point(&v.mosaic, x, y, z, weight, scale),
        2 => snap_point(&v.mosaic, x, y, z, weight, scale, snap, &mut unit),
        3 => disk_sample(&v.mosaic, weight, &mut unit),
        _ => sample_ink(&v.mosaic, z, weight, scale, &mut unit),
    };
    if let Some(p) = out {
        if p.iter().all(|c| c.is_finite()) {
            *v.fp[0] += p[0] as f64;
            *v.fp[1] += p[1] as f64;
            *v.fp[2] += p[2] as f64;
        }
    }
    TRUE
}
