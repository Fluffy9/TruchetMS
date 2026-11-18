//! Prints every tile as an N x N mask ('1' = ink), `v` up. Usage: cargo run --example tiles [N]
use truchet_tiles::{RoundTile, Tile};

fn main() {
    let n: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(128);
    for tile in Tile::ALL {
        println!("## {tile:?}");
        for r in 0..n {
            let v = 0.5 - (r as f32 + 0.5) / n as f32;
            let row: String = (0..n)
                .map(|c| {
                    if tile.ink(-0.5 + (c as f32 + 0.5) / n as f32, v) {
                        '1'
                    } else {
                        '0'
                    }
                })
                .collect();
            println!("{row}");
        }
    }
    for tile in RoundTile::ALL {
        for rot in 0..tile.turns() {
            println!("## {tile:?}{rot}");
            for r in 0..n {
                let v = 0.5 - (r as f32 + 0.5) / n as f32;
                let row: String = (0..n)
                    .map(|c| {
                        if tile.ink(-0.5 + (c as f32 + 0.5) / n as f32, v, rot) {
                            '1'
                        } else {
                            '0'
                        }
                    })
                    .collect();
                println!("{row}");
            }
        }
    }
}
