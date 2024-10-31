use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Instant;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use data::*;

pub use parameters::*;
use crate::renderer::vulkan::VulkanRenderer;
use player_viewport::*;
use crate::error::{Error, MResult};

pub use player_viewport::Camera;
pub use player_viewport::get_default_vertical_fov;
pub use player_viewport::horizontal_to_vertical_fov;

use glam::{FloatExt, Vec3};
use crate::types::FloatColor;

mod parameters;
mod vulkan;
mod data;
mod player_viewport;

pub struct Renderer {
    vulkan: VulkanRenderer,
    player_viewports: Vec<PlayerViewport>,

    bitmaps: HashMap<Arc<String>, Bitmap>,
    shaders: HashMap<Arc<String>, Shader>,
    geometries: HashMap<Arc<String>, Geometry>,
    skies: HashMap<Arc<String>, Sky>,
    bsps: HashMap<Arc<String>, Arc<BSP>>,
    fonts: HashMap<Arc<String>, Font>,

    default_bitmaps: DefaultBitmaps,
    current_bsp: Option<Arc<String>>,

    fps_counter_value: f64,
    fps_counter_time: Instant,
    fps_counter_count: u32,

    debug_text: VecDeque<Bitmap>,
    debug_text_stale: bool,
    debug_font: Option<Arc<String>>,
}

impl Renderer {
    /// Initialize a new renderer.
    ///
    /// Errors if:
    /// - `parameters` is invalid
    /// - the renderer backend could not be initialized for some reason
    pub unsafe fn new(surface: &(impl HasRawWindowHandle + HasRawDisplayHandle), parameters: RendererParameters) -> MResult<Self> {
        if parameters.resolution.height == 0 || parameters.resolution.width == 0 {
            return Err(Error::DataError { error: "resolution has 0 on one or more dimensions".to_owned() })
        }

        let mut player_viewports = vec![PlayerViewport::default(); parameters.number_of_viewports];

        match parameters.number_of_viewports {
            1 => {
                player_viewports[0].rel_x = 0.0;
                player_viewports[0].rel_y = 0.0;
                player_viewports[0].rel_width = 1.0;
                player_viewports[0].rel_height = 1.0;
            }
            2 => {
                player_viewports[0].rel_x = 0.0;
                player_viewports[0].rel_y = 0.0;
                player_viewports[0].rel_width = 1.0;
                player_viewports[0].rel_height = 0.5;

                player_viewports[1].rel_x = 0.0;
                player_viewports[1].rel_y = 0.5;
                player_viewports[1].rel_width = 1.0;
                player_viewports[1].rel_height = 0.5;
            }
            3 => {
                player_viewports[0].rel_x = 0.0;
                player_viewports[0].rel_y = 0.0;
                player_viewports[0].rel_width = 1.0;
                player_viewports[0].rel_height = 0.5;

                player_viewports[1].rel_x = 0.0;
                player_viewports[1].rel_y = 0.5;
                player_viewports[1].rel_width = 0.5;
                player_viewports[1].rel_height = 0.5;

                player_viewports[2].rel_x = 0.5;
                player_viewports[2].rel_y = 0.5;
                player_viewports[2].rel_width = 0.5;
                player_viewports[2].rel_height = 0.5;
            }
            4 => {
                player_viewports[0].rel_x = 0.0;
                player_viewports[0].rel_y = 0.0;
                player_viewports[0].rel_width = 0.5;
                player_viewports[0].rel_height = 0.5;

                player_viewports[1].rel_x = 0.5;
                player_viewports[1].rel_y = 0.0;
                player_viewports[1].rel_width = 0.5;
                player_viewports[1].rel_height = 0.5;

                player_viewports[2].rel_x = 0.0;
                player_viewports[2].rel_y = 0.5;
                player_viewports[2].rel_width = 0.5;
                player_viewports[2].rel_height = 0.5;

                player_viewports[3].rel_x = 0.5;
                player_viewports[3].rel_y = 0.5;
                player_viewports[3].rel_width = 0.5;
                player_viewports[3].rel_height = 0.5;
            }
            n => return Err(Error::DataError { error: format!("number of viewports was set to {n}, but only 1-4 are supported") })
        }

        let mut result = Self {
            vulkan: VulkanRenderer::new(&parameters, surface)?,
            player_viewports,
            bitmaps: HashMap::new(),
            shaders: HashMap::new(),
            geometries: HashMap::new(),
            skies: HashMap::new(),
            bsps: HashMap::new(),
            fonts: HashMap::new(),
            current_bsp: None,
            default_bitmaps: DefaultBitmaps::default(),
            fps_counter_value: 0.0,
            fps_counter_count: 0,
            fps_counter_time: Instant::now(),
            debug_text: VecDeque::with_capacity(64),
            debug_text_stale: true,
            debug_font: None,
        };

        populate_default_bitmaps(&mut result)?;

        Ok(result)
    }

