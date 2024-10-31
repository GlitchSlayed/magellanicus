use crate::vertex::ModelVertex;

pub use crate::renderer::data::GeometryDetailData;

#[derive(Clone, Debug)]
pub struct AddGeometryParameter {
    pub nodes: Vec<AddGeometryParameterNode>,
    pub geometries: Vec<AddGeometryParameterGeometry>,
    pub cutoff: GeometryDetailData<f32>,
    pub base_uv: [f32; 2]
}

#[derive(Clone, Debug)]
pub struct AddGeometryParameterVertex {
    pub vertex_data: ModelVertex,
    pub node0: String,
    pub node1: Option<String>,
    pub node0_weight: f32,
}

#[derive(Clone, Debug)]
pub struct AddGeometryParameterPart {
    pub shader: String,
    pub vertices: Vec<AddGeometryParameterVertex>,
    pub indices: Vec<u16>,
    pub centroid: [f32; 3],
    pub previous_filthy_part_index: Option<usize>,
    pub next_filthy_part_index: Option<usize>
}

#[derive(Clone, Debug)]
pub struct AddGeometryParameterGeometry {
    pub parts: Vec<AddGeometryParameterPart>
}

#[derive(Clone, Debug)]
pub struct AddGeometryParameterNode {
    pub name: String,
    pub children: Vec<AddGeometryParameterNode>,
    pub default_translation: [f32; 3],
    pub default_rotation: [f32; 4],
    pub node_distance_from_parent: f32
}

#[derive(Clone, Debug)]
pub struct AddGeometryParameterRegion {
    pub name: String,
    pub cannot_be_chosen_randomly: bool,
    pub geometry_indices: GeometryDetailData<usize>
}
