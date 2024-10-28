use crate::error::MResult;
use crate::renderer::{AddBSPParameter, DefaultType, Renderer};

use crate::renderer::data::BSPGeometry;
use crate::renderer::vulkan::vertex::{VulkanModelVertex, VulkanModelVertexLightmapTextureCoords, VulkanModelVertexTextureCoords};
use crate::renderer::vulkan::{default_allocation_create_info, VulkanPipelineType};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::vec::Vec;
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage, Subbuffer};
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::image::sampler::{Sampler, SamplerAddressMode, SamplerCreateInfo};
use vulkano::image::view::{ImageView, ImageViewCreateInfo};
use vulkano::pipeline::Pipeline;

pub struct VulkanBSPData {
    pub subbuffers: Option<VulkanBSPVertexDataBuffers>,

    pub lightmap_images: BTreeMap<usize, Arc<PersistentDescriptorSet>>,
    pub null_lightmaps: Arc<PersistentDescriptorSet>,

    pub transparent_geometries: Vec<usize>,
    pub opaque_geometries: Vec<usize>,
}

impl VulkanBSPData {
    pub fn new(
        renderer: &mut Renderer,
        param: &AddBSPParameter,
        geometries: &Vec<BSPGeometry>
    ) -> MResult<Self> {
        let mut vertex_data: Vec<VulkanModelVertex> = Vec::new();
        let mut indices: Vec<u16> = Vec::new();
        let mut texture_coords_data: Vec<VulkanModelVertexTextureCoords> = Vec::new();
        let mut lightmap_texture_coords_data: Vec<VulkanModelVertexLightmapTextureCoords> = Vec::new();

        for l in &param.lightmap_sets {
            for m in &l.materials {
                indices.extend(m.surfaces.iter().map(|m| m.indices.iter()).flatten());
                vertex_data.extend(m.shader_vertices.iter().map(|s| VulkanModelVertex {
                    position: s.position,
                    normal: s.normal,
                    binormal: s.binormal,
                    tangent: s.tangent
                }));
                texture_coords_data.extend(m.shader_vertices.iter().map(|s| VulkanModelVertexTextureCoords {
                    texture_coords: s.texture_coords
                }));
                if let Some(n) = m.lightmap_vertices.as_ref() {
                    lightmap_texture_coords_data.extend(n.iter().map(|s| VulkanModelVertexLightmapTextureCoords {
                        lightmap_texture_coords: s.lightmap_texture_coords
                    }));
                }
                else {
                    lightmap_texture_coords_data.extend(m.shader_vertices.iter().map(|s| VulkanModelVertexLightmapTextureCoords {
                        lightmap_texture_coords: s.texture_coords
                    }));
                }
            }
        }

        let shader_environment_pipeline = renderer.vulkan.pipelines[&VulkanPipelineType::ShaderEnvironment].get_pipeline();
        let mut images = BTreeMap::new();
        if let Some(n) = &param.lightmap_bitmap {
            let image = renderer
                .bitmaps
                .get(n)
                .unwrap();

            for i in param.lightmap_sets.iter().filter_map(|b| b.lightmap_index) {
                if images.contains_key(&i) {
                    continue;
                }

                let image = image.bitmaps[i].vulkan.image.clone();

                let lightmap = ImageView::new(
                    image.clone(),
                    ImageViewCreateInfo::from_image(image.as_ref())
                )?;

                let sampler = Sampler::new(
                    renderer.vulkan.device.clone(),
                    SamplerCreateInfo {
                        address_mode: [
                            SamplerAddressMode::ClampToEdge,
                            SamplerAddressMode::ClampToEdge,
                            SamplerAddressMode::ClampToEdge
                        ],
                        ..SamplerCreateInfo::simple_repeat_linear_no_mipmap()
                    }
                )?;

                let descriptor_set = PersistentDescriptorSet::new(
                    renderer.vulkan.descriptor_set_allocator.as_ref(),
                    shader_environment_pipeline.layout().set_layouts()[1].clone(),
                    [
                        WriteDescriptorSet::sampler(0, sampler),
                        WriteDescriptorSet::image_view(1, lightmap),
                    ],
                    []
                )?;

                images.insert(i, descriptor_set);
            }
        }

        let null_set = PersistentDescriptorSet::new(
            renderer.vulkan.descriptor_set_allocator.as_ref(),
            shader_environment_pipeline.layout().set_layouts()[1].clone(),
            [
                WriteDescriptorSet::sampler(0, renderer.vulkan.default_2d_sampler.clone()),
                WriteDescriptorSet::image_view(1, ImageView::new_default(renderer.get_default_2d(DefaultType::White).vulkan.image.clone())?),
            ],
            []
        ).unwrap();

        let mut transparent_geometries: Vec<usize> = geometries
            .iter()
            .enumerate()
            .filter_map(|f| if renderer.shaders[&f.1.shader].vulkan.pipeline_data.is_transparent() {
                Some(f.0)
            }
            else {
                None
            }).collect();

        let mut opaque_geometries: Vec<usize> = geometries
            .iter()
            .enumerate()
            .filter_map(|f| if !renderer.shaders[&f.1.shader].vulkan.pipeline_data.is_transparent() {
                Some(f.0)
            }
            else {
                None
            }).collect();

        transparent_geometries.sort_by(|a,b| geometries[*a].shader.cmp(&geometries[*b].shader));
        opaque_geometries.sort_by(|a,b| geometries[*a].shader.cmp(&geometries[*b].shader));

        let subbuffers = if !indices.is_empty() {
            let vertex_data_subbuffer = Buffer::from_iter(
                renderer.vulkan.memory_allocator.clone(),
                BufferCreateInfo {
                    usage: BufferUsage::VERTEX_BUFFER,
                    ..Default::default()
                },
                default_allocation_create_info(),
                vertex_data.into_iter()
            )?;

            let texture_coords_subbuffer = Buffer::from_iter(
                renderer.vulkan.memory_allocator.clone(),
                BufferCreateInfo {
                    usage: BufferUsage::VERTEX_BUFFER,
                    ..Default::default()
                },
                default_allocation_create_info(),
                texture_coords_data.into_iter()
            )?;

            let lightmap_texture_coords_subbuffer = Buffer::from_iter(
                renderer.vulkan.memory_allocator.clone(),
                BufferCreateInfo {
                    usage: BufferUsage::VERTEX_BUFFER,
                    ..Default::default()
                },
                default_allocation_create_info(),
                lightmap_texture_coords_data.into_iter()
            )?;

            let index_subbuffer = Buffer::from_iter(
                renderer.vulkan.memory_allocator.clone(),
                BufferCreateInfo {
                    usage: BufferUsage::INDEX_BUFFER,
                    ..Default::default()
                },
                default_allocation_create_info(),
                indices.into_iter()
            )?;

            Some(VulkanBSPVertexDataBuffers {
                vertex_data_subbuffer,
                texture_coords_subbuffer,
                lightmap_texture_coords_subbuffer,
                index_subbuffer,
            })
        }
        else {
            None
        };

        Ok(Self {
            subbuffers,
            lightmap_images: images,
            null_lightmaps: null_set,
            opaque_geometries,
            transparent_geometries
        })
    }
}

pub struct VulkanBSPVertexDataBuffers {
    pub vertex_data_subbuffer: Subbuffer<[VulkanModelVertex]>,
    pub texture_coords_subbuffer: Subbuffer<[VulkanModelVertexTextureCoords]>,
    pub lightmap_texture_coords_subbuffer: Subbuffer<[VulkanModelVertexLightmapTextureCoords]>,
    pub index_subbuffer: Subbuffer<[u16]>,
}
