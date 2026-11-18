//! Multi-scale Truchet mosaic.
//!
//! Two tile sets share one square quadtree: the **band set** (Carlson's 15
//! tiles, [`Tile`], [`Mosaic::types`] bits `1..=128`) and the **dot set** (22
//! tiles, [`RoundTile`], [`TYPE_ROUND`]). The band set is a stroke of half-width
//! `1/6` around a circle
//! of radius `1/2` centred on a corner of the unit square. Odd subdivision
//! levels swap ink and ground. A cell splits with [`Mosaic::split`], up to
//! [`Mosaic::levels`]. Where the fine tiles sit is whatever space the point
//! arrived in, so a log variation in front of this one can grade the detail.

/// Which tiles a mosaic may use. Bits of [`Mosaic::types`].
///
/// The full set is Carlson's 15 tiles. Every tile has the same ink profile along
/// each edge (the middle third), which is why any tiles join, at any scale.
/// Diagonal pair of arcs, one orientation.
pub const TYPE_DL: u32 = 1;
/// Diagonal pair of arcs, the other orientation.
pub const TYPE_DR: u32 = 2;
/// Straight bar, `-` and `|`.
pub const TYPE_BAR: u32 = 4;
/// Four dots and no strokes (`+.`).
pub const TYPE_DOTS: u32 = 8;
/// Cross (`+`).
pub const TYPE_CROSS: u32 = 16;
/// Four-leaf clover (`x.`): all four arms joined, with round inner corners.
pub const TYPE_CLOVER: u32 = 32;
/// T-junction, four turns.
pub const TYPE_T: u32 = 64;
/// One quarter arc plus two dots, four turns.
pub const TYPE_FILLET: u32 = 128;
/// Both diagonal arc tiles. Used when `types` is empty.
pub const TYPE_ARCS: u32 = TYPE_DL | TYPE_DR;
/// Carlson's fifteen. Unchanged, so old flames keep their layouts.
pub const TYPE_ALL: u32 = 255;
/// The dot set (see [`RoundTile`]). Its tiles have two ink dots per edge, at
/// 1/4 and 3/4, so they join each other but not Carlson's fifteen: this bit
/// replaces the other bits rather than adding to them.
pub const TYPE_ROUND: u32 = 256;

const BAND: f32 = 1.0 / 6.0;
const RADIUS: f32 = 0.5;

/// A seeded multi-scale Truchet mosaic. Build one, then ask [`Mosaic::ink`] whether a point is on ink.
#[derive(Clone, Debug)]
pub struct Mosaic {
    /// Selects the layout. The same seed and parameters always give the same mosaic.
    pub seed: u32,
    /// Width of one repeat, in top-level cells. Clamped to `1..=64`.
    pub width: f32,
    /// Height of one repeat, in top-level cells. Clamped to `1..=64`.
    pub height: f32,
    /// Deepest subdivision. `0` is a single tile per cell.
    pub levels: i32,
    /// Chance in `0..1` that a cell splits. Independent of position.
    pub split: f32,
    /// Bitmask of `TYPE_*`. Empty (or no known bit) means [`TYPE_ARCS`].
    pub types: u32,
    /// How far the split roll is nudged. `0` is a hard chance.
    pub noise: f32,
    /// Size gradient. `0` is off (cells split by `split`). Above `0`, cells
    /// shrink toward `y = 0`: a cell splits while its side is larger than
    /// `grade * |y|`, so tile size is proportional to height, as in the upper
    /// half plane. The mosaic then repeats in x only, not in y, and the lower
    /// half plane (`y <= 0`) is ground.
    pub grade: f32,
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
            grade: 0.0,
        }
    }
}

impl Mosaic {
    /// A copy with every field clamped into its valid range, as [`Mosaic::ink`] uses it.
    pub fn sanitized(&self) -> Self {
        let mut out = self.clone();
        out.width = out.width.clamp(1.0, 64.0);
        out.height = out.height.clamp(1.0, 64.0);
        out.levels = out.levels.clamp(0, 8);
        out.split = out.split.clamp(0.0, 1.0);
        out.noise = out.noise.max(0.0);
        out.grade = if out.grade.is_finite() {
            out.grade.clamp(0.0, 64.0)
        } else {
            0.0
        };
        out.types &= TYPE_ALL | TYPE_ROUND;
        if out.types & TYPE_ROUND != 0 {
            out.types = TYPE_ROUND;
        }
        if out.types == 0 {
            out.types = TYPE_ARCS;
        }
        out
    }

    /// `true` when `(x, y)` lands on ink. Coordinates wrap into the mosaic.
    ///
    /// Tiles are painted coarse to fine, Carlson style: each tile also paints
    /// "wings" past its square, a ground disc of radius `1/3` on every corner
    /// and an ink cap of radius `1/6` on every edge midpoint. Where a finer tile
    /// meets a coarser one its wings land on top, which is what joins the scales
    /// with round shapes instead of square notches.
    pub fn ink(&self, x: f32, y: f32) -> bool {
        let m = self.sanitized();
        // With a size gradient the mosaic is the upper half plane: no ink at or below y = 0.
        #[allow(clippy::neg_cmp_op_on_partial_ord)] // deliberate: NaN counts as ground
        if m.grade > 0.0 && !(y > 0.0) {
            return false;
        }
        let (px, py) = m.fold(x, y);
        let (cx, cy, level) = m.leaf(px, py);
        let scale = 0.5f32.powi(level);
        let u = (px - cx) / scale;
        let v = (py - cy) / scale;
        let wing = if m.is_round() {
            m.finer_wing_round(px, py, level, u, v)
        } else {
            m.finer_wing(px, py, level, u, v)
        };
        if let Some(wing) = wing {
            return wing;
        }
        let kind = m.tile_at(cx, cy, level);
        let band = on_band(kind, u, v);
        if level % 2 == 1 {
            !band
        } else {
            band
        }
    }

