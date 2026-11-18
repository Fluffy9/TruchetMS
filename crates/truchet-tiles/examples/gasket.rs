//! Offline check of the gasket flame: tiled hyperbolic disk copies placed in every
//! circle of an Apollonian gasket (outer r=1, two r=1/2, r=1/3, ...) with the
//! reflection group, following the same xaos rules the flame file uses.
use truchet_tiles::{filter_point, Mosaic};
type C = (f64, f64);
fn inv(z: C, c: C, r: f64) -> C {
    // inversion in circle (c, r)
    let (dx, dy) = (z.0 - c.0, z.1 - c.1);
    let d2 = dx * dx + dy * dy + 1e-18;
    (c.0 + r * r * dx / d2, c.1 + r * r * dy / d2)
}
fn s(i: usize, z: C) -> C {
    match i {
        0 => inv(z, (0.0, 0.25), 0.25),
        1 => inv(z, (-1.0, 1.0), 1.0),
        2 => inv(z, (1.0, 1.0), 1.0),
        _ => (z.0, -z.1),
    }
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let get = |i: usize, d: f64| a.get(i).and_then(|s| s.parse().ok()).unwrap_or(d);
    let cont = get(1, 0.62);
    let iters = get(2, 30_000_000.0) as u64;
    let px = 900usize;
    let m = Mosaic {
        types: 255,
        levels: 6,
        split: 0.6,
        seed: 3,
        width: 6.0,
        height: 6.0,
        noise: 0.1,
        grade: 0.5,
    };
    let mut st: u64 = 88172645463325252;
    let mut rnd = move || {
        st ^= st << 13;
        st ^= st >> 7;
        st ^= st << 17;
        (st >> 11) as f64 / (1u64 << 53) as f64
    };
    let mut img = vec![0u32; px * px];
    let plot = |z: C, img: &mut Vec<u32>| {
        let x = ((z.0 * 0.5 + 0.5) * px as f64) as i64;
        let y = ((0.5 - z.1 * 0.5) * px as f64) as i64;
        if x >= 0 && y >= 0 && (x as usize) < px && (y as usize) < px {
            img[y as usize * px + x as usize] += 1;
        }
    };
    for _ in 0..iters {
        let (rr, th) = (rnd().sqrt(), rnd() * std::f64::consts::TAU);
        let (wr, wi) = (rr * th.cos(), rr * th.sin());
        let (nr, ni, dr, di) = (1.0 + wr, wi, 1.0 - wr, -wi);
        let dd = dr * dr + di * di;
        if dd < 1e-12 {
            continue;
        }
        let (qr, qi) = ((nr * dr + ni * di) / dd, (ni * dr - nr * di) / dd);
        let Some(p) = filter_point(&m, (-qi) as f32, qr as f32, 0.0, 1.0, 1.0) else {
            continue;
        };
        let (ar, ai, br, bi) = (
            p[0] as f64,
            p[1] as f64 - 1.0,
            p[0] as f64,
            p[1] as f64 + 1.0,
        );
        let bb = br * br + bi * bi;
        if bb < 1e-12 {
            continue;
        }
        let d: C = ((ar * br + ai * bi) / bb, (ai * br - ar * bi) / bb);
        let start = (rnd() * 4.0) as usize;
        let mut z: C = match start {
            0 => {
                let n = d.0 * d.0 + d.1 * d.1 + 1e-18;
                (d.0 / n, d.1 / n)
            }
            1 => (0.5 * d.0 + 0.5, 0.5 * d.1),
            2 => (0.5 * d.0 - 0.5, 0.5 * d.1),
            _ => (d.0 / 3.0, d.1 / 3.0 + 2.0 / 3.0),
        };
        if start != 0 {
            plot(z, &mut img);
        }
        let mut last = start;
        z = s(start, z);
        plot(z, &mut img);
        while rnd() < cont {
            let mut k = (rnd() * 3.0) as usize;
            if k >= last {
                k += 1;
            }
            z = s(k, z);
            last = k;
            plot(z, &mut img);
        }
    }
    let max = *img.iter().max().unwrap() as f64;
    println!("P5\n{px} {px}\n255");
    let out: Vec<u8> = img
        .iter()
        .map(|&c| (255.0 * (c as f64 / (max * 0.02)).powf(0.5).min(1.0)) as u8)
        .collect();
    use std::io::Write;
    std::io::stdout().write_all(&out).unwrap();
}
