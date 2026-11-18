//! Fireheart guest for `truchet_ms`.
//!
//! The host sends the point after the pre-affine. This module maps that point
//! and leaves the affine alone. A sample on the ground is written as NaN so the
//! host drops it. Prefer the host builtin for filter/snap soft-hide.

use truchet_tiles::{
    disk_map, disk_sample, filter_point, sample_ink, snap_point, Mosaic, TYPE_ROUND,
};

const BATCH_MAX: usize = 256;
const BUF_CAP: usize = 64 * 1024;
const CATALOG: &str = concat!(
    "{\"id\":\"truchet\",\"plugins\":[{",
    "\"index\":0,",
    "\"name\":\"truchet_ms\",",
    "\"author\":\"Pupcakes\",",
    "\"description\":\"Multi-scale Carlson Truchet bands. Odd levels flip ink and ground.\",",
    "\"stage\":0,",
    "\"params\":[",
    "{\"name\":\"seed\",\"default\":1},",
    "{\"name\":\"width\",\"default\":6},",
    "{\"name\":\"height\",\"default\":6},",
    "{\"name\":\"levels\",\"default\":4},",
    "{\"name\":\"split\",\"default\":0.5},",
    "{\"name\":\"types\",\"default\":3},",
    "{\"name\":\"style\",\"default\":0},",
    "{\"name\":\"noise\",\"default\":0.45},",
    "{\"name\":\"scale\",\"default\":1},",
    "{\"name\":\"mode\",\"default\":0},",
    "{\"name\":\"snap\",\"default\":0.4},",
    "{\"name\":\"grade\",\"default\":0}",
    "]},{",
    "\"index\":1,",
    "\"name\":\"hypertile\",",
    "\"author\":\"fireheart\",",
    "\"description\":\"{p,q} disk map. n picks the p-fold turn.\",",
    "\"stage\":0,",
    "\"params\":[",
    "{\"name\":\"p\",\"default\":3},",
    "{\"name\":\"q\",\"default\":7},",
    "{\"name\":\"n\",\"default\":0}",
    "]}]}"
);

static mut BUF: [u8; BUF_CAP] = [0; BUF_CAP];
static mut IN: [f32; BATCH_MAX * 3] = [0.0; BATCH_MAX * 3];
static mut OUT: [f32; BATCH_MAX * 3] = [0.0; BATCH_MAX * 3];

#[no_mangle]
pub extern "C" fn pack_json_ptr() -> i32 {
    CATALOG.as_ptr() as i32
}

#[no_mangle]
pub extern "C" fn pack_json_len() -> i32 {
    CATALOG.len() as i32
}

#[no_mangle]
pub extern "C" fn pack_buf_ptr() -> i32 {
    std::ptr::addr_of!(BUF) as i32
}

#[no_mangle]
pub extern "C" fn pack_buf_cap() -> i32 {
    BUF_CAP as i32
}

#[no_mangle]
pub extern "C" fn pack_in_ptr() -> i32 {
    std::ptr::addr_of!(IN) as i32
}

#[no_mangle]
pub extern "C" fn pack_out_ptr() -> i32 {
    std::ptr::addr_of!(OUT) as i32
}

#[no_mangle]
pub extern "C" fn pack_batch_cap() -> i32 {
    BATCH_MAX as i32
}

#[no_mangle]
pub extern "C" fn pack_apply(len: i32, x: f32, y: f32, z: f32) -> i32 {
    unsafe {
        let inp = std::ptr::addr_of_mut!(IN).cast::<f32>();
        *inp = x;
        *inp.add(1) = y;
        *inp.add(2) = z;
    }
    pack_apply_batch(len, 1)
}