    /// Wrap a point into the mosaic. With a size gradient y is left alone.
    fn fold(&self, x: f32, y: f32) -> (f32, f32) {
        let y = if self.grade > 0.0 {
            if y.is_finite() {
                y
            } else {
                0.0
            }
        } else {
            wrap(y, self.height)
        };
        (wrap(x, self.width), y)
    }

    /// Centre and level of the leaf cell holding a wrapped point.
    fn leaf(&self, px: f32, py: f32) -> (f32, f32, i32) {
        let mut cx = px.floor() + 0.5;
        let mut cy = py.floor() + 0.5;
        let mut level = 0;
        while level < self.levels {
            if !self.splits(cx, cy, level) {
                break;
            }
            let step = 0.5f32.powi(level) * 0.25;
            cx += if px < cx { -step } else { step };
            cy += if py < cy { -step } else { step };
            level += 1;
        }
        (cx, cy, level)
    }

    /// `true` when the aligned cell of side `0.5^level` centred on `(x, y)`
    /// is itself a leaf (not split further, not part of a coarser leaf).
    fn is_leaf(&self, x: f32, y: f32, level: i32) -> bool {
        let (fx, fy) = self.fold(x, y);
        let (_, _, found) = self.leaf(fx, fy);
        found == level
    }

    /// Colour painted here by a finer neighbour's wing, if any. Only wings of
    /// leaves finer than the one holding the point can show (coarser ones are
    /// painted over, same-level ones already agree with this tile).
    fn finer_wing(&self, px: f32, py: f32, level: i32, u: f32, v: f32) -> Option<bool> {
        if level >= self.levels {
            return None;
        }
        // The widest wing of the next level reaches 1/6 of this tile past its edge.
        let edge = 0.5 - u.abs().max(v.abs());
        if edge >= 1.0 / 6.0 {
            return None;
        }
        for k in (level + 1..=self.levels).rev() {
            let s = 0.5f32.powi(k);
            let ground = k % 2 == 1;
            let ink = !ground;
            // Corner wings: ground discs of radius s/3 on the lattice corners.
            let gx = (px / s).round() * s;
            let gy = (py / s).round() * s;
            if (px - gx).hypot(py - gy) < s / 3.0 {
                let h = s * 0.5;
                if [(-h, -h), (h, -h), (-h, h), (h, h)]
                    .into_iter()
                    .any(|(dx, dy)| self.is_leaf(gx + dx, gy + dy, k))
                {
                    return Some(ground);
                }
            }
            // Edge caps: ink discs of radius s/6 on edge midpoints.
            let hx = ((px / s - 0.5).round() + 0.5) * s; // horizontal edge
            if (px - hx).hypot(py - gy) < s / 6.0
                && (self.is_leaf(hx, gy - s * 0.5, k) || self.is_leaf(hx, gy + s * 0.5, k))
            {
                return Some(ink);
            }
            let vy = ((py / s - 0.5).round() + 0.5) * s; // vertical edge
            if (px - gx).hypot(py - vy) < s / 6.0
                && (self.is_leaf(gx - s * 0.5, vy, k) || self.is_leaf(gx + s * 0.5, vy, k))
            {
                return Some(ink);
            }
        }
        None
    }

    /// Wings for the round family. A tile's ground circles (radius `s/6`, on the
    /// corners and edge midpoints) and its port dots (radius `s/12`, at 1/4 and
    /// 3/4 of each edge) overhang its square, finer over coarser.
    fn finer_wing_round(&self, px: f32, py: f32, level: i32, u: f32, v: f32) -> Option<bool> {
        if level >= self.levels {
            return None;
        }
        if 0.5 - u.abs().max(v.abs()) >= 1.0 / 12.0 {
            return None;
        }
        for k in (level + 1..=self.levels).rev() {
            let s = 0.5f32.powi(k);
            let odd = k % 2 == 1;
            let h = s * 0.5;
            // Ground circles: lattice points of spacing s/2 that are a corner or an edge midpoint.
            let (ix, iy) = ((px / h).round(), (py / h).round());
            let (gx, gy) = (ix * h, iy * h);
            if (px - gx).hypot(py - gy) < s / 6.0 {
                let (ox, oy) = ((ix as i32).rem_euclid(2), (iy as i32).rem_euclid(2));
                let touching: &[(f32, f32)] = match (ox, oy) {
                    (0, 0) => &[(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)],
                    (1, 0) => &[(0.0, -1.0), (0.0, 1.0)],
                    (0, 1) => &[(-1.0, 0.0), (1.0, 0.0)],
                    _ => &[],
                };
                if touching
                    .iter()
                    .any(|&(dx, dy)| self.is_leaf(gx + dx * h, gy + dy * h, k))
                {
                    return Some(odd);
                }
            }
            // Port dots: lattice of spacing s/4, on a cell edge and at 1/4 or 3/4 along it.
            let q = s * 0.25;
            let (jx, jy) = ((px / q).round(), (py / q).round());
            let (dxp, dyp) = (jx * q, jy * q);
            if (px - dxp).hypot(py - dyp) < s / 12.0 {
                let (mx, my) = ((jx as i32).rem_euclid(4), (jy as i32).rem_euclid(4));
                let touching: &[(f32, f32)] = if my == 0 && (mx == 1 || mx == 3) {
                    &[(0.0, -1.0), (0.0, 1.0)]
                } else if mx == 0 && (my == 1 || my == 3) {
                    &[(-1.0, 0.0), (1.0, 0.0)]
                } else {
                    &[]
                };
                if touching
                    .iter()
                    .any(|&(dx, dy)| self.is_leaf(dxp + dx * h, dyp + dy * h, k))
                {
                    return Some(!odd);
                }
            }
        }
        None
    }

