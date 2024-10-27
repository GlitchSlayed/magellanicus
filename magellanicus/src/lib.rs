#![no_std]
#![allow(dead_code)]

// crate `std` is needed for the Vulkano crate (and thus the vulkan module), but nothing else
extern crate std;
extern crate alloc;

pub mod vertex;
pub mod error;
pub mod renderer;

/// RGBA
pub type FloatColor = [f32; 4];

/// Converts [`FloatColor`] to RGBAF32 in bytes.
pub(crate) fn to_rgbaf32(v: FloatColor) -> [u8; 16] {
    let r = v[0].to_le_bytes();
    let g = v[1].to_le_bytes();
    let b = v[2].to_le_bytes();
    let a = v[3].to_le_bytes();
    [
        r[0],r[1],r[2],r[3],
        g[0],g[1],g[2],g[3],
        b[0],b[1],b[2],b[3],
        a[0],a[1],a[2],a[3],
    ]
}
