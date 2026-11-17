//! Multi-scale Truchet mosaic.
//!
//! Tiles are the Carlson bands: a stroke of half-width `1/6` around a circle
//! of radius `1/2` centred on a corner of the unit square. Odd subdivision
//! levels swap ink and ground. A cell splits with [`Mosaic::split`], up to
//! [`Mosaic::levels`]. Where the fine tiles sit is whatever space the point
//! arrived in, so a log variation in front of this one can grade the detail.

/// Which corner-pair a tile uses. Bits of [`Mosaic::types`].
pub const TYPE_DL: u32 = 1;
pub const TYPE_DR: u32 = 2;

const BAND: f32 = 1.0 / 6.0;
const RADIUS: f32 = 0.5;

#[derive(Clone, Debug)]
pub struct Mosaic {
    pub seed: u32,
    pub width: f32,
    pub height: f32,
    /// Deepest subdivision. `0` is a single tile per cell.
    pub levels: i32,
    /// Chance in `0..1` that a cell splits. Independent of position.
    pub split: f32,
    /// Bitmask of [`TYPE_DL`] and [`TYPE_DR`]. Empty means both.
    pub types: u32,
    /// How far the split roll is nudged. `0` is a hard chance.
    pub noise: f32,
}

impl Default for Mosaic {
    fn default() -> Self {
        Self {
            seed: 1,
            width: 6.0,
            height: 6.0,
            levels: 4,
            split: 0.5,
            types: TYPE_DL | TYPE_DR,
            noise: 0.45,
        }
    }
}

impl Mosaic {
    pub fn sanitized(&self) -> Self {
        let mut out = self.clone();
        out.width = out.width.clamp(1.0, 64.0);
        out.height = out.height.clamp(1.0, 64.0);
        out.levels = out.levels.clamp(0, 8);
        out.split = out.split.clamp(0.0, 1.0);
        out.noise = out.noise.max(0.0);
        if out.types & (TYPE_DL | TYPE_DR) == 0 {
            out.types = TYPE_DL | TYPE_DR;
        }
        out
    }

    /// `true` when `(x, y)` lands on ink. Coordinates wrap into the mosaic.
    pub fn ink(&self, x: f32, y: f32) -> bool {
        let m = self.sanitized();
        let px = wrap(x, m.width);
        let py = wrap(y, m.height);
        let mut cx = px.floor() + 0.5;
        let mut cy = py.floor() + 0.5;
        let mut level = 0;
        while level < m.levels {
            if !m.splits(cx, cy, level) {
                break;
            }
            let step = 0.5f32.powi(level) * 0.25;
            cx += if px < cx { -step } else { step };
            cy += if py < cy { -step } else { step };
            level += 1;
        }
        let scale = 0.5f32.powi(level);
        let u = (px - cx) / scale;
        let v = (py - cy) / scale;
        let kind = m.tile_at(cx, cy, level);
        let band = on_band(kind, u, v);
        if level % 2 == 1 { !band } else { band }
    }

    fn splits(&self, cx: f32, cy: f32, level: i32) -> bool {
        if level >= self.levels {
            return false;
        }
        let roll = unit(mix(self.seed, quant(cx), quant(cy), level, 1));
        let nudged = if self.noise == 0.0 {
            roll
        } else {
            (roll + gaussian(self.seed, quant(cx), quant(cy), level, 3) * self.noise).clamp(0.0, 1.0)
        };
        nudged < self.split
    }

    fn tile_at(&self, cx: f32, cy: f32, level: i32) -> Tile {
        let choices = self.choices();
        let h = mix(self.seed, quant(cx), quant(cy), level, 2);
        choices[(h as usize) % choices.len()]
    }

    fn choices(&self) -> Vec<Tile> {
        let mut out = Vec::with_capacity(2);
        if self.types & TYPE_DL != 0 {
            out.push(Tile::Dl);
        }
        if self.types & TYPE_DR != 0 {
            out.push(Tile::Dr);
        }
        if out.is_empty() {
            out.push(Tile::Dl);
            out.push(Tile::Dr);
        }
        out
    }
}