    fn splits(&self, cx: f32, cy: f32, level: i32) -> bool {
        if level >= self.levels {
            return false;
        }
        let roll = unit(mix(self.seed, quant(cx), quant(cy), level, 1));
        if self.grade > 0.0 {
            let target = self.grade * cy.abs() * (1.0 + (roll - 0.5) * self.noise);
            return 0.5f32.powi(level) > target;
        }
        let nudged = if self.noise == 0.0 {
            roll
        } else {
            (roll + gaussian(self.seed, quant(cx), quant(cy), level, 3) * self.noise)
                .clamp(0.0, 1.0)
        };
        nudged < self.split
    }

    fn tile_at(&self, cx: f32, cy: f32, level: i32) -> Kind {
        let choices = self.kinds();
        let h = mix(self.seed, quant(cx), quant(cy), level, 2);
        choices[(h as usize) % choices.len()]
    }

    fn is_round(&self) -> bool {
        self.types & TYPE_ROUND != 0
    }

    /// Every tile the mosaic may place, with its orientation.
    fn kinds(&self) -> Vec<Kind> {
        if self.is_round() {
            RoundTile::ALL
                .iter()
                .flat_map(|&t| (0..t.turns()).map(move |r| Kind::Round(t, r)))
                .collect()
        } else {
            self.choices().into_iter().map(Kind::Carlson).collect()
        }
    }

    fn choices(&self) -> Vec<Tile> {
        // Order is part of the seed contract: the two arc tiles stay first, so
        // arcs-only mosaics keep exactly the layouts they had.
        let mut out: Vec<Tile> = Tile::ALL
            .iter()
            .copied()
            .filter(|t| self.types & t.bit() != 0)
            .collect();
        if out.is_empty() {
            out = vec![Tile::Dl, Tile::Dr];
        }
        out
    }
}

/// Map one chaos point. `None` is ground (the host should drop the sample).
/// Ink is returned centred on the mosaic and scaled by `weight`.
pub fn map_point(
    mosaic: &Mosaic,
    x: f32,
    y: f32,
    z: f32,
    weight: f32,
    scale: f32,
) -> Option<[f32; 3]> {
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

/// Filter mode: keep the incoming point where the mosaic has ink, drop it
/// elsewhere. The point is returned unmoved (times `weight`), so whatever was
/// applied before this variation survives, with the tiles eating into it.
/// `scale` sizes the tiles in the incoming space: one coarse cell is `1/scale`.
pub fn filter_point(
    mosaic: &Mosaic,
    x: f32,
    y: f32,
    z: f32,
    weight: f32,
    scale: f32,
) -> Option<[f32; 3]> {
    let m = mosaic.sanitized();
    if !(x.is_finite() && y.is_finite() && m.ink(x * scale, y * scale)) {
        return None;
    }
    Some([x * weight, y * weight, z * weight])
}

/// Snap mode: keep the incoming point if it is on ink, otherwise try up to 24
/// jittered copies within `radius` (incoming units) and return the first that is.
/// Unlike [`filter_point`] a miss usually still yields a point, so hosts that
/// restart the orbit on `None` do not thrash. Points are pulled to nearby ink,
/// so this is a projection, not a strict filter: ground is thinned, not holed.
#[allow(clippy::too_many_arguments)]
pub fn snap_point(
    mosaic: &Mosaic,
    x: f32,
    y: f32,
    z: f32,
    weight: f32,
    scale: f32,
    radius: f32,
    mut unit: impl FnMut() -> f32,
) -> Option<[f32; 3]> {
    let m = mosaic.sanitized();
    if !(x.is_finite() && y.is_finite()) {
        return None;
    }
    if m.ink(x * scale, y * scale) {
        return Some([x * weight, y * weight, z * weight]);
    }
    let r = if radius.is_finite() {
        radius.max(0.0)
    } else {
        0.0
    };
    for _ in 0..24 {
        let jx = x + (unit() * 2.0 - 1.0) * r;
        let jy = y + (unit() * 2.0 - 1.0) * r;
        if m.ink(jx * scale, jy * scale) {
            return Some([jx * weight, jy * weight, z * weight]);
        }
    }
    None
}

/// Throw fresh mosaic points until one lands on ink. Keeps strokes filled when
/// the orbit would otherwise stick to the first hit.
pub fn sample_ink(
    mosaic: &Mosaic,
    z: f32,
    weight: f32,
    scale: f32,
    mut unit: impl FnMut() -> f32,
) -> Option<[f32; 3]> {
    let m = mosaic.sanitized();
    for _ in 0..24 {
        let x = unit() * m.width;
        let y = unit() * m.height;
        if let Some(point) = map_point(&m, x, y, z, weight, scale) {
            return Some(point);
        }
    }
    None
}

/// Hyperbolic (Poincare disk) generator: draw a point uniformly in the unit disk, pull it
/// back to the upper half plane with the inverse Cayley map z = i(1+w)/(1-w), keep it if it
/// lands on ink (with `grade` > 0), and return the disk point `w`. Sampling in the disk
/// keeps the rim populated, which uniform sampling of the half plane would not.
pub fn disk_sample(
    mosaic: &Mosaic,
    weight: f32,
    mut unit: impl FnMut() -> f32,
) -> Option<[f32; 3]> {
    let m = mosaic.sanitized();
    for _ in 0..64 {
        let r = unit().sqrt();
        let t = unit() * std::f32::consts::TAU;
        let (wr, wi) = (r * t.cos(), r * t.sin());
        let (nr, ni, dr, di) = (1.0 + wr, wi, 1.0 - wr, -wi);
        let dd = dr * dr + di * di;
        if dd < 1e-9 {
            continue;
        }
        let qr = (nr * dr + ni * di) / dd;
        let qi = (ni * dr - nr * di) / dd;
        if m.ink(-qi, qr) {
            return Some([wr * weight, wi * weight, 0.0]);
        }
    }
    None
}

/// Star generator: a uniform point inside an `points`-pointed star of outer radius 1
/// (tips at angles 2*pi*m/points, inner vertices at radius `inner` between them).
pub fn star_sample(
    points: f32,
    inner: f32,
    weight: f32,
    mut unit: impl FnMut() -> f32,
) -> Option<[f32; 3]> {
    let n = points.round().clamp(2.0, 64.0);
    let k = if inner.is_finite() {
        inner.clamp(0.02, 0.98)
    } else {
        0.5
    };
    let half = std::f32::consts::PI / n;
    let (ix, iy) = (k * half.cos(), k * half.sin());
    for _ in 0..64 {
        let r = unit().sqrt();
        let t = unit() * std::f32::consts::TAU;
        let mut psi = t.rem_euclid(2.0 * half);
        if psi > half {
            psi = 2.0 * half - psi;
        }
        let boundary = iy / (iy * psi.cos() + (1.0 - ix) * psi.sin());
        if r <= boundary {
            return Some([r * t.cos() * weight, r * t.sin() * weight, 0.0]);
        }
    }
    None
}

/// Keep the incoming point only if it lies outside the circle of radius `cut` about the
/// origin (used to clip a packing so a central disk can sit on top of it).
pub fn keep_outside(x: f32, y: f32, z: f32, weight: f32, cut: f32) -> Option<[f32; 3]> {
    if !(x.is_finite() && y.is_finite()) || x * x + y * y < cut * cut {
        return None;
    }
    Some([x * weight, y * weight, z * weight])
}

/// One of Carlson's 15 tiles, in the unit square centred on the origin
/// (`u`, `v` in `-0.5..=0.5`, `v` up).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tile {
    /// Two arcs joining adjacent edges, one diagonal.
    Dl,
    /// Two arcs joining adjacent edges, the other diagonal.
    Dr,
    /// `-`
    H,
    /// `|`
    V,
    /// `+.`
    Dots,
    /// `+`
    Cross,
    /// `x.`
    Clover,
    /// T-junctions, named for the way the stem points.
    Tn,
    #[allow(missing_docs)]
    Te,
    #[allow(missing_docs)]
    Ts,
    #[allow(missing_docs)]
    Tw,
    /// One arc round the named corner, plus a dot on each of the other two edges.
    Fnw,
    #[allow(missing_docs)]
    Fne,
    #[allow(missing_docs)]
    Fsw,
    #[allow(missing_docs)]
    Fse,
}

