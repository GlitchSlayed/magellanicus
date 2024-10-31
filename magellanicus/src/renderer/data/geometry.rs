use std::collections::HashMap;
use std::sync::Arc;
use crate::renderer::vulkan::VulkanMaterialData;
use crate::vertex::{ModelVertex, VertexOffsets};

#[derive(Copy, Clone, Debug)]
pub struct GeometryDetailData<T: Sized + 'static> {
    pub super_low: T,
    pub low: T,
    pub medium: T,
    pub high: T,
    pub super_high: T,
}

impl<T: Sized + 'static> GeometryDetailData<T> {
    /// Returns an array of the data sorted from lowest to highest.
    pub fn as_arr(&self) -> [&T; 5] {
        [&self.super_low, &self.low, &self.medium, &self.high, &self.super_high]
    }

    /// Returns an array of mutable references to the data sorted from lowest to highest.
    pub fn as_arr_mut(&mut self) -> [&mut T; 5] {
        [&mut self.super_low, &mut self.low, &mut self.medium, &mut self.high, &mut self.super_high]
    }

    /// Return an iterator over the items, sorted from lowest to highest.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.as_arr().into_iter()
    }

    /// Return an iterator over mutable references to the items, sorted from lowest to highest.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.as_arr_mut().into_iter()
    }
}

pub struct Geometry {
    pub nodes: HashMap<Arc<String>, GeometryNode>,
    pub geometries: Vec<GeometryGeometry>,
    pub cutoff: GeometryDetailData<f32>,
    pub base_uv: [f32; 2],
    pub vulkan: VulkanMaterialData,
}

#[derive(Clone, Debug)]
pub struct Vertex {
    pub vertex_data: ModelVertex,
    pub node0: Arc<String>,
    pub node1: Option<Arc<String>>,
    pub node0_weight: f32,
}

#[derive(Clone, Debug)]
pub struct GeometryPart {
    pub shader: Arc<String>,
    pub offsets: VertexOffsets,

    // These fields are used for depth sorting
    pub centroid: [f32; 3],
    pub previous_filthy_part_index: Option<usize>,
    pub next_filthy_part_index: Option<usize>
}

#[derive(Clone, Debug)]
pub struct GeometryGeometry {
    pub parts: Vec<GeometryPart>
}

#[derive(Clone, Debug)]
pub struct GeometryNode {
    pub name: Arc<String>,
    pub children: Vec<GeometryNode>,
    pub default_translation: [f32; 3],
    pub default_rotation: [f32; 4],
    pub node_distance_from_parent: f32
}

#[derive(Clone, Debug)]
pub struct GeometryRegion {
    pub name: Arc<String>,
    pub cannot_be_chosen_randomly: bool,
    pub geometry_indices: GeometryDetailData<usize>
}