    /// Clear all data without resetting the renderer.
    ///
    /// All objects added with `add_` methods will be cleared.
    pub fn reset(&mut self) {
        self.bitmaps.clear();
        self.shaders.clear();
        self.geometries.clear();
        self.skies.clear();
        self.bsps.clear();
        self.fonts.clear();
        self.current_bsp = None;
        self.debug_font = None;
        self.default_bitmaps = DefaultBitmaps::default();

        populate_default_bitmaps(self).unwrap();
        self.invalidate_debug_text();
    }

    /// Add a font with the given parameters.
    ///
    /// Note that replacing fonts is not yet supported.
    ///
    /// This will error if:
    /// - `font` is invalid
    pub fn add_font(&mut self, path: &str, font: AddFontParameter) -> MResult<()> {
        let font_path = Arc::new(path.to_owned());
        if self.fonts.contains_key(&font_path) {
            return Err(Error::from_data_error_string(format!("{path} already exists (replacing fonts is not yet supported)")))
        }

        font.validate()?;
        let font = Font::load_from_parameters(self, font)?;
        self.fonts.insert(font_path, font);
        Ok(())
    }

    /// Add a bitmap with the given parameters.
    ///
    /// Note that replacing bitmaps is not yet supported.
    ///
    /// This will error if:
    /// - `bitmap` is invalid
    /// - replacing a bitmap would break any dependencies (HUDs, shaders, etc.)
    pub fn add_bitmap(&mut self, path: &str, bitmap: AddBitmapParameter) -> MResult<()> {
        let bitmap_path = Arc::new(path.to_owned());
        if self.bitmaps.contains_key(&bitmap_path) {
            return Err(Error::from_data_error_string(format!("{path} already exists (replacing bitmaps is not yet supported)")))
        }

        bitmap.validate()?;
        let bitmap = Bitmap::load_from_parameters(self, bitmap)?;
        self.bitmaps.insert(bitmap_path, bitmap);
        Ok(())
    }

    /// Add a shader.
    ///
    /// Note that replacing shaders is not yet supported.
    ///
    /// This will error if:
    /// - `pipeline` is invalid
    /// - `pipeline` contains invalid dependencies
    /// - replacing a pipeline would break any dependencies
    pub fn add_shader(&mut self, path: &str, shader: AddShaderParameter) -> MResult<()> {
        let shader_path = Arc::new(path.to_owned());
        if self.shaders.contains_key(&shader_path) {
            return Err(Error::from_data_error_string(format!("{path} already exists (replacing shaders is not yet supported)")))
        }

        shader.validate(self)?;
        let shader = Shader::load_from_parameters(self, shader)?;
        self.shaders.insert(shader_path, shader);
        Ok(())
    }

    /// Add a geometry.
    ///
    /// Note that replacing geometries is not yet supported.
    ///
    /// This will error if:
    /// - `geometry` is invalid
    /// - `geometry` contains invalid dependencies
    /// - replacing a geometry would break any dependencies
    #[allow(unused_variables)]
    pub fn add_geometry(&mut self, path: &str, geometry: AddGeometryParameter) -> MResult<()> {
        todo!()
    }

    /// Add a sky.
    ///
    /// This will error if:
    /// - `sky` is invalid
    /// - `sky` contains invalid dependencies
    pub fn add_sky(&mut self, path: &str, sky: AddSkyParameter) -> MResult<()> {
        sky.validate(self)?;

        // tool.exe defaults 0.0 max density to 1.0, so fog should be disabled if both the start and
        // max distance are 0.0.

        let mut outdoor_fog = sky.outdoor_fog;
        let mut indoor_fog = sky.indoor_fog;

        if outdoor_fog.distance_to == 0.0 {
            outdoor_fog = FogData::default();
        }

        if indoor_fog.distance_to == 0.0 {
            indoor_fog = FogData::default();
        }

        self.skies.insert(Arc::new(path.to_owned()), Sky {
            geometry: sky.geometry.map(|s| self.geometries.get_key_value(&s).unwrap().0.clone()),
            outdoor_fog,
            indoor_fog
        });

        Ok(())
    }

