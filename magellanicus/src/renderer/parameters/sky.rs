use crate::error::{Error, MResult};
use crate::renderer::Renderer;

pub use crate::renderer::data::FogData;

pub struct AddSkyParameter {
    pub geometry: Option<String>,
    pub outdoor_fog: FogData,
    pub indoor_fog: FogData
}

impl AddSkyParameter {
    pub(crate) fn validate(&self, renderer: &Renderer) -> MResult<()> {
        self.outdoor_fog.validate()?;
        self.indoor_fog.validate()?;
        if let Some(s) = self.geometry.as_ref() {
            if !renderer.geometries.contains_key(s) {
                return Err(Error::from_data_error_string(format!("Fog references skybox geometry {s} which is not loaded")))
            }
        }
        Ok(())
    }
}