/// Radius of the dot at each edge midpoint; also half the stroke width.
const DOT: f32 = BAND;
/// Radius of the round corners of the clover and the T-junctions.
const FILLET: f32 = 1.0 / 3.0;

impl Tile {
    /// All 15 tiles.
    pub const ALL: [Tile; 15] = [
        Tile::Dl,
        Tile::Dr,
        Tile::H,
        Tile::V,
        Tile::Dots,
        Tile::Cross,
        Tile::Clover,
        Tile::Tn,
        Tile::Te,
        Tile::Ts,
        Tile::Tw,
        Tile::Fnw,
        Tile::Fne,
        Tile::Fsw,
        Tile::Fse,
    ];

    /// The `TYPE_*` bit that enables this tile.
    pub fn bit(self) -> u32 {
        match self {
            Tile::Dl => TYPE_DL,
            Tile::Dr => TYPE_DR,
            Tile::H | Tile::V => TYPE_BAR,
            Tile::Dots => TYPE_DOTS,
            Tile::Cross => TYPE_CROSS,
            Tile::Clover => TYPE_CLOVER,
            Tile::Tn | Tile::Te | Tile::Ts | Tile::Tw => TYPE_T,
            Tile::Fnw | Tile::Fne | Tile::Fsw | Tile::Fse => TYPE_FILLET,
        }
    }

    /// `true` when `(u, v)` is ink. Outside the unit square is always ground.
    pub fn ink(self, u: f32, v: f32) -> bool {
        if u.abs() > 0.5 || v.abs() > 0.5 {
            return false;
        }
        // Every tile has a dot on every edge midpoint, so edges always meet.
        if [(0.0, 0.5), (0.0, -0.5), (0.5, 0.0), (-0.5, 0.0)]
            .into_iter()
            .any(|(px, py)| (u - px).hypot(v - py) < DOT)
        {
            return true;
        }
        let arc = |cx: f32, cy: f32| ((u - cx).hypot(v - cy) - RADIUS).abs() < BAND;
        let disc = |cx: f32, cy: f32| (u - cx).hypot(v - cy) < FILLET;
        match self {
            Tile::Dl => arc(-0.5, -0.5) || arc(0.5, 0.5),
            Tile::Dr => arc(-0.5, 0.5) || arc(0.5, -0.5),
            Tile::H => v.abs() < BAND,
            Tile::V => u.abs() < BAND,
            Tile::Dots => false,
            Tile::Cross => u.abs() < BAND || v.abs() < BAND,
            Tile::Clover => {
                !(disc(-0.5, -0.5) || disc(0.5, -0.5) || disc(-0.5, 0.5) || disc(0.5, 0.5))
            }
            Tile::Tn => v > -BAND && !(disc(-0.5, 0.5) || disc(0.5, 0.5)),
            Tile::Te => u > -BAND && !(disc(0.5, 0.5) || disc(0.5, -0.5)),
            Tile::Ts => v < BAND && !(disc(-0.5, -0.5) || disc(0.5, -0.5)),
            Tile::Tw => u < BAND && !(disc(-0.5, 0.5) || disc(-0.5, -0.5)),
            Tile::Fnw => arc(-0.5, 0.5),
            Tile::Fne => arc(0.5, 0.5),
            Tile::Fsw => arc(-0.5, -0.5),
            Tile::Fse => arc(0.5, -0.5),
        }
    }
}

/// A placed tile: one of Carlson's fifteen, or a round tile with its turns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Carlson(Tile),
    Round(RoundTile, u8),
}

fn on_band(kind: Kind, u: f32, v: f32) -> bool {
    match kind {
        Kind::Carlson(t) => t.ink(u, v),
        Kind::Round(t, r) => t.ink(u, v, r),
    }
}

