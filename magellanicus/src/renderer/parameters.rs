mod bitmap;
mod geometry;
mod shader;
mod bsp;
mod sky;
mod font;

pub use bitmap::*;
pub use geometry::*;
pub use shader::*;
pub use bsp::*;
pub use sky::*;
pub use font::*;

/// Used for initializing a renderer.
///
/// These fields can be changed later with their respective set_* methods.
pub struct RendererParameters {
    /// Resolution of the renderer in (width, height)
    ///
    /// Default = 640x480
    pub resolution: Resolution,

    // TODO: Separate number_of_viewports from this?
    /// Number of viewports (must be 1-4)
    ///
    /// Default = 1
    pub number_of_viewports: usize,

    /// Enable vSync.
    ///
    /// Default = false
    pub vsync: bool,

    /// Number of samples per pixel.
    pub msaa: MSAA,

    /// Anisotropic filtering.
    pub anisotropic_filtering: Option<f32>,

    /// Render scaling
    pub render_scale: f32,
}

#[derive(Copy, Clone, PartialEq, Default)]
pub enum MSAA {
    #[default]
    NoMSAA = 1,
    MSAA2x = 2,
    MSAA4x = 4,
    MSAA8x = 8,
    MSAA16x = 16,
    MSAA32x = 32,
    MSAA64x = 64,
}

impl Default for RendererParameters {
    fn default() -> Self {
        Self {
            resolution: Resolution::default(),
            number_of_viewports: 1,
            vsync: false,
            msaa: Default::default(),
            anisotropic_filtering: None,
            render_scale: 1.0
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Resolution {
    pub width: u32,
    pub height: u32
}

impl Default for Resolution {
    fn default() -> Self {
        Self { width: 640, height: 480 }
    }
}
