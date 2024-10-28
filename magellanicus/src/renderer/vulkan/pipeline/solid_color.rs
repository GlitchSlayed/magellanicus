use std::sync::Arc;
use crate::error::MResult;
use crate::renderer::vulkan::pipeline::pipeline_loader::{load_pipeline, DepthAccess, PipelineSettings};
use crate::renderer::vulkan::vertex::VulkanModelVertex;
use crate::renderer::vulkan::{SwapchainImages, VulkanPipelineData};
use std::vec;
use vulkano::device::Device;
use vulkano::pipeline::graphics::color_blend::ColorBlendAttachmentState;
use vulkano::pipeline::graphics::vertex_input::Vertex;
use vulkano::pipeline::GraphicsPipeline;

mod vertex {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/renderer/vulkan/pipeline/solid_color/vertex.vert"
    }
}

mod fragment {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/renderer/vulkan/pipeline/solid_color/fragment.frag"
    }
}

pub struct SolidColorShader {
    pub pipeline: Arc<GraphicsPipeline>
}

impl SolidColorShader {
    pub fn new(swapchain_images: &SwapchainImages, device: Arc<Device>) -> MResult<Self> {
        let pipeline = load_pipeline(swapchain_images, device, vertex::load, fragment::load, &PipelineSettings {
            depth_access: DepthAccess::DepthWrite,
            vertex_buffer_descriptions: vec![VulkanModelVertex::per_vertex()],
            color_blend_attachment_state: ColorBlendAttachmentState::default(),
            samples: swapchain_images.color.image().samples(),
            ..Default::default()
        })?;

        Ok(Self { pipeline })
    }
}

impl VulkanPipelineData for SolidColorShader {
    fn get_pipeline(&self) -> Arc<GraphicsPipeline> {
        self.pipeline.clone()
    }
    fn has_lightmaps(&self) -> bool {
        false
    }
    fn has_fog(&self) -> bool {
        false
    }
}