fn wrap(value: f32, span: f32) -> f32 {
    if !value.is_finite() || span <= 0.0 {
        return 0.0;
    }
    let mut m = value % span;
    if m < 0.0 {
        m += span;
    }
    if m == span {
        0.0
    } else {
        m
    }
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

/// `{p,q}` disk map. `n` selects the p-fold turn.
///
/// `c` sits at radius `1/sqrt(1 + (1-cos pa)/(cos pa + cos qa))` and angle
/// `n * 2π/p`. The point moves by `(z + conj(c)) / (1 + c z)`.
pub fn disk_map(x: f32, y: f32, p: f32, q: f32, n: f32, weight: f32) -> Option<[f32; 2]> {
    let p = p.round().clamp(3.0, 1.0e6);
    let q = q.round().clamp(3.0, 1.0e6);
    let n = n.round().max(0.0);
    let pa = std::f32::consts::TAU / p;
    let qa = std::f32::consts::TAU / q;
    let denom = pa.cos() + qa.cos();
    if denom.abs() < 1e-6 {
        return None;
    }
    let mut radius = (1.0 - pa.cos()) / denom + 1.0;
    radius = if radius > 0.0 {
        radius.recip().sqrt()
    } else {
        1.0
    };
    let angle = n * pa;
    let re = radius * angle.cos();
    let im = radius * angle.sin();
    let num_re = x + re;
    let num_im = y - im;
    let den_re = re * x - im * y + 1.0;
    let den_im = re * y + im * x;
    let mag = den_re * den_re + den_im * den_im;
    if mag < 1e-12 {
        return None;
    }
    let scale = weight / mag;
    Some([
        scale * (num_re * den_re + num_im * den_im),
        scale * (num_im * den_re - num_re * den_im),
    ])
}

fn gaussian(seed: u32, x: i32, y: i32, level: i32, salt: u32) -> f32 {
    let u = unit(mix(seed, x, y, level, salt)).clamp(1e-4, 1.0 - 1e-4);
    let v = unit(mix(seed, x, y, level, salt.wrapping_add(17)));
    let radius = (-2.0 * u.ln()).sqrt();
    radius * (std::f32::consts::TAU * v).cos()
}

/// The round tile family. Every edge has two ink ports, at 1/4 and 3/4 of its
/// length, drawn as dots of radius 1/12. A ground circle of radius 1/6 sits on
/// each corner and each edge midpoint, so the edge reads dot, ground, dot with
/// no flat spans, and every stroke is a 1/6-wide ring around one of those
/// circles (radius 1/4, or 3/4 around a corner), a straight bar, or a fill
/// with the ground circles cut out of it. Tiles never end in a square corner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoundTile {
    /// No ink.
    Empty,
    /// A single ink dot at the centre.
    Empty1,
    /// Only the edge dots.
    Dotted,
    /// A small ring around each corner.
    CornerHash,
    /// Small rings on the top two corners and a half ring on the bottom edge.
    SadFace,
    /// Half rings on the top and bottom edges.
    HourGlass,
    /// A half ring on every edge.
    EdgeHash,
    /// Half rings on three edges.
    Edge34,
    /// Small rings on the top two corners and a bar.
    Frog,
    /// Small and large rings around one corner.
    Slash2,
    /// Small rings on two opposite corners and a large ring on one.
    Slash21,
    /// A small ring on one corner and a large ring on the opposite one.
    Slash11,
    /// A small ring on one corner and a horizontal and a vertical bar.
    CrossCross,
    /// Solid ink, minus the ground circles.
    Filled,
    /// Solid ink with the centre cut out.
    FilledHollow,
    /// A filled band across the middle.
    Filled12,
    /// A filled band across the middle with the centre cut out.
    Filled12Hollow,
    /// Ink over the upper part of the tile.
    Filled34,
    /// Ink over the upper part of the tile with the centre cut out.
    Filled34Hollow,
    /// A filled strip along the bottom.
    Filled13,
    /// A filled strip along the bottom plus a bar.
    Filled13Bar,
    /// Ink around one corner, as a large quarter disc.
    Cowboy,
}

#[derive(Clone, Copy)]
enum Fill {
    None,
    /// The whole square.
    Full,
    /// The band `1/6 <= y <= 5/6`.
    Band,
    /// Everything above `y = 5/6`.
    Upper,
    /// The strip below `y = 1/6`.
    Strip,
    /// The disc of radius 5/6 around the top-left corner.
    Quarter,
}

#[derive(Clone, Copy)]
struct Spec {
    /// Corner bitmasks (TL, TR, BR, BL): rings of radius 1/4 and 3/4.
    small: u8,
    big: u8,
    /// Half rings of radius 1/4 on the edge midpoints (T, R, B, L).
    semi: u8,
    /// Straight bars at 1/4 (bit 0) and 3/4 (bit 1).
    hbar: u8,
    vbar: u8,
    fill: Fill,
    hole: bool,
    centre: bool,
    dotted: bool,
    /// Distinct orientations (1, 2 or 4).
    turns: u8,
}

const ZERO: Spec = Spec {
    small: 0,
    big: 0,
    semi: 0,
    hbar: 0,
    vbar: 0,
    fill: Fill::None,
    hole: false,
    centre: false,
    dotted: false,
    turns: 4,
};

const PORT: f32 = 1.0 / 12.0;
const GROUND: f32 = 1.0 / 6.0;

