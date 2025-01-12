use crate::error::MResult;
use crate::renderer::vulkan::{VertexOffsets, VulkanMaterial, VulkanPipelineType};
use crate::renderer::{AddShaderBasicShaderData, DefaultType, Renderer};
use std::eprintln;
use std::sync::Arc;
use vulkano::command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer};
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::image::sampler::Sampler;
use vulkano::image::view::{ImageView, ImageViewCreateInfo};
use vulkano::image::{ImageAspects, ImageSubresourceRange, ImageType};
use vulkano::pipeline::{Pipeline, PipelineBindPoint};

pub struct VulkanSimpleShaderMaterial {
    diffuse: Arc<ImageView>,
    diffuse_sampler: Arc<Sampler>,
    descriptor_set: Arc<PersistentDescriptorSet>
}

impl VulkanSimpleShaderMaterial {
    pub fn new(renderer: &mut Renderer, add_shader_parameter: AddShaderBasicShaderData) -> MResult<Self> {
        let diffuse = renderer
            .get_or_default_2d(&add_shader_parameter.bitmap, 0, DefaultType::White)
            .vulkan
            .image
            .clone();

        if diffuse.array_layers() != 1 || diffuse.image_type() != ImageType::Dim2d {
            eprintln!("Warning: Can't display {} in a simple shader material. Using fallback...", add_shader_parameter.bitmap.as_ref().unwrap());
            return VulkanSimpleShaderMaterial::new(renderer, AddShaderBasicShaderData {
                bitmap: None,
                ..add_shader_parameter
            })
        }

        let diffuse = ImageView::new(diffuse.clone(), ImageViewCreateInfo {
            subresource_range: ImageSubresourceRange {
                aspects: ImageAspects::COLOR,
                mip_levels: 0..diffuse.mip_levels(),
                array_layers: 0..diffuse.array_layers()
            },
            format: diffuse.format(),
            ..Default::default()
        })?;

        let diffuse_sampler = renderer.vulkan.default_2d_sampler.clone();

        let pipeline = renderer.vulkan.pipelines.get(&VulkanPipelineType::SimpleTexture).unwrap();

        let descriptor_set = PersistentDescriptorSet::new(
            renderer.vulkan.descriptor_set_allocator.as_ref(),
            pipeline.get_pipeline().layout().set_layouts()[3].clone(),
            [
                WriteDescriptorSet::sampler(0, diffuse_sampler.clone()),
                WriteDescriptorSet::image_view(1, diffuse.clone()),
            ],
            []
        )?;

        Ok(Self { diffuse, diffuse_sampler, descriptor_set })
    }
}

impl VulkanMaterial for VulkanSimpleShaderMaterial {
    fn generate_commands(
        &self,
        renderer: &Renderer,
        vertices: &VertexOffsets,
        repeat_shader: bool,
        to: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>
    ) -> MResult<()> {
        if !repeat_shader {
            let pipeline = renderer.vulkan.pipelines.get(&self.get_main_pipeline()).unwrap();
            to.bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                pipeline.get_pipeline().layout().clone(),
                3,
                self.descriptor_set.clone()
            )?;
        }
        vertices.make_vulkan_draw_command(to)?;
        Ok(())
    }

    fn is_transparent(&self) -> bool {
        true
    }

    fn get_main_pipeline(&self) -> VulkanPipelineType {
        VulkanPipelineType::SimpleTexture
    }

    fn can_reuse_descriptors(&self) -> bool {
        true
    }
}