    /// Add a BSP.
    ///
    /// Note that replacing BSPs is not yet supported.
    ///
    /// This will error if:
    /// - `bsp` is invalid
    /// - `bsp` contains invalid dependencies
    pub fn add_bsp(&mut self, path: &str, bsp: AddBSPParameter) -> MResult<()> {
        let bsp_path = Arc::new(path.to_owned());
        if self.bsps.contains_key(&bsp_path) {
            return Err(Error::from_data_error_string(format!("{path} already exists (replacing BSPs is not yet supported)")))
        }

        bsp.validate(self)?;
        let bsp = BSP::load_from_parameters(self, bsp)?;
        self.bsps.insert(bsp_path, Arc::new(bsp));
        Ok(())
    }

    /// Set the current BSP.
    ///
    /// If `path` is `None`, the BSP will be unloaded.
    ///
    /// Returns `Err` if `path` refers to a BSP that isn't loaded.
    pub fn set_current_bsp(&mut self, path: Option<&str>) -> MResult<()> {
        if let Some(p) = path {
            let key = self
                .bsps
                .keys()
                .find(|f| f.as_str() == p)
                .map(|b| b.clone());

            if key.is_none() {
                return Err(Error::from_data_error_string(format!("Can't set current BSP to {path:?}: that BSP is not loaded")))
            }

            self.current_bsp = key;
        }
        else {
            self.current_bsp = None;
        }

        Ok(())
    }

    /// Rebuild the swapchain.
    ///
    /// You must use this when the window is resized or if the swapchain is invalidated.
    pub fn rebuild_swapchain(&mut self, parameters: RendererParameters) -> MResult<()> {
        if parameters.resolution.height == 0 || parameters.resolution.width == 0 {
            return Err(Error::DataError { error: "resolution has 0 on one or more dimensions".to_owned() })
        }
        self.vulkan.rebuild_swapchain(
            &parameters
        )
    }

    /// Set the position, rotation, and FoV of the camera for the given viewport.
    ///
    /// `fov` must be in radians, and `position` must be a vector.
    ///
    /// # Panics
    ///
    /// Panics if `viewport >= self.viewport_count()` or if `!(camera.fov > 0.0 && camera.fov < PI)`
    pub fn set_camera_for_viewport(&mut self, viewport: usize, camera: Camera) {
        assert!(camera.fov > 0.0 && camera.fov < core::f32::consts::PI, "camera.fov is not between 0 (exclusive) and pi (exclusive)");

        let viewport = &mut self.player_viewports[viewport];
        if camera == viewport.camera {
            return;
        }

        // FIXME: determine how fast it is supposed to be transitioned here?
        let fog_transition_amount = Vec3::from(camera.position).distance(Vec3::from(viewport.camera.position)).min(10.0) / 10.0;
        if let Some(n) = viewport.viewport_fog.as_mut() {
            n.transition_amount = (n.transition_amount + fog_transition_amount).min(1.0);
        }

        viewport.camera = Camera {
            position: camera.position,
            rotation: Vec3::from(camera.rotation).try_normalize().unwrap_or(Vec3::new(0.0, 1.0, 0.0)).into(),
            fov: camera.fov,
            lightmaps: camera.lightmaps,
            fog: camera.fog
        };

        self.invalidate_debug_text();
    }

    /// Get the camera data for the given viewport.
    ///
    /// # Panics
    ///
    /// Panics if `viewport >= self.viewport_count()`
    pub fn get_camera_for_viewport(&self, viewport: usize) -> Camera {
        self.player_viewports[viewport].camera
    }

    /// Get the number of viewports.
    pub fn get_viewport_count(&self) -> usize {
        self.player_viewports.len()
    }

    /// Draw a frame.
    ///
    /// If `true`, the swapchain needs rebuilt.
    pub fn draw_frame(&mut self) -> MResult<bool> {
        if self.debug_text_stale {
            self.draw_debug_text()?;
        }
        self.fixup_fog_and_render_distances();
        let result = VulkanRenderer::draw_frame(self)?;

        self.update_frame_rate_counter();

        Ok(result)
    }

    /// Set whether debug info is displayed.
    ///
    /// Returns `Err` if the `font` is not loaded.
    pub fn set_debug_font(&mut self, font: Option<&str>) -> MResult<()> {
        match font {
            Some(font) => {
                let Some(font) = self.fonts.get_key_value(&font.to_owned()) else {
                    return Err(Error::from_data_error_string(format!("Font {font} is not loaded")))
                };
                self.debug_font = Some(font.0.clone())
            }
            None => {
                self.debug_font = None;
            }
        }

        self.invalidate_debug_text();
        Ok(())
    }

