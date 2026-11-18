//! Writes a PNG contact sheet of a tile set to stdout.
//!
//! ```text
//! cargo run --example sheet -- square > square.png   # Carlson's fifteen
//! cargo run --example sheet -- round  > round.png    # the rounded set, 22 tiles
//! cargo run --example sheet -- round 160 > big.png   # tile size in pixels
//! ```
use std::io::Write;
use truchet_tiles::{RoundTile, Tile};

const GUTTER: usize = 12;
const BACKGROUND: u8 = 238;
const BORDER: u8 = 170;
const INK: u8 = 18;
const GROUND: u8 = 255;

type TileFn = Box<dyn Fn(f32, f32) -> bool>;

fn main() {
    let mut args = std::env::args().skip(1);
    let set = args.next().unwrap_or_else(|| "square".into());
    let px: usize = args
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(120)
        .max(16);

    // Each cell is a closure from (u, v) in -0.5..=0.5 (v up) to "is ink".
    let (cols, tiles): (usize, Vec<TileFn>) = match set.as_str() {
        "square" => (
            5,
            Tile::ALL
                .iter()
                .map(|&t| Box::new(move |u, v| t.ink(u, v)) as Box<_>)
                .collect(),
        ),
        "round" => (
            6,
            RoundTile::ALL
                .iter()
                .map(|&t| Box::new(move |u, v| t.ink(u, v, 0)) as Box<_>)
                .collect(),
        ),
        other => {
            eprintln!("unknown set {other:?}; use `square` or `round`");
            std::process::exit(2);
        }
    };

    let rows = tiles.len().div_ceil(cols);
    let width = GUTTER + cols * (px + GUTTER);
    let height = GUTTER + rows * (px + GUTTER);
    let mut pixels = vec![BACKGROUND; width * height];

    for (i, ink) in tiles.iter().enumerate() {
        let x0 = GUTTER + (i % cols) * (px + GUTTER);
        let y0 = GUTTER + (i / cols) * (px + GUTTER);
        for r in 0..px {
            for c in 0..px {
                let u = -0.5 + (c as f32 + 0.5) / px as f32;
                let v = 0.5 - (r as f32 + 0.5) / px as f32;
                let edge = r == 0 || c == 0 || r == px - 1 || c == px - 1;
                pixels[(y0 + r) * width + x0 + c] = if edge {
                    BORDER
                } else if ink(u, v) {
                    INK
                } else {
                    GROUND
                };
            }
        }
    }

    std::io::stdout()
        .write_all(&png_gray(width, height, &pixels))
        .unwrap();
}

/// Minimal 8-bit grayscale PNG: stored (uncompressed) deflate blocks, no dependencies.
fn png_gray(width: usize, height: usize, pixels: &[u8]) -> Vec<u8> {
    let mut raw = Vec::with_capacity((width + 1) * height);
    for row in pixels.chunks(width) {
        raw.push(0); // filter: none
        raw.extend_from_slice(row);
    }

    let mut z = vec![0x78, 0x01];
    let mut blocks = raw.chunks(65535).peekable();
    while let Some(block) = blocks.next() {
        z.push(u8::from(blocks.peek().is_none()));
        z.extend_from_slice(&(block.len() as u16).to_le_bytes());
        z.extend_from_slice(&(!(block.len() as u16)).to_le_bytes());
        z.extend_from_slice(block);
    }
    z.extend_from_slice(&adler32(&raw).to_be_bytes());

    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&(width as u32).to_be_bytes());
    ihdr.extend_from_slice(&(height as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, 0, 0, 0, 0]); // 8 bit, grayscale, deflate, no filter, no interlace
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &z);
    chunk(&mut out, b"IEND", &[]);
    out
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let mut body = kind.to_vec();
    body.extend_from_slice(data);
    out.extend_from_slice(&body);
    out.extend_from_slice(&crc32(&body).to_be_bytes());
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in data {
        a = (a + byte as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}
