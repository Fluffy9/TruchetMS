use truchet_tiles::*;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let f = |i: usize| a[i].parse::<f32>().unwrap();
    let m = Mosaic {
        types: f(1) as u32,
        seed: f(2) as u32,
        levels: f(3) as i32,
        split: f(4),
        noise: f(5),
        grade: f(6),
        width: f(7),
        height: f(7),
    };
    let n = 256usize;
    let (x0, y0, span) = (f(8), f(9), f(10));
    let mut s = String::new();
    for j in 0..n {
        for i in 0..n {
            let x = x0 + (i as f32 + 0.5) / n as f32 * span;
            let y = y0 + (j as f32 + 0.5) / n as f32 * span;
            s.push(if m.ink(x, y) { '1' } else { '0' });
        }
        s.push('\n');
    }
    print!("{s}");
}