#[no_mangle]
pub extern "C" fn pack_apply_batch(len: i32, count: i32) -> i32 {
    if len < 0 || len as usize > BUF_CAP || count <= 0 || count as usize > BATCH_MAX {
        return 0;
    }
    let bytes =
        unsafe { std::slice::from_raw_parts(std::ptr::addr_of!(BUF).cast::<u8>(), len as usize) };
    let Some(slot) = decode_slot(bytes) else {
        return 0;
    };
    for i in 0..count as usize {
        let (x, y, z) = unsafe {
            let inp = std::ptr::addr_of!(IN).cast::<f32>();
            (*inp.add(i * 3), *inp.add(i * 3 + 1), *inp.add(i * 3 + 2))
        };
        let mapped = apply_slot(&slot, x, y, z);
        unsafe {
            let out = std::ptr::addr_of_mut!(OUT).cast::<f32>().add(i * 3);
            match mapped {
                Some(p) => {
                    *out = p[0];
                    *out.add(1) = p[1];
                    *out.add(2) = p[2];
                }
                None => {
                    *out = f32::NAN;
                    *out.add(1) = f32::NAN;
                    *out.add(2) = f32::NAN;
                }
            }
        }
    }
    1
}

fn apply_slot(slot: &Slot, x: f32, y: f32, z: f32) -> Option<[f32; 3]> {
    if slot.index == 1 {
        let point = disk_map(
            x,
            y,
            slot.param("p", 3.0),
            slot.param("q", 7.0),
            slot.param("n", 0.0),
            slot.weight,
        )?;
        return Some([point[0], point[1], z * slot.weight]);
    }
    let mosaic = Mosaic {
        seed: slot.param("seed", 1.0) as u32,
        width: slot.param("width", 6.0),
        height: slot.param("height", 6.0),
        levels: slot.param("levels", 4.0) as i32,
        split: slot.param("split", 0.5),
        // style >= 0.5 selects the dot set; `types` is ignored then.
        types: if slot.param("style", 0.0) >= 0.5 {
            TYPE_ROUND
        } else {
            slot.param("types", 3.0) as u32
        },
        noise: slot.param("noise", 0.45),
        grade: slot.param("grade", 0.0),
    };
    let scale = slot.param("scale", 1.0);
    let mode = slot.param("mode", 0.0).round() as i32;
    if mode == 1 {
        return filter_point(&mosaic, x, y, z, slot.weight, scale);
    }
    if mode == 3 {
        return disk_sample(&mosaic, slot.weight, next_unit);
    }
    if mode == 2 {
        return snap_point(
            &mosaic,
            x,
            y,
            z,
            slot.weight,
            scale,
            slot.param("snap", 0.4),
            next_unit,
        );
    }
    sample_ink(&mosaic, z, slot.weight, scale, next_unit)
}

fn next_unit() -> f32 {
    static mut STATE: u32 = 0xA341_316C;
    unsafe {
        let mut x = STATE;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        if x == 0 {
            x = 0xA341_316C;
        }
        STATE = x;
        (x >> 8) as f32 / 16777216.0
    }
}

struct Slot {
    index: u32,
    weight: f32,
    params: Vec<(String, f32)>,
}

impl Slot {
    fn param(&self, name: &str, default: f32) -> f32 {
        self.params
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| *value)
            .unwrap_or(default)
    }
}

fn decode_slot(bytes: &[u8]) -> Option<Slot> {
    let mut cur = Cursor { bytes, i: 0 };
    for _ in 0..6 {
        cur.f32()?;
    }
    let count = cur.u32()?;
    if count == 0 {
        return None;
    }
    let index = cur.u32()?;
    let weight = cur.f32()?;
    let nparams = cur.u32()? as usize;
    if nparams > 32 {
        return None;
    }
    let mut params = Vec::with_capacity(nparams);
    for _ in 0..nparams {
        let len = cur.u32()? as usize;
        if len > 64 {
            return None;
        }
        let name = std::str::from_utf8(cur.bytes(len)?).ok()?.to_string();
        let pad = (4 - (len % 4)) % 4;
        cur.bytes(pad)?;
        params.push((name, cur.f32()?));
    }
    Some(Slot {
        index,
        weight,
        params,
    })
}

struct Cursor<'a> {
    bytes: &'a [u8],
    i: usize,
}

impl<'a> Cursor<'a> {
    fn bytes(&mut self, n: usize) -> Option<&'a [u8]> {
        let end = self.i.checked_add(n)?;
        let slice = self.bytes.get(self.i..end)?;
        self.i = end;
        Some(slice)
    }

    fn u32(&mut self) -> Option<u32> {
        let raw = self.bytes(4)?;
        Some(u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]))
    }

    fn f32(&mut self) -> Option<f32> {
        Some(f32::from_bits(self.u32()?))
    }
}
