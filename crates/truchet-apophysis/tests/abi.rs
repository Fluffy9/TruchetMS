//! Drives the plugin through its C exports the way Apophysis 7X does.

use std::ffi::{c_void, CStr, CString};
use std::ptr;

use truchet_ms::*;
use truchet_tiles::{Mosaic, TYPE_ROUND};

/// The iterator state a host owns and hands to the plugin by pointer.
struct Host {
    ft: [f64; 3],
    fp: [f64; 3],
    color: f64,
}

impl Host {
    fn new() -> Box<Self> {
        Box::new(Host {
            ft: [0.0; 3],
            fp: [0.0; 3],
            color: 0.0,
        })
    }
}

struct Plugin {
    handle: *mut c_void,
}

impl Plugin {
    fn new() -> Self {
        let handle = PluginVarCreate();
        assert!(!handle.is_null());
        Plugin { handle }
    }

    fn set(&self, name: &str, value: f64) -> bool {
        let name = CString::new(name).unwrap();
        let mut value = value;
        unsafe { PluginVarSetVariable(self.handle, name.as_ptr(), &mut value) == 1 }
    }

    fn get(&self, name: &str) -> Option<f64> {
        let name = CString::new(name).unwrap();
        let mut value = 0.0;
        let ok = unsafe { PluginVarGetVariable(self.handle, name.as_ptr(), &mut value) };
        (ok == 1).then_some(value)
    }

    fn init3d(&self, host: &mut Host, weight: f64) {
        let ok = unsafe {
            PluginVarInit3D(
                self.handle,
                &mut host.fp[0] as *mut f64 as *mut c_void,
                &mut host.fp[1] as *mut f64 as *mut c_void,
                &mut host.fp[2] as *mut f64 as *mut c_void,
                &mut host.ft[0] as *mut f64 as *mut c_void,
                &mut host.ft[1] as *mut f64 as *mut c_void,
                &mut host.ft[2] as *mut f64 as *mut c_void,
                weight,
            )
        };
        assert_eq!(ok, 1);
    }

    fn prepare(&self) {
        assert_eq!(unsafe { PluginVarPrepare(self.handle) }, 1);
    }

    fn calc(&self) {
        assert_eq!(unsafe { PluginVarCalc(self.handle) }, 1);
    }
}

impl Drop for Plugin {
    fn drop(&mut self) {
        let mut handle = self.handle;
        assert_eq!(unsafe { PluginVarDestroy(&mut handle) }, 1);
        assert!(handle.is_null(), "Destroy must null the host's pointer");
    }
}

fn cstr(p: *const std::ffi::c_char) -> String {
    unsafe { CStr::from_ptr(p) }.to_str().unwrap().to_string()
}

#[test]
fn reports_its_name_and_flame_attribute_names() {
    assert_eq!(cstr(PluginVarGetName()), "truchet_ms");
    let n = PluginVarGetNrVariables();
    assert_eq!(n as usize, VARS.len());
    let names: Vec<String> = (0..n)
        .map(|i| cstr(PluginVarGetVariableNameAt(i)))
        .collect();
    // These are the attributes Fireheart writes, so its flames load unchanged.
    for want in [
        "truchet_ms_seed",
        "truchet_ms_width",
        "truchet_ms_height",
        "truchet_ms_levels",
        "truchet_ms_split",
        "truchet_ms_types",
        "truchet_ms_style",
        "truchet_ms_noise",
        "truchet_ms_scale",
        "truchet_ms_mode",
        "truchet_ms_snap",
        "truchet_ms_grade",
    ] {
        assert!(names.iter().any(|n| n == want), "missing {want}: {names:?}");
    }
    assert_eq!(cstr(PluginVarGetVariableNameAt(-1)), "");
    assert_eq!(cstr(PluginVarGetVariableNameAt(n)), "");
    assert!(matches!(PluginVarGetArchitecture(), 0x20 | 0x40));
}

#[test]
fn variables_default_clamp_round_and_reset() {
    let p = Plugin::new();
    assert_eq!(p.get("truchet_ms_seed"), Some(1.0));
    assert_eq!(p.get("truchet_ms_types"), Some(3.0));

    assert!(p.set("truchet_ms_seed", 5.5)); // integers round, halves up
    assert_eq!(p.get("truchet_ms_seed"), Some(6.0));
    assert!(p.set("truchet_ms_width", 1000.0)); // reals clamp to their range
    assert_eq!(p.get("truchet_ms_width"), Some(64.0));
    assert!(p.set("truchet_ms_levels", -3.0));
    assert_eq!(p.get("truchet_ms_levels"), Some(0.0));
    assert!(p.set("truchet_ms_style", f64::NAN)); // NaN falls back to the default
    assert_eq!(p.get("truchet_ms_style"), Some(0.0));

    let name = CString::new("truchet_ms_width").unwrap();
    assert_eq!(
        unsafe { PluginVarResetVariable(p.handle, name.as_ptr()) },
        1
    );
    assert_eq!(p.get("truchet_ms_width"), Some(6.0));

    assert!(!p.set("not_a_variable", 1.0));
    assert_eq!(p.get("not_a_variable"), None);
    let unknown = CString::new("not_a_variable").unwrap();
    assert_eq!(
        unsafe { PluginVarResetVariable(p.handle, unknown.as_ptr()) },
        0
    );
}

