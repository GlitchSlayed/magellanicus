use crate::error::MResult;
use crate::renderer::vulkan::pipeline::pipeline_loader::{load_pipeline, DepthAccess, PipelineSettings};
use crate::renderer::vulkan::vertex::{VulkanModelVertex, VulkanModelVertexLightmapTextureCoords, VulkanModelVertexTextureCoords};
use crate::renderer::vulkan::{SwapchainImages, VulkanPipelineData};
use std::sync::Arc;
use std::vec;
use vulkano::device::Device;
use vulkano::pipeline::graphics::color_blend::ColorBlendAttachmentState;
use vulkano::pipeline::graphics::vertex_input::Vertex;
use vulkano::pipeline::GraphicsPipeline;

mod vertex {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/renderer/vulkan/pipeline/shader_environment/vertex.vert"
    }
}

// FIXME: remove the ./
mod fragment {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "./src/renderer/vulkan/pipeline/shader_environment/fragment.frag"
    }
}

pub use fragment::ShaderEnvironmentData;

pub struct ShaderEnvironment {
    pub pipeline: Arc<GraphicsPipeline>
}

impl ShaderEnvironment {
    pub fn new(swapchain_images: &SwapchainImages, device: Arc<Device>) -> MResult<Self> {
        let pipeline = load_pipeline(swapchain_images, device, vertex::load, fragment::load, &PipelineSettings {
            depth_access: DepthAccess::DepthWrite,
            vertex_buffer_descriptions: vec![VulkanModelVertex::per_vertex(), VulkanModelVertexTextureCoords::per_vertex(), VulkanModelVertexLightmapTextureCoords::per_vertex()],
            samples: swapchain_images.color.image().samples(),
            color_blend_attachment_state: ColorBlendAttachmentState::default(),
            ..Default::default()
        })?;

        Ok(Self { pipeline })
    }
}

impl VulkanPipelineData for ShaderEnvironment {
    fn get_pipeline(&self) -> Arc<GraphicsPipeline> {
        self.pipeline.clone()
    }
    fn has_lightmaps(&self) -> bool {
        true
    }
    fn has_fog(&self) -> bool {
        true
    }
}
