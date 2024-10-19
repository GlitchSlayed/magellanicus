use crate::error::MResult;
use crate::renderer::vulkan::{default_allocation_create_info, VulkanMaterial, VulkanPipelineData, VulkanPipelineType};
use crate::renderer::{AddShaderTransparentChicagoShaderData, AddShaderTransparentChicagoShaderMap, DefaultType, Renderer, ShaderTransparentChicagoFirstMapType, ShaderTransparentChicagoFramebufferFunction};
use std::sync::Arc;
use std::borrow::ToOwned;
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage};
use vulkano::command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer};
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::image::view::{ImageView, ImageViewCreateInfo, ImageViewType};
use vulkano::pipeline::{Pipeline, PipelineBindPoint};
use vulkano::pipeline::graphics::rasterization::CullMode;

pub struct VulkanShaderTransparentChicagoMaterial {
    pipeline: Arc<dyn VulkanPipelineData>,
    descriptor_set: Arc<PersistentDescriptorSet>,
    two_sided: bool
}

impl VulkanShaderTransparentChicagoMaterial {
    pub fn new(renderer: &mut Renderer, add_shader_parameter: AddShaderTransparentChicagoShaderData) -> MResult<Self> {
        let get_map = |index: usize| -> AddShaderTransparentChicagoShaderMap {
            add_shader_parameter
                .maps
                .get(index)
                .map(|f| f.to_owned())
                .unwrap_or_default()
        };

        let map0 = get_map(0);
        let map1 = get_map(1);
        let map2 = get_map(2);
        let map3 = get_map(3);

        let default_map = DefaultType::Null;

        let (map0_2d, map0_cubemap) = if add_shader_parameter.first_map_type == ShaderTransparentChicagoFirstMapType::Dim2D {
            (renderer.get_or_default_2d(&map0.bitmap, 0, default_map), renderer.get_default_cubemap(default_map))
        }
        else {
            (renderer.get_default_2d(default_map), renderer.get_or_default_cubemap(&map0.bitmap, 0, default_map))
        };
        let map0_2d = ImageView::new_default(map0_2d.vulkan.image.clone())?;
        let map0_cubemap = ImageView::new(
            map0_cubemap.vulkan.image.clone(),
            ImageViewCreateInfo {
                view_type: ImageViewType::Cube,
                ..ImageViewCreateInfo::from_image(&map0_cubemap.vulkan.image)
            }
        )?;
        let map1_2d = ImageView::new_default(renderer.get_or_default_2d(&map1.bitmap, 0, default_map).vulkan.image.clone())?;
        let map2_2d = ImageView::new_default(renderer.get_or_default_2d(&map2.bitmap, 0, default_map).vulkan.image.clone())?;
        let map3_2d = ImageView::new_default(renderer.get_or_default_2d(&map3.bitmap, 0, default_map).vulkan.image.clone())?;

        let premultiply = match add_shader_parameter.framebuffer_method {
            ShaderTransparentChicagoFramebufferFunction::Add => 1,
            ShaderTransparentChicagoFramebufferFunction::Subtract => 1,
            _ => 0
        };

        let alpha_replicate: u32 = (map0.alpha_replicate as u32)
            | ((map1.alpha_replicate as u32) << 1)
            | ((map2.alpha_replicate as u32) << 2)
            | ((map3.alpha_replicate as u32) << 3);

        let uniform = super::super::pipeline::shader_transparent_chicago::ShaderTransparentChicagoData {
            map0_uv: map0.uv_offset,
            map0_scale: map0.uv_scale,
            map0_color_function: map0.color_function as u32,
            map0_alpha_function: map0.alpha_function as u32,

            map1_uv: map1.uv_offset,
            map1_scale: map1.uv_scale,
            map1_color_function: map1.color_function as u32,
            map1_alpha_function: map1.alpha_function as u32,

            map2_uv: map2.uv_offset,
            map2_scale: map2.uv_scale,
            map2_color_function: map2.color_function as u32,
            map2_alpha_function: map2.alpha_function as u32,

            map3_uv: map3.uv_offset,
            map3_scale: map3.uv_scale,
            map3_color_function: map3.color_function as u32,
            map3_alpha_function: map3.alpha_function as u32,

            first_map_type: add_shader_parameter.first_map_type as u32,
            map_count: add_shader_parameter.maps.len() as u32,

            premultiply,
            alpha_replicate
        };

        let uniform_buffer = Buffer::from_data(
            renderer.renderer.memory_allocator.clone(),
            BufferCreateInfo { usage: BufferUsage::UNIFORM_BUFFER, ..Default::default() },
            default_allocation_create_info(),
            uniform
        )?;

        let map_sampler = renderer.renderer.default_2d_sampler.clone();

        let pipeline = renderer
            .renderer
            .pipelines[
                match add_shader_parameter.framebuffer_method {
                    ShaderTransparentChicagoFramebufferFunction::Add => &VulkanPipelineType::ShaderTransparentChicagoAdd,
                    ShaderTransparentChicagoFramebufferFunction::AlphaBlend => &VulkanPipelineType::ShaderTransparentChicagoAlphaBlend,
                    ShaderTransparentChicagoFramebufferFunction::Multiply => &VulkanPipelineType::ShaderTransparentChicagoMultiply,
                    ShaderTransparentChicagoFramebufferFunction::DoubleMultiply => &VulkanPipelineType::ShaderTransparentChicagoMultiply, // FIXME
                    ShaderTransparentChicagoFramebufferFunction::Subtract => &VulkanPipelineType::ShaderTransparentChicagoSubtract,
                    ShaderTransparentChicagoFramebufferFunction::ComponentMin => &VulkanPipelineType::ShaderTransparentChicagoComponentMin,
                    ShaderTransparentChicagoFramebufferFunction::ComponentMax => &VulkanPipelineType::ShaderTransparentChicagoComponentMax,
                    ShaderTransparentChicagoFramebufferFunction::AlphaMultiplyAdd => &VulkanPipelineType::ShaderTransparentChicagoAlphaBlend // FIXME
                }
            ]
            .clone();

        let descriptor_set = PersistentDescriptorSet::new(
            renderer.renderer.descriptor_set_allocator.as_ref(),
            pipeline.get_pipeline().layout().set_layouts()[3].clone(),
            [
                WriteDescriptorSet::buffer(0, uniform_buffer),
                WriteDescriptorSet::sampler(1, map_sampler),
                WriteDescriptorSet::image_view(2, map0_cubemap),
                WriteDescriptorSet::image_view(3, map0_2d),
                WriteDescriptorSet::image_view(4, map1_2d),
                WriteDescriptorSet::image_view(5, map2_2d),
                WriteDescriptorSet::image_view(6, map3_2d),
            ],
            []
        )?;

        let shader_data = Self {
            pipeline,
            descriptor_set,
            two_sided: add_shader_parameter.two_sided
        };

        Ok(shader_data)
    }
}

impl VulkanMaterial for VulkanShaderTransparentChicagoMaterial {
    fn generate_commands(
        &self,
        _renderer: &Renderer,
        index_count: u32,
        repeat_shader: bool,
        to: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>
    ) -> MResult<()> {
        if !repeat_shader {
            to.bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.pipeline.get_pipeline().layout().clone(),
                3,
                self.descriptor_set.clone()
            )?;
            if self.two_sided {
                to.set_cull_mode(CullMode::None)?;
            }
        }
        to.draw_indexed(index_count, 1, 0, 0, 0)?;
        Ok(())
    }

    fn is_transparent(&self) -> bool {
        true
    }

    fn get_main_pipeline(&self) -> Arc<dyn VulkanPipelineData> {
        self.pipeline.clone()
    }

    fn can_reuse_descriptors(&self) -> bool {
        true
    }
}