impl RoundTile {
    /// All 22 tiles.
    pub const ALL: [RoundTile; 22] = [
        RoundTile::Empty,
        RoundTile::Empty1,
        RoundTile::Dotted,
        RoundTile::CornerHash,
        RoundTile::SadFace,
        RoundTile::HourGlass,
        RoundTile::EdgeHash,
        RoundTile::Edge34,
        RoundTile::Frog,
        RoundTile::Slash2,
        RoundTile::Slash21,
        RoundTile::Slash11,
        RoundTile::CrossCross,
        RoundTile::Filled,
        RoundTile::FilledHollow,
        RoundTile::Filled12,
        RoundTile::Filled12Hollow,
        RoundTile::Filled34,
        RoundTile::Filled34Hollow,
        RoundTile::Filled13,
        RoundTile::Filled13Bar,
        RoundTile::Cowboy,
    ];

    fn spec(self) -> Spec {
        use RoundTile::*;
        match self {
            Empty => Spec { turns: 1, ..ZERO },
            Empty1 => Spec {
                centre: true,
                turns: 1,
                ..ZERO
            },
            Dotted => Spec {
                dotted: true,
                turns: 1,
                ..ZERO
            },
            CornerHash => Spec {
                small: 0b1111,
                turns: 1,
                ..ZERO
            },
            SadFace => Spec {
                small: 0b0011,
                semi: 0b0100,
                ..ZERO
            },
            HourGlass => Spec {
                semi: 0b0101,
                turns: 2,
                ..ZERO
            },
            EdgeHash => Spec {
                semi: 0b1111,
                turns: 1,
                ..ZERO
            },
            Edge34 => Spec {
                semi: 0b0111,
                ..ZERO
            },
            Frog => Spec {
                small: 0b0011,
                hbar: 0b10,
                ..ZERO
            },
            Slash2 => Spec {
                small: 0b0001,
                big: 0b0001,
                ..ZERO
            },
            Slash21 => Spec {
                small: 0b0101,
                big: 0b0001,
                ..ZERO
            },
            Slash11 => Spec {
                small: 0b0100,
                big: 0b0001,
                ..ZERO
            },
            CrossCross => Spec {
                small: 0b0001,
                hbar: 0b10,
                vbar: 0b10,
                ..ZERO
            },
            Filled => Spec {
                fill: Fill::Full,
                turns: 1,
                ..ZERO
            },
            FilledHollow => Spec {
                fill: Fill::Full,
                hole: true,
                turns: 1,
                ..ZERO
            },
            Filled12 => Spec {
                fill: Fill::Band,
                turns: 2,
                ..ZERO
            },
            Filled12Hollow => Spec {
                fill: Fill::Band,
                hole: true,
                turns: 2,
                ..ZERO
            },
            Filled34 => Spec {
                fill: Fill::Upper,
                ..ZERO
            },
            Filled34Hollow => Spec {
                fill: Fill::Upper,
                hole: true,
                ..ZERO
            },
            Filled13 => Spec {
                fill: Fill::Strip,
                ..ZERO
            },
            Filled13Bar => Spec {
                fill: Fill::Strip,
                hbar: 0b10,
                ..ZERO
            },
            Cowboy => Spec {
                fill: Fill::Quarter,
                ..ZERO
            },
        }
    }

    /// How many distinct orientations the tile has.
    pub fn turns(self) -> u8 {
        self.spec().turns
    }