#[test]
fn null_handles_are_rejected_not_dereferenced() {
    let name = CString::new("truchet_ms_seed").unwrap();
    let mut value = 1.0;
    unsafe {
        assert_eq!(PluginVarPrepare(ptr::null_mut()), 0);
        assert_eq!(PluginVarCalc(ptr::null_mut()), 0);
        assert_eq!(
            PluginVarGetVariable(ptr::null_mut(), name.as_ptr(), &mut value),
            0
        );
        assert_eq!(PluginVarDestroy(ptr::null_mut()), 0);
        let mut null: *mut c_void = ptr::null_mut();
        assert_eq!(PluginVarDestroy(&mut null), 0);
    }
}

#[test]
fn calc_before_init_does_nothing_and_fails() {
    let p = Plugin::new();
    assert_eq!(unsafe { PluginVarCalc(p.handle) }, 0);
}

/// The mosaic a default plugin builds, for comparing against `Mosaic::ink`.
fn default_mosaic() -> Mosaic {
    Mosaic {
        seed: 1,
        width: 6.0,
        height: 6.0,
        levels: 4,
        split: 0.5,
        types: 3,
        noise: 0.45,
        grade: 0.0,
    }
}

/// Filter mode keeps exactly the points the mosaic says are ink, scaled by the weight.
fn check_filter(setup: impl Fn(&Plugin), mosaic: &Mosaic) {
    let p = Plugin::new();
    p.set("truchet_ms_mode", 1.0);
    setup(&p);
    let mut host = Host::new();
    p.init3d(&mut host, 0.5);
    p.prepare();
    let (mut kept, mut dropped) = (0, 0);
    for i in 0..600 {
        let x = i as f64 * 0.137 - 20.0;
        let y = (i as f64 * 0.071).sin() * 9.0;
        host.ft = [x, y, 0.25];
        host.fp = [0.0; 3];
        p.calc();
        if mosaic.ink(x as f32, y as f32) {
            kept += 1;
            assert!(
                (host.fp[0] - x * 0.5).abs() < 1e-4,
                "x at {x},{y}: {:?}",
                host.fp
            );
            assert!((host.fp[1] - y * 0.5).abs() < 1e-4);
            assert!((host.fp[2] - 0.125).abs() < 1e-6);
        } else {
            dropped += 1;
            assert_eq!(host.fp, [0.0; 3], "ground must add nothing at {x},{y}");
        }
    }
    assert!(kept > 50 && dropped > 50, "kept {kept}, dropped {dropped}");
}

#[test]
fn filter_mode_matches_the_square_mosaic() {
    check_filter(|_| {}, &default_mosaic());
}

#[test]
fn style_one_switches_to_the_rounded_set() {
    let rounded = Mosaic {
        types: TYPE_ROUND,
        ..default_mosaic()
    };
    check_filter(
        |p| {
            p.set("truchet_ms_style", 1.0);
        },
        &rounded,
    );
    // The rounded set must actually differ from the square one at some points.
    let square = default_mosaic();
    let differs = (0..2000).any(|i| {
        let (x, y) = (i as f32 * 0.0731, (i as f32 * 0.0419).cos() * 5.0 + 5.0);
        rounded.ink(x, y) != square.ink(x, y)
    });
    assert!(differs);
}

#[test]
fn types_bitmask_reaches_the_mosaic() {
    let all = Mosaic {
        types: 255,
        ..default_mosaic()
    };
    check_filter(
        |p| {
            p.set("truchet_ms_types", 255.0);
        },
        &all,
    );
}

#[test]
fn generate_mode_lands_on_ink_and_adds_to_fp() {
    let p = Plugin::new();
    let mut host = Host::new();
    p.init3d(&mut host, 1.0);
    p.prepare();
    let mosaic = default_mosaic();
    let mut hits = 0;
    for _ in 0..2000 {
        host.ft = [0.3, 0.4, 0.7];
        host.fp = [10.0, 20.0, 30.0]; // generate must add to, not overwrite, FP
        p.calc();
        let (dx, dy, dz) = (host.fp[0] - 10.0, host.fp[1] - 20.0, host.fp[2] - 30.0);
        if dx != 0.0 || dy != 0.0 {
            hits += 1;
            // Output is centred on the mosaic; shifting back lands on ink.
            assert!(
                mosaic.ink((dx + 3.0) as f32, (dy + 3.0) as f32),
                "{dx} {dy}"
            );
            assert!((dz - 0.7).abs() < 1e-6, "z passes through");
            assert!(dx.abs() <= 3.0 + 1e-6 && dy.abs() <= 3.0 + 1e-6);
        }
    }
    assert!(
        hits > 1500,
        "generate should almost always find ink, got {hits}"
    );
}

