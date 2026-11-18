//! cargo run --example compare -- <types> <levels> <split> <seed> <grade> <flat|disk> <px> > out.pgm
use truchet_tiles::Mosaic;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let g = |i: usize, d: f32| a.get(i).and_then(|s| s.parse().ok()).unwrap_or(d);
    let m = Mosaic {
        types: g(1, 25.0) as u32,
        levels: g(2, 6.0) as i32,
        split: g(3, 0.6),
        seed: g(4, 3.0) as u32,
        width: 6.0,
        height: 6.0,
        noise: 0.1,
        grade: g(5, 0.0),
    };
    let disk = a.get(6).map(|s| s == "disk").unwrap_or(false);
    let px = g(7, 700.0) as usize;
    println!("P5\n{px} {px}\n255");
    let mut out = Vec::with_capacity(px * px);
    for r in 0..px {
        for c in 0..px {
            let fx = (c as f32 + 0.5) / px as f32;
            let fy = (r as f32 + 0.5) / px as f32;
            let ink = if disk {
                let (wr, wi) = (fx * 2.0 - 1.0, 1.0 - fy * 2.0);
                if wr * wr + wi * wi > 0.995 {
                    out.push(128u8);
                    continue;
                }
                // z = i (1 + w) / (1 - w)
                let (nr, ni) = (1.0 + wr, wi);
                let (dr, di) = (1.0 - wr, -wi);
                let den = dr * dr + di * di;
                let (qr, qi) = ((nr * dr + ni * di) / den, (ni * dr - nr * di) / den);
                m.ink(-qi, qr)
            } else {
                m.ink(fx * 6.0, fy * 6.0)
            };
            out.push(if ink { 0u8 } else { 255u8 });
        }
    }
    use std::io::Write;
    std::io::stdout().write_all(&out).unwrap();
}