    pub fn invalidate_debug_text(&mut self) {
        self.debug_text_stale = true;
    }

    fn fixup_fog_and_render_distances(&mut self) {
        let Some(bsp) = self.current_bsp.as_ref().and_then(|b| self.bsps.get(b)) else { return };

        // First pass: get fog
        for viewport in &mut self.player_viewports {
            let Some(cluster) = bsp.bsp_data.find_cluster(viewport.camera.position) else {
                continue
            };

            let cluster = &bsp.bsp_data.clusters[cluster];
            let sky = cluster.sky.as_ref().and_then(|s| self.skies.get(s));

            let Some(viewport_fog) = viewport.viewport_fog.as_mut() else {
                let Some(sky) = sky else {
                    continue
                };
                viewport.viewport_fog = Some(ViewportFog {
                    current_fog_data: sky.outdoor_fog,
                    outdoor_fog_data: sky.outdoor_fog,
                    indoor_fog_data: sky.indoor_fog,
                    target_fog_data: sky.outdoor_fog,
                    transition_amount: 0.0
                });
                continue;
            };

            match sky {
                Some(sky) => {
                    viewport_fog.outdoor_fog_data = sky.outdoor_fog;
                    viewport_fog.indoor_fog_data = sky.indoor_fog;
                    viewport_fog.target_fog_data = sky.outdoor_fog;
                }
                None => {
                    viewport_fog.target_fog_data = viewport_fog.indoor_fog_data;
                }
            }
        }

        // Second pass: render distances and transitions
        for viewport in &mut self.player_viewports {
            viewport.draw_distance[0] = DRAW_DISTANCE_MINIMUM;
            if let Some(f) = viewport.viewport_fog.as_mut() {
                if f.transition_amount > 0.0 {
                    f.current_fog_data.distance_from = f.current_fog_data.distance_from.lerp(f.target_fog_data.distance_from, f.transition_amount);
                    f.current_fog_data.distance_to = f.current_fog_data.distance_to.lerp(f.target_fog_data.distance_to, f.transition_amount);
                    f.current_fog_data.min_opacity = f.current_fog_data.min_opacity.lerp(f.target_fog_data.min_opacity, f.transition_amount);
                    f.current_fog_data.max_opacity = f.current_fog_data.max_opacity.lerp(f.target_fog_data.max_opacity, f.transition_amount);
                    f.current_fog_data.color = Vec3::from(f.current_fog_data.color).lerp(Vec3::from(f.target_fog_data.color), f.transition_amount).to_array();
                    f.transition_amount = 0.0;
                }
                f.current_fog_data.normalize();
                if f.current_fog_data.max_opacity == 1.0 {
                    viewport.draw_distance[1] = bsp.draw_distance.min(f.current_fog_data.distance_to);
                    continue;
                }
            }
            viewport.draw_distance[1] = bsp.draw_distance;
        }
    }

    fn draw_debug_text(&mut self) -> MResult<()> {
        let Some(f) = self.debug_font.as_ref() else {
            return Ok(())
        };

        let font = self.fonts.get(f).expect("selected debug font no longer loaded?");

        let fps = self.fps_counter_value;
        let fps_ms = (1000.0 / fps) as f32;

        let max = 12.0;
        let min = 1.0;
        let half_high = 7.5;
        let half_low = 2.0;

        let color = if fps_ms > max {
            [1.0, 0.0, 0.0, 1.0]
        }
        else if fps_ms > half_high {
            let d = (fps_ms - half_high) / (max - half_high);
            [1.0, 1.0 - d*d*d, 0.0, 1.0]
        }
        else if fps_ms > half_low {
            let d = (fps_ms - half_low) / (half_high - half_low);
            [d.sqrt().sqrt(), 1.0, 0.0, 1.0]
        }
        else if fps_ms > min {
            let d = (fps_ms - min) / (half_low - min);
            [0.0, 1.0, 1.0 - d*d*d, 1.0]
        }
        else {
            [0.0, 1.0, 1.0, 1.0]
        };

        let request = FontDrawRequest {
            alignment: TextAlignment::Left,
            color,
            // TODO: determine how resolution will work
            ..FontDrawRequest::default()
        };

        let mut text = String::with_capacity(1024);

        std::fmt::write(&mut text, format_args!("FPS: {fps:-7.03} ({fps_ms} ms / frame)\n^7BSP: {bsp}\n\n",
                                                bsp=self.current_bsp.as_ref().map(|b| {
                                                    let bsp = b.as_str();
                                                    match bsp.rfind(".scenario_structure_bsp") {
                                                        Some(b) => &bsp[..b],
                                                        None => bsp
                                                    }
                                                }).unwrap_or("No BSP loaded!"))).unwrap();

        for (index, viewport) in self.player_viewports.iter().enumerate() {
            std::fmt::write(&mut text, format_args!("Viewport #{index}\n")).unwrap();
            std::fmt::write(&mut text, format_args!("  X:{:13.06}\n", viewport.camera.position[0])).unwrap();
            std::fmt::write(&mut text, format_args!("  Y:{:13.06}\n", viewport.camera.position[1])).unwrap();
            std::fmt::write(&mut text, format_args!("  Z:{:13.06}\n", viewport.camera.position[2])).unwrap();
            std::fmt::write(&mut text, format_args!("\n")).unwrap();
        }

        let mut vec = Vec::new();
        font.generate_string_draws(&text, request, &mut vec);
        let parameter = font.draw_string_buffer_to_bitmap(&vec, request);
        let bitmap = Bitmap::load_from_parameters(self, parameter)?;
        self.debug_text.push_back(bitmap);

        if self.debug_text.len() == self.debug_text.capacity() {
            self.debug_text.pop_front();
        }

        Ok(())
    }

