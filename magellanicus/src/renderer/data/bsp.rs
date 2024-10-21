use crate::error::MResult;
use crate::renderer::vulkan::VulkanBSPData;
use crate::renderer::{AddBSPParameter, AddBSPParameterLightmapMaterial, BSPData, Renderer};
use crate::vertex::VertexOffsets;
use alloc::vec::Vec;
use alloc::sync::Arc;
use alloc::string::String;

pub const MIN_DRAW_DISTANCE_LIMIT: f32 = 100.0;
pub const MAX_DRAW_DISTANCE_LIMIT: f32 = 2250.0;

pub struct BSP {
    pub vulkan: VulkanBSPData,
    pub geometries: Vec<BSPGeometry>,
    pub bsp_data: BSPData,
    pub cluster_surfaces: Vec<Vec<usize>>,
    pub geometry_indices_sorted_by_material: Vec<usize>,

    /// Calculated based on the size of the BSP, clamped between [`MIN_DRAW_DISTANCE_LIMIT`] and [`MAX_DRAW_DISTANCE_LIMIT`].
    pub draw_distance: f32
}

impl BSP {
    pub fn load_from_parameters(renderer: &mut Renderer, mut add_bsp_parameter: AddBSPParameter) -> MResult<Self> {
        struct BSPMaterialData<'a> {
            material_reflexive_index: usize,
            material_data: &'a AddBSPParameterLightmapMaterial,
            lightmap_reflexive_index: usize,
            lightmap_bitmap_index: Option<usize>
        }

        let add_bsp_iterator = add_bsp_parameter
            .lightmap_sets
            .iter()
            .enumerate()
            .map(|i|
                i.1
                    .materials
                    .iter()
                    .enumerate()
                    .zip(core::iter::repeat((i.0, i.1.lightmap_index)))
            )
            .flatten()
            .map(|(material, lightmap)| {
                BSPMaterialData {
                    material_reflexive_index: material.0,
                    material_data: material.1,
                    lightmap_reflexive_index: lightmap.0,
                    lightmap_bitmap_index: lightmap.1
                }
            });

        let count = add_bsp_iterator.clone().count();
        let mut geometries = Vec::with_capacity(count);

        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        let mut max_z = f32::NEG_INFINITY;
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut min_z = f32::INFINITY;

        let mut vertex_offset = 0i32;
        let mut index_offset = 0u32;

        for data in add_bsp_iterator {
            for p in &data.material_data.shader_vertices {
                min_x = min_x.min(p.position[0]);
                min_y = min_y.min(p.position[1]);
                min_z = min_z.min(p.position[2]);
                max_x = max_x.max(p.position[0]);
                max_y = max_y.max(p.position[1]);
                max_z = max_z.max(p.position[2]);
            }

            let index_count = (data.material_data.surfaces.len() * 3) as u32;
            geometries.push(BSPGeometry {
                shader: renderer.shaders.get_key_value(&data.material_data.shader).unwrap().0.clone(),
                lightmap_index: data.material_data.lightmap_vertices.as_ref().and(data.lightmap_bitmap_index),
                material_reflexive_index: data.material_reflexive_index,
                lightmap_reflexive_index: data.lightmap_reflexive_index,
                centroid: data.material_data.centroid,
                offset: VertexOffsets {
                    index_offset,
                    vertex_offset,
                    index_count
                },
            });

            vertex_offset += data.material_data.shader_vertices.len() as i32;
            index_offset += index_count;
        }

        let mut geometry_indices_sorted_by_material = Vec::from_iter(0usize..geometries.len());
        geometry_indices_sorted_by_material.sort_by(|a, b| {
            geometries[*a].shader.cmp(&geometries[*b].shader)
        });

        let draw_distance = if max_x == f32::NEG_INFINITY {
            0.0
        }
        else {
            let x = max_x - min_x;
            let y = max_y - min_y;
            let z = max_z - min_z;
            (x*x+y*y+z*z).sqrt() + 10.0 // add some leeway for if the camera goes slightly outside the BSP
        }.clamp(MIN_DRAW_DISTANCE_LIMIT, MAX_DRAW_DISTANCE_LIMIT);

        let bsp_data = &mut add_bsp_parameter.bsp_data;
        let cluster_surfaces: Vec<Vec<usize>> = Vec::with_capacity(bsp_data.clusters.len());

        let vulkan = VulkanBSPData::new(renderer, &add_bsp_parameter, &geometries)?;

        Ok(Self { vulkan, geometries, bsp_data: add_bsp_parameter.bsp_data, cluster_surfaces, draw_distance, geometry_indices_sorted_by_material })
    }
}

pub struct BSPGeometry {
    pub offset: VertexOffsets,
    pub shader: Arc<String>,
    pub lightmap_index: Option<usize>,
    pub centroid: [f32; 3],

    pub material_reflexive_index: usize,
    pub lightmap_reflexive_index: usize
}
