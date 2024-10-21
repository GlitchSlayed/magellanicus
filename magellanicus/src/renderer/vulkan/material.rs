mod simple_shader;
mod shader_environment;
mod shader_transparent_chicago;

use crate::error::MResult;
use crate::renderer::vulkan::material::shader_environment::VulkanShaderEnvironmentMaterial;
use crate::renderer::vulkan::material::shader_transparent_chicago::VulkanShaderTransparentChicagoMaterial;
use crate::renderer::vulkan::material::simple_shader::VulkanSimpleShaderMaterial;
use crate::renderer::vulkan::VulkanPipelineType;
use crate::renderer::{AddShaderData, AddShaderParameter, Renderer};
use std::sync::Arc;
use vulkano::command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer};
use crate::vertex::VertexOffsets;

/// Material shader data
///
/// Vertex inputs are bound like this:
///
/// - layout 0, location 0 is vertex data, defined as [`VulkanModelVertex`](crate::renderer::vulkan::vertex::VulkanModelVertex)
/// - layout 0, location 1 is texture coordinates, defined as [`VulkanModelVertexTextureCoords`](crate::renderer::vulkan::vertex::VulkanModelVertexTextureCoords)
/// - layout 0, location 2 is lightmap texture coordinates, defined as [`VulkanModelVertexTextureCoords`](crate::renderer::vulkan::vertex::VulkanModelVertexTextureCoords)
///
/// Descriptor sets are bound like this:
///
/// - set 0, binding 0 is ModelData, defined as [`VulkanModelData`](crate::renderer::vulkan::vertex::VulkanModelData)
/// - set 1, binding 0 is a sampler for lightmaps
/// - set 1, binding 1 is an image view for lightmaps
///
/// Nothing will be bound on layout 1+. Anything on set 2+ is shader-specific.

pub struct VulkanMaterialShaderData {
    pub pipeline_data: Arc<dyn VulkanMaterial>,
}

impl VulkanMaterialShaderData {
    pub fn new_from_parameters(renderer: &mut Renderer, shader: AddShaderParameter) -> MResult<Self> {
        match shader.data {
            AddShaderData::BasicShader(shader) => {
                let shader = Arc::new(VulkanSimpleShaderMaterial::new(renderer, shader)?);
                Ok(Self { pipeline_data: shader })
            }
            AddShaderData::ShaderEnvironment(shader) => {
                let shader = Arc::new(VulkanShaderEnvironmentMaterial::new(renderer, shader)?);
                Ok(Self { pipeline_data: shader })
            }
            AddShaderData::ShaderTransparentChicago(shader) => {
                let shader = Arc::new(VulkanShaderTransparentChicagoMaterial::new(renderer, shader)?);
                Ok(Self { pipeline_data: shader })
            }
        }
    }
}

impl VertexOffsets {
    pub fn make_vulkan_draw_command(&self, to: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>) -> MResult<()> {
        to.draw_indexed(self.index_count, 1, self.index_offset, self.vertex_offset, 0)?;
        Ok(())
    }
}

pub trait VulkanMaterial: Send + Sync + 'static {
    /// Generate rendering commands.
    ///
    /// All vertex buffers (vertices, texture coords, lightmap texture coords) will be bound before
    /// this is called.
    fn generate_commands(
        &self,
        renderer: &Renderer,
        vertices: &VertexOffsets,
        repeat_shader: bool,
        to: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    ) -> MResult<()>;

    /// Return `true` if the material is transparent.
    ///
    /// If so, it needs to be rendered back-to-front.
    ///
    /// Default: `false`
    fn is_transparent(&self) -> bool {
        false
    }

    /// Get the main graphics pipeline that will be used for drawing.
    fn get_main_pipeline(&self) -> VulkanPipelineType;

    /// If `true`, this can reuse descriptors from a previous call.
    fn can_reuse_descriptors(&self) -> bool;
}