    fn get_default_2d(&self, default_type: DefaultType) -> &BitmapBitmap {
        &self.bitmaps[&self.default_bitmaps.default_2d].bitmaps[default_type as usize]
    }
    fn get_default_cubemap(&self, default_type: DefaultType) -> &BitmapBitmap {
        &self.bitmaps[&self.default_bitmaps.default_cubemap].bitmaps[default_type as usize]
    }
    fn get_or_default_2d(&self, bitmap: &Option<String>, bitmap_index: usize, default_type: DefaultType) -> &BitmapBitmap {
        let bitmap = match bitmap.as_ref() {
            Some(n) => &self.bitmaps[n].bitmaps[bitmap_index],
            None => &self.get_default_2d(default_type)
        };
        debug_assert_eq!(BitmapType::Dim2D, bitmap.bitmap_type);
        bitmap
    }
    fn get_or_default_3d(&self, bitmap: &Option<String>, bitmap_index: usize, default_type: DefaultType) -> &BitmapBitmap {
        let bitmap = match bitmap.as_ref() {
            Some(n) => &self.bitmaps[n].bitmaps[bitmap_index],
            None => &self.bitmaps[&self.default_bitmaps.default_3d].bitmaps[default_type as usize]
        };
        debug_assert!(matches!(bitmap.bitmap_type, BitmapType::Dim3D { .. }));
        bitmap
    }
    fn get_or_default_cubemap(&self, bitmap: &Option<String>, bitmap_index: usize, default_type: DefaultType) -> &BitmapBitmap {
        let bitmap = match bitmap.as_ref() {
            Some(n) => &self.bitmaps[n].bitmaps[bitmap_index],
            None => &self.bitmaps[&self.default_bitmaps.default_cubemap].bitmaps[default_type as usize]
        };
        debug_assert_eq!(BitmapType::Cubemap, bitmap.bitmap_type);
        bitmap
    }
    fn update_frame_rate_counter(&mut self) {
        self.fps_counter_count = self.fps_counter_count.saturating_add(1);

        let now = Instant::now();
        let microseconds_since = (now - self.fps_counter_time).as_micros();
        if microseconds_since >= 1000000 {
            self.fps_counter_value = self.fps_counter_count as f64 / ((microseconds_since as f64) / 1000000.0);
            self.fps_counter_time = now;
            self.fps_counter_count = 0;
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
#[repr(usize)]
enum DefaultType {
    /// Describes a map with all channels set to 0x00.
    ///
    /// This provides a texture that does nothing on alpha blend, min, add, or subtract.
    Null,

    /// Describes a map with all channels set to 0xFF.
    ///
    /// This provides a texture that does nothing on multiply/min.
    White,

    /// Describes a map with red, green, and blue set to 0x7F and alpha set to 0xFF.
    ///
    /// This provides a texture that does nothing on double multiply.
    Gray,

    /// Describes a map with red and green set to 0x7F and blue and alpha set to 0xFF.
    ///
    /// This provides a neutral vector map.
    Vector
}

/// Describes the default background color and clear color.
const DEFAULT_BACKGROUND: FloatColor = [0.0f32, 0.0, 0.0, 1.0];