/// Map one chaos point. `None` is ground (the host should drop the sample).
/// Ink is returned centred on the mosaic and scaled by `weight`.
pub fn map_point(mosaic: &Mosaic, x: f32, y: f32, z: f32, weight: f32, scale: f32) -> Option<[f32; 3]> {
    let m = mosaic.sanitized();
    let sx = x * scale;
    let sy = y * scale;
    if !m.ink(sx, sy) {
        return None;
    }
    Some([
        (wrap(sx, m.width) - m.width * 0.5) * weight,
        (wrap(sy, m.height) - m.height * 0.5) * weight,
        z * weight,
    ])
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tile {
    Dl,
    Dr,
}

fn on_band(tile: Tile, u: f32, v: f32) -> bool {
    if u.abs() > 0.5 || v.abs() > 0.5 {
        return false;
    }
    let corners: [(f32, f32); 2] = match tile {
        Tile::Dl => [(-0.5, -0.5), (0.5, 0.5)],
        Tile::Dr => [(-0.5, 0.5), (0.5, -0.5)],
    };
    corners
        .into_iter()
        .any(|(cx, cy)| ((u - cx).hypot(v - cy) - RADIUS).abs() < BAND)
}

fn wrap(value: f32, span: f32) -> f32 {
    if !value.is_finite() || span <= 0.0 {
        return 0.0;
    }
    let mut m = value % span;
    if m < 0.0 {
        m += span;
    }
    if m == span { 0.0 } else { m }
}

fn quant(value: f32) -> i32 {
    (value * 4096.0).round() as i32
}

fn mix(seed: u32, x: i32, y: i32, level: i32, salt: u32) -> u32 {
    let mut n = seed
        .wrapping_mul(0x9E37_79B1)
        .wrapping_add(x as u32)
        .wrapping_mul(0x85EB_CA6B)
        .wrapping_add(y as u32)
        .wrapping_mul(0xC2B2_AE35)
        .wrapping_add(level as u32)
        .wrapping_mul(0x27D4_EB2F)
        .wrapping_add(salt);
    n ^= n >> 16;
    n = n.wrapping_mul(0x7FEB_352D);
    n ^= n >> 15;
    n = n.wrapping_mul(0x846C_A68B);
    n ^ (n >> 16)
}

fn unit(hash: u32) -> f32 {
    (hash as f32 + 0.5) / 4294967296.0
}

fn gaussian(seed: u32, x: i32, y: i32, level: i32, salt: u32) -> f32 {
    let u = unit(mix(seed, x, y, level, salt)).clamp(1e-4, 1.0 - 1e-4);
    let v = unit(mix(seed, x, y, level, salt.wrapping_add(17)));
    let radius = (-2.0 * u.ln()).sqrt();
    radius * (std::f32::consts::TAU * v).cos()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat() -> Mosaic {
        Mosaic {
            levels: 0,
            split: 0.0,
            noise: 0.0,
            types: TYPE_DL,
            ..Mosaic::default()
        }
    }

    #[test]
    fn diagonal_arc_is_ink_and_the_middle_is_not() {
        let mosaic = flat();
        let along = 0.5 / std::f32::consts::SQRT_2;
        let on_arc = 0.5 + (-0.5 + along);
        assert!(mosaic.ink(on_arc, on_arc), "quarter-circle should be ink");
        assert!(!mosaic.ink(0.5, 0.5), "tile centre is ground");
    }

    #[test]
    fn odd_level_flips_the_band() {
        let centre = 0.5;
        assert!(!flat().ink(centre, centre));
        let odd = Mosaic {
            levels: 1,
            split: 1.0,
            noise: 0.0,
            width: 6.0,
            height: 6.0,
            types: TYPE_DL,
            seed: 1,
        };
        assert!(odd.ink(centre, centre), "odd level paints the ground");
    }

    #[test]
    fn split_chance_is_the_same_everywhere() {
        let shallow = Mosaic {
            levels: 4,
            split: 0.0,
            noise: 0.0,
            ..Mosaic::default()
        };
        let deep = Mosaic {
            split: 1.0,
            ..shallow.clone()
        };
        assert_eq!(leaf_level(&shallow, 0.25, 3.0), 0);
        assert_eq!(leaf_level(&shallow, 5.75, 3.0), 0);
        assert_eq!(leaf_level(&deep, 0.25, 3.0), 4);
        assert_eq!(leaf_level(&deep, 5.75, 1.2), 4);
    }

    #[test]
    fn the_same_point_is_stable() {
        let mosaic = Mosaic::default();
        assert_eq!(mosaic.ink(1.2, 3.4), mosaic.ink(1.2, 3.4));
        assert_eq!(mosaic.ink(1.2, 3.4), mosaic.ink(1.2 + 6.0, 3.4 - 12.0));
    }

    fn leaf_level(mosaic: &Mosaic, px: f32, py: f32) -> i32 {
        let m = mosaic.sanitized();
        let px = wrap(px, m.width);
        let py = wrap(py, m.height);
        let mut cx = px.floor() + 0.5;
        let mut cy = py.floor() + 0.5;
        let mut level = 0;
        while level < m.levels {
            if !m.splits(cx, cy, level) {
                break;
            }
            let step = 0.5f32.powi(level) * 0.25;
            cx += if px < cx { -step } else { step };
            cy += if py < cy { -step } else { step };
            level += 1;
        }
        level
    }
}
