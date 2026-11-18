//! Offline check of: uniform disk -> inverse Cayley -> tile filter -> Cayley -> crop.
use truchet_tiles::{filter_point, Mosaic};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let get = |i: usize, d: f32| a.get(i).and_then(|s| s.parse().ok()).unwrap_or(d);
    let m = Mosaic {
        types: get(1, 255.0) as u32,
        levels: get(2, 6.0) as i32,
        split: 0.6,
        seed: 3,
        width: 6.0,
        height: 6.0,
        noise: 0.1,
        grade: get(3, 0.5),
    };
    let px = 700usize;
    let r_src = get(4, 1.2) as f64;
    let mut img = vec![0u32; px * px];
    let mut s: u64 = 88172645463325252;
    let mut rnd = || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        (s >> 11) as f64 / (1u64 << 53) as f64
    };
    let (mut kept, mut tried) = (0u64, 0u64);
    for _ in 0..40_000_000u64 {
        // blur_circle-like: uniform in a disk of radius r_src
        let (rr, th) = (r_src * rnd().sqrt(), rnd() * std::f64::consts::TAU);
        let (wr, wi) = (rr * th.cos(), rr * th.sin());
        // z = i(1+w)/(1-w)
        let (nr, ni) = (1.0 + wr, wi);
        let (dr, di) = (1.0 - wr, -wi);
        let dd = dr * dr + di * di;
        if dd < 1e-12 {
            continue;
        }
        let (qr, qi) = ((nr * dr + ni * di) / dd, (ni * dr - nr * di) / dd);
        let (zr, zi) = (-qi, qr); // times i
        tried += 1;
        let Some(p) = filter_point(&m, zr as f32, zi as f32, 0.0, 1.0, 1.0) else {
            continue;
        };
        kept += 1;
        // w = (z - i)/(z + i)
        let (z_r, z_i) = (p[0] as f64, p[1] as f64);
        let (ar, ai) = (z_r, z_i - 1.0);
        let (br, bi) = (z_r, z_i + 1.0);
        let bb = br * br + bi * bi;
        if bb < 1e-12 {
            continue;
        }
        let (fr, fi) = ((ar * br + ai * bi) / bb, (ai * br - ar * bi) / bb);
        if fr * fr + fi * fi > 1.0 {
            continue;
        }
        let x = ((fr * 0.5 + 0.5) * px as f64) as usize;
        let y = ((0.5 - fi * 0.5) * px as f64) as usize;
        if x < px && y < px {
            img[y * px + x] += 1;
        }
    }
    eprintln!("kept {kept} of {tried}");
    let max = *img.iter().max().unwrap_or(&1) as f64;
    println!("P5\n{px} {px}\n255");
    let out: Vec<u8> = img
        .iter()
        .map(|&c| (255.0 * (c as f64 / (max * 0.15)).min(1.0)) as u8)
        .collect();
    use std::io::Write;
    std::io::stdout().write_all(&out).unwrap();
}
