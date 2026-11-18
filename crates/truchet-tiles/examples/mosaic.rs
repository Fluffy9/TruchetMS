//! Renders a mosaic to a binary PGM. Usage:
//! cargo run --example mosaic -- <types> <levels> <split> <seed> <px> > out.pgm
use truchet_tiles::Mosaic;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let get = |i: usize, d: f32| a.get(i).and_then(|s| s.parse().ok()).unwrap_or(d);
    let m = Mosaic {
        types: get(1, 3.0) as u32,
        levels: get(2, 4.0) as i32,
        split: get(3, 0.5),
        seed: get(4, 1.0) as u32,
        width: 6.0,
        height: 6.0,
        noise: 0.45,
        grade: get(6, 0.0),
    };
    let px = get(5, 600.0) as usize;
    println!("P5\n{px} {px}\n255");
    let mut out = Vec::with_capacity(px * px);
    for r in 0..px {
        for c in 0..px {
            let x = (c as f32 + 0.5) / px as f32 * m.width;
            let y = m.height - (r as f32 + 0.5) / px as f32 * m.height;
            out.push(if m.ink(x, y) { 0u8 } else { 255u8 });
        }
    }
    use std::io::Write;
    std::io::stdout().write_all(&out).unwrap();
}
