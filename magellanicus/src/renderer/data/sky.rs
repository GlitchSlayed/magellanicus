use std::sync::Arc;
use crate::error::{Error, MResult};

pub struct Sky {
    pub geometry: Option<Arc<String>>,
    pub outdoor_fog: FogData,
    pub indoor_fog: FogData
}

#[derive(Copy, Clone, Debug)]
pub struct FogData {
    /// Current color in RGB.
    pub color: [f32; 3],

    /// Minimum distance that fog is applied.
    pub distance_from: f32,

    /// Maximum distance that fog is applied.
    pub distance_to: f32,

    /// Minimum opacity of fog (from 0.0 - 1.0).
    pub min_opacity: f32,

    /// Maximum opacity of fog (from 0.0 - 1.0).
    ///
    /// At 1.0, the render distance is set to `distance_from`.
    pub max_opacity: f32
}

impl FogData {
    pub(crate) fn validate(&self) -> MResult<()> {
        if let Some(c) = self.color.iter().find(|c| **c < 0.0 || **c > 1.0 || !(**c).is_finite()) {
            return Err(Error::from_data_error_string(format!("Invalid fog color channel value {c}")))
        }

        if self.distance_from < 0.0 || !self.distance_from.is_finite() {
            return Err(Error::from_data_error_string(format!("Invalid distance from {}", self.distance_from)))
        }

        if self.distance_to < self.distance_from || !self.distance_to.is_finite() {
            return Err(Error::from_data_error_string(format!("Invalid distance to {}", self.distance_to)))
        }

        if self.min_opacity < 0.0 || self.min_opacity > 1.0 || !self.min_opacity.is_finite() {
            return Err(Error::from_data_error_string(format!("Invalid min opacity {}", self.min_opacity)))
        }

        if self.max_opacity < self.min_opacity || self.max_opacity > 1.0 || !self.max_opacity.is_finite() {
            return Err(Error::from_data_error_string(format!("Invalid max opacity {}", self.max_opacity)))
        }

        Ok(())
    }

    pub(crate) fn normalize(&mut self) {
        self.color[0] = self.color[0].clamp(0.0, 1.0);
        self.color[1] = self.color[1].clamp(0.0, 1.0);
        self.color[2] = self.color[2].clamp(0.0, 1.0);
        self.distance_from = self.distance_from.clamp(0.0, f32::MAX);
        self.distance_to = self.distance_to.clamp(self.distance_from, f32::MAX);
        self.min_opacity = self.min_opacity.clamp(0.0, 1.0);
        self.max_opacity = self.max_opacity.clamp(self.min_opacity, 1.0);
    }
}

impl Default for FogData {
    fn default() -> Self {
        Self {
            color: [0.0, 0.0, 0.0],
            distance_from: 0.0,
            distance_to: 1.0,
            min_opacity: 0.0,
            max_opacity: 0.0
        }
    }
}
