use glam::Vec3;
use crate::renderer::data::{DRAW_DISTANCE_MINIMUM, MAX_DRAW_DISTANCE_LIMIT};
use crate::renderer::FogData;

#[derive(Copy, Clone, Debug)]
pub struct PlayerViewport {
    /// Relative X of the viewport (0.0-1.0)
    pub rel_x: f32,

    /// Relative Y of the viewport (0.0-1.0)
    pub rel_y: f32,

    /// Width of the viewport (0.0-1.0)
    pub rel_width: f32,

    /// Height of the viewport (0.0-1.0)
    pub rel_height: f32,

    /// Camera data
    pub camera: Camera,

    /// Current viewport fog data.
    ///
    /// NOTE: This will be automatically modified to the correct values when a BSP is loaded.
    pub viewport_fog: Option<ViewportFog>,

    /// Current draw distance.
    ///
    /// NOTE: This will be automatically modified to the correct value when a BSP is loaded.
    pub draw_distance: [f32; 2],
}

#[derive(Copy, Clone, Debug)]
pub struct ViewportFog {
    /// Current fog data (displayed)
    pub current_fog_data: FogData,

    /// Current outdoor fog.
    pub outdoor_fog_data: FogData,

    /// Current indoor fog.
    pub indoor_fog_data: FogData,

    /// Target fog data (transitioned as the camera moves)
    pub target_fog_data: FogData,

    /// Pending transition amount.
    pub transition_amount: f32
}

impl Default for PlayerViewport {
    fn default() -> Self {
        PlayerViewport {
            rel_x: 0.0,
            rel_y: 0.0,
            rel_width: 1.0,
            rel_height: 1.0,
            camera: Camera::default(),
            viewport_fog: None,
            draw_distance: [DRAW_DISTANCE_MINIMUM, MAX_DRAW_DISTANCE_LIMIT],
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Camera {
    /// Vertical FoV in radians
    pub fov: f32,

    /// Position in the map of the camera
    pub position: [f32; 3],

    /// Rotation of the camera
    pub rotation: [f32; 3],

    /// Enable lightmap.
    pub lightmaps: bool,

    /// Enable fog.
    pub fog: bool
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            fov: get_default_vertical_fov(),
            position: Vec3::default().to_array(),
            rotation: [0.0, 1.0, 0.0],
            lightmaps: true,
            fog: true
        }
    }
}

/// Default horizontal FoV to use.
pub const DEFAULT_HORIZONTAL_FOV: f32 = 70.0;

/// Get the default FoV.
pub fn get_default_vertical_fov() -> f32 {
    horizontal_to_vertical_fov(DEFAULT_HORIZONTAL_FOV.to_radians(), 640.0, 480.0)
}

/// Calculate the vertical FoV given horizontal FoV and aspect ratio.
#[inline(always)]
pub fn horizontal_to_vertical_fov(horizontal: f32, width: f32, height: f32) -> f32 {
    2.0 * ((horizontal / 2.0).tan() * height / width).atan()
}
