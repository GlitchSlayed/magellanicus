use std::collections::BTreeMap;
use std::sync::Arc;
use vulkano::device::Device;
use vulkano::pipeline::graphics::color_blend::{AttachmentBlend, BlendFactor, BlendOp};
use vulkano::pipeline::GraphicsPipeline;
use crate::error::MResult;
use crate::renderer::vulkan::SwapchainImages;

pub mod solid_color;
pub mod simple_texture;
mod pipeline_loader;
mod color_box;
pub mod shader_environment;
pub mod shader_transparent_chicago;

pub trait VulkanPipelineData: Send + Sync + 'static {
    fn get_pipeline(&self) -> Arc<GraphicsPipeline>;
    fn has_lightmaps(&self) -> bool;
    fn has_fog(&self) -> bool;
}

pub fn load_all_pipelines(swapchain_images: &SwapchainImages, device: Arc<Device>) -> MResult<BTreeMap<VulkanPipelineType, Arc<dyn VulkanPipelineData>>> {
    let mut pipelines: BTreeMap<VulkanPipelineType, Arc<dyn VulkanPipelineData>> = BTreeMap::new();

    pipelines.insert(VulkanPipelineType::SolidColor, Arc::new(solid_color::SolidColorShader::new(swapchain_images, device.clone())?));
    pipelines.insert(VulkanPipelineType::SimpleTexture, Arc::new(simple_texture::SimpleTextureShader::new(swapchain_images, device.clone())?));
    pipelines.insert(VulkanPipelineType::ColorBox, Arc::new(color_box::ColorBox::new(swapchain_images, device.clone())?));
    pipelines.insert(VulkanPipelineType::ShaderEnvironment, Arc::new(shader_environment::ShaderEnvironment::new(swapchain_images, device.clone())?));

    let add = AttachmentBlend::additive();
    let alpha_blend = AttachmentBlend::alpha();
    let subtract = AttachmentBlend {
        src_color_blend_factor: BlendFactor::One,
        dst_color_blend_factor: BlendFactor::One,
        color_blend_op: BlendOp::Subtract,
        src_alpha_blend_factor: BlendFactor::One,
        dst_alpha_blend_factor: BlendFactor::One,
        alpha_blend_op: BlendOp::Subtract,
    };
    let component_min = AttachmentBlend {
        src_color_blend_factor: BlendFactor::One,
        dst_color_blend_factor: BlendFactor::One,
        color_blend_op: BlendOp::Min,
        src_alpha_blend_factor: BlendFactor::One,
        dst_alpha_blend_factor: BlendFactor::One,
        alpha_blend_op: BlendOp::Min,
    };
    let component_max = AttachmentBlend {
        src_color_blend_factor: BlendFactor::One,
        dst_color_blend_factor: BlendFactor::One,
        color_blend_op: BlendOp::Max,
        src_alpha_blend_factor: BlendFactor::One,
        dst_alpha_blend_factor: BlendFactor::One,
        alpha_blend_op: BlendOp::Max,
    };
    let multiply = AttachmentBlend {
        src_color_blend_factor: BlendFactor::SrcColor,
        dst_color_blend_factor: BlendFactor::OneMinusSrcColor,
        color_blend_op: BlendOp::Add,
        src_alpha_blend_factor: BlendFactor::SrcAlpha,
        dst_alpha_blend_factor: BlendFactor::OneMinusSrcAlpha,
        alpha_blend_op: BlendOp::Add,
    };

    pipelines.insert(VulkanPipelineType::ShaderTransparentChicagoAdd, Arc::new(shader_transparent_chicago::ShaderTransparentChicago::new(swapchain_images, device.clone(), Some(add))?));
    pipelines.insert(VulkanPipelineType::ShaderTransparentChicagoAlphaBlend, Arc::new(shader_transparent_chicago::ShaderTransparentChicago::new(swapchain_images, device.clone(), Some(alpha_blend))?));
    pipelines.insert(VulkanPipelineType::ShaderTransparentChicagoSubtract, Arc::new(shader_transparent_chicago::ShaderTransparentChicago::new(swapchain_images, device.clone(), Some(subtract))?));
    pipelines.insert(VulkanPipelineType::ShaderTransparentChicagoComponentMin, Arc::new(shader_transparent_chicago::ShaderTransparentChicago::new(swapchain_images, device.clone(), Some(component_min))?));
    pipelines.insert(VulkanPipelineType::ShaderTransparentChicagoComponentMax, Arc::new(shader_transparent_chicago::ShaderTransparentChicago::new(swapchain_images, device.clone(), Some(component_max))?));
    pipelines.insert(VulkanPipelineType::ShaderTransparentChicagoMultiply, Arc::new(shader_transparent_chicago::ShaderTransparentChicago::new(swapchain_images, device.clone(), Some(multiply))?));

    Ok(pipelines)
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u32)]
pub enum VulkanPipelineType {
    /// Writes a solid color.
    ///
    /// Useful for testing.
    SolidColor,

    /// Draws a texture.
    SimpleTexture,

    /// Draw a box of a given color.
    ColorBox,

    /// shader_environment
    ShaderEnvironment,

    /// shader_transparent_chicago + Add
    ShaderTransparentChicagoAdd,
    /// shader_transparent_chicago + Alpha Blend
    ShaderTransparentChicagoAlphaBlend,
    /// shader_transparent_chicago + Subtract
    ShaderTransparentChicagoSubtract,
    /// shader_transparent_chicago + Component Min
    ShaderTransparentChicagoComponentMin,
    /// shader_transparent_chicago + Component Max
    ShaderTransparentChicagoComponentMax,
    /// shader_transparent_chicago + Multiply
    ShaderTransparentChicagoMultiply
}