#[test]
fn snap_mode_always_returns_ink() {
    let p = Plugin::new();
    p.set("truchet_ms_mode", 2.0);
    let mut host = Host::new();
    p.init3d(&mut host, 1.0);
    p.prepare();
    let mosaic = default_mosaic();
    let (mut moved, mut kept) = (0, 0);
    for i in 0..800 {
        let (x, y) = (i as f64 * 0.113 - 25.0, (i as f64 * 0.057).cos() * 8.0);
        host.ft = [x, y, 0.0];
        host.fp = [0.0; 3];
        p.calc();
        if host.fp == [0.0; 3] {
            continue; // no ink within reach: adds nothing
        }
        assert!(
            mosaic.ink(host.fp[0] as f32, host.fp[1] as f32),
            "snapped point is ink"
        );
        // The engine works in f32, so an untouched point differs by rounding only.
        if (host.fp[0] - x).abs() < 1e-4 && (host.fp[1] - y).abs() < 1e-4 {
            kept += 1
        } else {
            moved += 1
        }
    }
    assert!(kept > 100 && moved > 100, "kept {kept}, moved {moved}");
}

#[test]
fn disk_mode_stays_inside_the_unit_disk() {
    let p = Plugin::new();
    for (name, v) in [
        ("truchet_ms_mode", 3.0),
        ("truchet_ms_grade", 0.5),
        ("truchet_ms_levels", 6.0),
        ("truchet_ms_types", 255.0),
    ] {
        p.set(name, v);
    }
    let mut host = Host::new();
    p.init3d(&mut host, 1.0);
    p.prepare();
    let mut hits = 0;
    for _ in 0..1500 {
        host.ft = [9.0, 9.0, 0.0]; // disk mode ignores the incoming point
        host.fp = [0.0; 3];
        p.calc();
        if host.fp != [0.0; 3] {
            hits += 1;
            assert!(host.fp[0].hypot(host.fp[1]) <= 1.0001, "{:?}", host.fp);
        }
    }
    assert!(hits > 1400, "hits {hits}");
}

#[test]
fn two_dimensional_init_never_touches_z_or_colour() {
    let p = Plugin::new();
    let (mut fpx, mut fpy, mut ftx, mut fty) = (0.0f64, 0.0f64, 0.3f64, 0.4f64);
    let ok = unsafe {
        PluginVarInit(
            p.handle,
            &mut fpx as *mut f64 as *mut c_void,
            &mut fpy as *mut f64 as *mut c_void,
            &mut ftx as *mut f64 as *mut c_void,
            &mut fty as *mut f64 as *mut c_void,
            1.0,
        )
    };
    assert_eq!(ok, 1);
    p.prepare();
    for _ in 0..200 {
        p.calc();
    }
    assert!(fpx != 0.0 || fpy != 0.0, "2D hosts still get output");
}

#[test]
fn dc_init_accepts_the_colour_pointer() {
    let p = Plugin::new();
    let mut host = Host::new();
    let ok = unsafe {
        PluginVarInitDC(
            p.handle,
            &mut host.fp[0] as *mut f64 as *mut c_void,
            &mut host.fp[1] as *mut f64 as *mut c_void,
            &mut host.fp[2] as *mut f64 as *mut c_void,
            &mut host.ft[0] as *mut f64 as *mut c_void,
            &mut host.ft[1] as *mut f64 as *mut c_void,
            &mut host.ft[2] as *mut f64 as *mut c_void,
            &mut host.color as *mut f64 as *mut c_void,
            1.0,
            1.0,
            0.0,
            0.0,
            1.0,
            0.0,
            0.0,
        )
    };
    assert_eq!(ok, 1);
    p.prepare();
    host.ft = [0.3, 0.4, 0.0];
    p.calc();
    assert_eq!(host.color, 0.0, "the plugin leaves the colour alone");
}

#[test]
fn non_finite_input_adds_nothing() {
    let p = Plugin::new();
    p.set("truchet_ms_mode", 1.0);
    let mut host = Host::new();
    p.init3d(&mut host, 1.0);
    p.prepare();
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        host.ft = [bad, 1.0, 0.0];
        host.fp = [0.0; 3];
        p.calc();
        assert_eq!(host.fp, [0.0; 3]);
    }
}

#[test]
fn instances_draw_independent_random_streams() {
    let draw = || {
        let p = Plugin::new();
        let mut host = Host::new();
        p.init3d(&mut host, 1.0);
        p.prepare();
        (0..20)
            .map(|_| {
                host.ft = [0.0; 3];
                host.fp = [0.0; 3];
                p.calc();
                host.fp
            })
            .collect::<Vec<_>>()
    };
    assert_ne!(
        draw(),
        draw(),
        "two instances must not replay the same samples"
    );
}
