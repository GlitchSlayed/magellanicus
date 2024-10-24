#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct ModelVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub binormal: [f32; 3],
    pub tangent: [f32; 3],
    pub texture_coords: [f32; 2]
}

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct LightmapVertex {
    pub lightmap_texture_coords: [f32; 2]
}

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct ModelTriangle {
    pub indices: [u16; 3]
}

#[derive(Copy, Clone, Debug)]
pub struct VertexOffsets {
    pub index_count: u32,
    pub vertex_offset: i32,
    pub index_offset: u32,
}