    /// `true` when `(u, v)` (centred on the tile, `v` up) is ink, after `rot`
    /// quarter turns. Outside the unit square is ground.
    pub fn ink(self, u: f32, v: f32, rot: u8) -> bool {
        let (mut u, mut v) = (u, v);
        for _ in 0..rot % 4 {
            let t = u;
            u = v;
            v = -t;
        }
        let (x, y) = (u + 0.5, 0.5 - v);
        if !(0.0..=1.0).contains(&x) || !(0.0..=1.0).contains(&y) {
            return false;
        }
        let s = self.spec();
        let d = |cx: f32, cy: f32| (x - cx).hypot(y - cy);
        for k in [0.25f32, 0.75] {
            if d(k, 0.0) < PORT || d(k, 1.0) < PORT || d(0.0, k) < PORT || d(1.0, k) < PORT {
                return true;
            }
        }
        if s.centre && d(0.5, 0.5) < PORT {
            return true;
        }
        if s.dotted
            && [(0.5, 0.25), (0.25, 0.5), (0.75, 0.5), (0.5, 0.75)]
                .iter()
                .any(|&(a, b)| d(a, b) < PORT)
        {
            return true;
        }
        let corners = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
        let mids = [(0.5, 0.0), (1.0, 0.5), (0.5, 1.0), (0.0, 0.5)];
        for i in 0..4 {
            let on = |mask: u8| mask >> i & 1 == 1;
            let (cx, cy) = corners[i];
            let (mx, my) = mids[i];
            if (on(s.small) && (d(cx, cy) - 0.25).abs() < PORT)
                || (on(s.big) && (d(cx, cy) - 0.75).abs() < PORT)
                || (on(s.semi) && (d(mx, my) - 0.25).abs() < PORT)
            {
                return true;
            }
        }
        if (s.hbar & 1 != 0 && (y - 0.25).abs() < PORT)
            || (s.hbar & 2 != 0 && (y - 0.75).abs() < PORT)
            || (s.vbar & 1 != 0 && (x - 0.25).abs() < PORT)
            || (s.vbar & 2 != 0 && (x - 0.75).abs() < PORT)
        {
            return true;
        }
        let inside = match s.fill {
            Fill::None => false,
            Fill::Full => true,
            Fill::Band => (1.0 / 6.0..=5.0 / 6.0).contains(&y),
            Fill::Upper => y <= 5.0 / 6.0,
            Fill::Strip => y <= 1.0 / 6.0,
            Fill::Quarter => d(0.0, 0.0) <= 5.0 / 6.0,
        };
        if !inside {
            return false;
        }
        if corners
            .iter()
            .chain(mids.iter())
            .any(|&(a, b)| d(a, b) < GROUND)
        {
            return false;
        }
        !(s.hole && d(0.5, 0.5) < GROUND)
    }
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
            grade: 0.0,
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
    fn disk_map_keeps_the_open_disk_and_turns_with_n() {
        let inside = disk_map(0.2, -0.4, 3.0, 7.0, 1.0, 1.0).expect("finite");
        assert!(inside[0].hypot(inside[1]) < 1.0, "{inside:?}");
        let origin = disk_map(0.0, 0.0, 3.0, 7.0, 0.0, 1.0).expect("origin");
        let turned = disk_map(0.0, 0.0, 3.0, 7.0, 1.0, 1.0).expect("turned");
        let cross = origin[0] * turned[1] - origin[1] * turned[0];
        assert!(
            cross.abs() > 0.05,
            "n should rotate the image of the origin"
        );
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

    #[test]
    fn every_tile_has_the_same_ink_on_every_edge() {
        // Ink along an edge is exactly the middle third, for all 15 tiles: that is
        // what lets any two tiles (and any two scales) meet without a seam.
        let eps = 1e-3;
        for tile in Tile::ALL {
            for i in 0..=100 {
                let t = -0.5 + i as f32 / 100.0;
                if (t.abs() - BAND).abs() < 2e-3 {
                    continue; // exactly on the boundary of the middle third
                }
                let want = t.abs() < BAND;
                for (u, v) in [
                    (t, 0.5 - eps),
                    (t, -0.5 + eps),
                    (0.5 - eps, t),
                    (-0.5 + eps, t),
                ] {
                    assert_eq!(tile.ink(u, v), want, "{tile:?} at ({u}, {v})");
                }
            }
        }
    }

    #[test]
    fn tile_set_is_carlsons_fifteen() {
        assert_eq!(Tile::ALL.len(), 15);
        let mut bits = 0;
        for tile in Tile::ALL {
            bits |= tile.bit();
        }
        assert_eq!(bits, TYPE_ALL);
    }

    #[test]
    fn arcs_stay_first_so_old_seeds_keep_their_layouts() {
        let arcs = Mosaic::default();
        assert_eq!(arcs.sanitized().choices(), vec![Tile::Dl, Tile::Dr]);
        let all = Mosaic {
            types: TYPE_ALL,
            ..Mosaic::default()
        };
        assert_eq!(&all.sanitized().choices()[..2], &[Tile::Dl, Tile::Dr]);
        assert_eq!(all.sanitized().choices().len(), 15);
    }

    #[test]
    fn empty_or_unknown_types_mean_arcs_and_other_bits_are_honoured() {
        for types in [0, 1 << 12, 1 << 20] {
            assert_eq!(
                Mosaic {
                    types,
                    ..Mosaic::default()
                }
                .sanitized()
                .types,
                TYPE_ARCS
            );
        }
        let bars = Mosaic {
            types: TYPE_BAR,
            ..Mosaic::default()
        }
        .sanitized();
        assert_eq!(bars.choices(), vec![Tile::H, Tile::V]);
    }

    #[test]
    fn clover_is_ink_in_the_middle_and_ground_in_the_corners() {
        assert!(Tile::Clover.ink(0.0, 0.0));
        assert!(Tile::Clover.ink(0.0, 0.45)); // arm
        assert!(!Tile::Clover.ink(0.45, 0.45)); // corner
        assert!(Tile::Dots.ink(0.0, 0.5 - 0.01) && !Tile::Dots.ink(0.0, 0.0));
        assert!(
            Tile::Cross.ink(0.0, 0.3) && Tile::Cross.ink(0.3, 0.0) && !Tile::Cross.ink(0.3, 0.3)
        );
    }

    #[test]
    fn finer_tiles_round_into_coarser_neighbours() {
        // Find a level-0 leaf whose right neighbour cell is split, then probe just
        // past the shared edge, inside the coarse tile, at a fine tile's edge
        // midpoint and at its corner: the cap is fine ink, the wing fine ground.
        let mut checked = 0;
        for seed in 1..200 {
            let m = Mosaic {
                seed,
                levels: 1,
                split: 0.5,
                noise: 0.0,
                ..Mosaic::default()
            }
            .sanitized();
            for cx in 0..5 {
                for cy in 0..6 {
                    let (x, y) = (cx as f32 + 0.5, cy as f32 + 0.5);
                    if m.leaf(x, y).2 != 0 || m.leaf(x + 1.0, y).2 != 1 {
                        continue;
                    }
                    let edge = cx as f32 + 1.0;
                    let fine = 0.5f32;
                    // Cap of the lower-left child of the split cell: midpoint of its left edge.
                    let cap_y = cy as f32 + fine * 0.5;
                    assert!(
                        !m.ink(edge - 0.02, cap_y),
                        "level-1 cap should paint ground-colour ink (white)"
                    );
                    // Corner wing of that child at the shared corner: level-1 ground is ink.
                    assert!(
                        m.ink(edge - 0.02, cy as f32 + 0.02),
                        "level-1 corner wing should be ink"
                    );
                    checked += 1;
                }
            }
        }
        assert!(checked > 10);
    }

    #[test]
    fn grade_makes_tiles_smaller_toward_y_zero_and_keeps_x_periodic() {
        let m = Mosaic {
            levels: 6,
            grade: 0.1,
            noise: 0.0,
            ..Mosaic::default()
        }
        .sanitized();
        let size = |y: f32| 0.5f32.powi(m.leaf(0.3, y).2);
        assert!(
            size(0.2) < size(2.0) && size(2.0) < size(5.5),
            "{} {} {}",
            size(0.2),
            size(2.0),
            size(5.5)
        );
        assert_eq!(m.ink(0.3, 2.2), m.ink(0.3 + m.width, 2.2));
    }

    #[test]
    fn filter_keeps_the_incoming_point_only_on_ink() {
        let m = Mosaic::default();
        let (mut kept, mut dropped) = (0, 0);
        for i in 0..400 {
            let (x, y) = (i as f32 * 0.137 - 20.0, (i as f32 * 0.071).sin() * 9.0);
            match filter_point(&m, x, y, 0.25, 0.5, 1.0) {
                Some(p) => {
                    kept += 1;
                    assert_eq!(p, [x * 0.5, y * 0.5, 0.125]);
                }
                None => dropped += 1,
            }
        }
        assert!(kept > 50 && dropped > 50);
    }

    #[test]
    fn snap_lands_on_ink_and_keeps_points_that_already_are() {
        let m = Mosaic::default();
        let mut state = 12345u32;
        let mut rnd = || {
            state = state.wrapping_mul(1664525).wrapping_add(1013904223);
            (state >> 8) as f32 / 16777216.0
        };
        let (mut moved, mut kept, mut none) = (0, 0, 0);
        for i in 0..500 {
            let (x, y) = (i as f32 * 0.113 - 25.0, (i as f32 * 0.057).cos() * 8.0);
            match snap_point(&m, x, y, 0.0, 1.0, 1.0, 0.4, &mut rnd) {
                Some(p) => {
                    assert!(m.ink(p[0], p[1]), "snapped point must be ink");
                    if p[0] == x && p[1] == y {
                        kept += 1
                    } else {
                        moved += 1
                    }
                }
                None => none += 1,
            }
        }
        assert!(
            kept > 100 && moved > 100 && none < 50,
            "{kept} {moved} {none}"
        );
    }

    #[test]
    fn with_a_gradient_the_lower_half_plane_is_ground() {
        let m = Mosaic {
            grade: 0.3,
            levels: 6,
            ..Mosaic::default()
        };
        for i in 0..200 {
            let x = i as f32 * 0.37 - 30.0;
            assert!(!m.ink(x, 0.0) && !m.ink(x, -0.001 - i as f32 * 0.1));
        }
        assert!((0..200).any(|i| m.ink(i as f32 * 0.37, 0.5 + (i % 7) as f32 * 0.4)));
    }

    #[test]
    fn disk_sample_stays_in_disk_and_on_ink() {
        let m = Mosaic {
            grade: 0.5,
            levels: 6,
            types: 255,
            ..Mosaic::default()
        };
        let mut st = 12345u32;
        let mut u = || {
            st ^= st << 13;
            st ^= st >> 17;
            st ^= st << 5;
            (st >> 8) as f32 / 16777216.0
        };
        let mut n = 0;
        for _ in 0..2000 {
            if let Some(p) = disk_sample(&m, 1.0, &mut u) {
                assert!(p[0] * p[0] + p[1] * p[1] <= 1.0001);
                n += 1;
            }
        }
        assert!(n > 1900, "hit {n}");
    }

    #[test]
    fn star_stays_inside_star() {
        let mut st = 777u32;
        let mut u = || {
            st ^= st << 13;
            st ^= st >> 17;
            st ^= st << 5;
            (st >> 8) as f32 / 16777216.0
        };
        for _ in 0..2000 {
            let p = star_sample(4.0, 0.4, 1.0, &mut u).unwrap();
            let r = (p[0] * p[0] + p[1] * p[1]).sqrt();
            assert!(r <= 1.0001);
            // along a tip direction (x axis) radius may reach 1; along 45 degrees it must be <= inner-ish
            let a = p[1].atan2(p[0]).rem_euclid(std::f32::consts::FRAC_PI_2);
            if (a - std::f32::consts::FRAC_PI_4).abs() < 0.02 {
                assert!(r <= 0.41, "r {r}");
            }
        }
    }

    #[test]
    fn keep_outside_clips() {
        assert!(keep_outside(0.1, 0.1, 0.0, 1.0, 0.5).is_none());
        assert!(keep_outside(0.9, 0.0, 0.0, 1.0, 0.5).is_some());
        assert!(keep_outside(f32::NAN, 0.0, 0.0, 1.0, 0.5).is_none());
    }

    #[test]
    fn round_tiles_show_two_dots_on_every_edge() {
        // Along every edge the ink is exactly the two port dots, at 1/4 and 3/4:
        // that is what lets round tiles join at any turn.
        let eps = 1e-3;
        for tile in RoundTile::ALL {
            for rot in 0..tile.turns() {
                for i in 0..=120 {
                    let t = -0.5 + i as f32 / 120.0;
                    let a = t.abs();
                    if (a - 1.0 / 6.0).abs() < 4e-3 || (a - 1.0 / 3.0).abs() < 4e-3 {
                        continue;
                    }
                    let want = a > 1.0 / 6.0 && a < 1.0 / 3.0;
                    for (u, v) in [
                        (t, 0.5 - eps),
                        (t, -0.5 + eps),
                        (0.5 - eps, t),
                        (-0.5 + eps, t),
                    ] {
                        assert_eq!(
                            tile.ink(u, v, rot),
                            want,
                            "{tile:?} rot {rot} at ({u}, {v})"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn round_flag_replaces_the_other_bits_and_old_masks_are_untouched() {
        let m = Mosaic {
            types: TYPE_ROUND | TYPE_ALL,
            ..Mosaic::default()
        }
        .sanitized();
        assert_eq!(m.types, TYPE_ROUND);
        assert!(m.kinds().len() > RoundTile::ALL.len());
        let old = Mosaic {
            types: TYPE_ALL,
            ..Mosaic::default()
        }
        .sanitized();
        assert_eq!(old.kinds().len(), 15);
    }

    #[test]
    fn round_mosaic_has_no_flat_edge_between_levels() {
        // Sample a deep round mosaic: both colours appear and nothing panics.
        let m = Mosaic {
            types: TYPE_ROUND,
            levels: 3,
            split: 0.6,
            seed: 5,
            ..Mosaic::default()
        };
        let (mut ink, mut ground) = (0, 0);
        for i in 0..200 {
            for j in 0..200 {
                if m.ink(i as f32 * 0.03, j as f32 * 0.03) {
                    ink += 1
                } else {
                    ground += 1
                }
            }
        }
        assert!(ink > 2000 && ground > 2000, "{ink} {ground}");
    }
}
