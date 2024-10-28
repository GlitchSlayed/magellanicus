use crate::error::{Error, MResult};
pub use crate::renderer::data::ShaderType;
use crate::renderer::{BitmapType, Renderer};
use crate::renderer::data::Bitmap;

pub const MAX_SHADER_TRANSPARENT_CHICAGO_MAPS: usize = 4;

pub struct AddShaderParameter {
    pub data: AddShaderData
}

impl AddShaderParameter {
    pub(crate) fn validate(&self, renderer: &Renderer) -> MResult<()> {
        match &self.data {
            AddShaderData::BasicShader(AddShaderBasicShaderData { bitmap, .. }) => {
                if let Some(bitmap) = bitmap {
                    if !renderer.bitmaps.contains_key(bitmap) {
                        return Err(Error::DataError { error: format!("Referenced bitmap {bitmap} is not loaded.") })
                    }
                }
            },
            AddShaderData::ShaderEnvironment(shader_data) => {
                shader_data.validate(renderer)?;
            },
            AddShaderData::ShaderTransparentChicago(shader_data) => {
                shader_data.validate(renderer)?;
            }
        }
        Ok(())
    }
}

pub enum AddShaderData {
    /// Basic pipeline that just renders a single texture. This does not map to an actual tag group
    /// and is to be removed once all shaders are implemented
    BasicShader(AddShaderBasicShaderData),

    /// Renders a shader_environment texture.
    ShaderEnvironment(AddShaderEnvironmentShaderData),

    /// Renders a shader_transparent_chicago texture.
    ShaderTransparentChicago(AddShaderTransparentChicagoShaderData)
}

pub struct AddShaderBasicShaderData {
    pub bitmap: Option<String>,
    pub shader_type: ShaderType,
    pub alpha_tested: bool
}

#[derive(Copy, Clone, PartialEq)]
#[repr(u32)]
pub enum ShaderEnvironmentType {
    Normal,
    Blended,
    BlendedBaseSpecular
}

#[derive(Copy, Clone, PartialEq)]
#[repr(u32)]
pub enum ShaderEnvironmentMapFunction {
    DoubleBiasedMultiply,
    Multiply,
    DoubleBiasedAdd
}

#[derive(Copy, Clone, PartialEq)]
#[repr(u32)]
pub enum ShaderReflectionType {
    BumpedCubeMap,
    FlatCubeMap,
    BumpedRadiosity
}

#[derive(Clone)]
pub struct AddShaderEnvironmentShaderData {
    pub alpha_tested: bool,
    pub bump_map_is_specular_mask: bool,

    pub shader_environment_type: ShaderEnvironmentType,
    pub base_map: Option<String>,

    pub detail_map_function: ShaderEnvironmentMapFunction,
    pub primary_detail_map: Option<String>,
    pub primary_detail_map_scale: f32,
    pub secondary_detail_map: Option<String>,
    pub secondary_detail_map_scale: f32,

    pub micro_detail_map: Option<String>,
    pub micro_detail_map_scale: f32,
    pub micro_detail_map_function: ShaderEnvironmentMapFunction,

    pub bump_map: Option<String>,
    pub bump_map_scale: f32,

    pub reflection_cube_map: Option<String>,
    pub reflection_type: ShaderReflectionType,

    pub perpendicular_color: [f32; 3],
    pub perpendicular_brightness: f32,
    pub parallel_color: [f32; 3],
    pub parallel_brightness: f32,
}
impl AddShaderEnvironmentShaderData {
    pub(crate) fn validate(&self, renderer: &Renderer) -> MResult<()> {
        check_bitmap(renderer, &self.base_map, BitmapType::Dim2D, "base map")?;
        check_bitmap(renderer, &self.primary_detail_map, BitmapType::Dim2D, "primary detail map")?;
        check_bitmap(renderer, &self.secondary_detail_map, BitmapType::Dim2D, "secondary detail map")?;
        check_bitmap(renderer, &self.micro_detail_map, BitmapType::Dim2D, "micro detail map")?;
        check_bitmap(renderer, &self.bump_map, BitmapType::Dim2D, "bump map")?;
        check_bitmap(renderer, &self.reflection_cube_map, BitmapType::Cubemap, "reflection cube map")?;
        Ok(())
    }
}

pub struct AddShaderTransparentChicagoShaderData {
    pub two_sided: bool,
    pub first_map_type: ShaderTransparentChicagoFirstMapType,
    pub framebuffer_method: ShaderTransparentChicagoFramebufferFunction,
    pub maps: Vec<AddShaderTransparentChicagoShaderMap>
}

impl AddShaderTransparentChicagoShaderData {
    pub(crate) fn validate(&self, renderer: &Renderer) -> MResult<()> {
        if self.maps.len() > MAX_SHADER_TRANSPARENT_CHICAGO_MAPS {
            return Err(Error::from_data_error_string(format!("Maximum number of maps ({MAX_SHADER_TRANSPARENT_CHICAGO_MAPS}) exceeded")))
        }

        if self.maps.is_empty() {
            return Err(Error::from_data_error_string("No maps given...".to_owned()))
        }

        for (index, map) in self.maps.iter().enumerate() {
            let expected_type = if index != 0 || self.first_map_type == ShaderTransparentChicagoFirstMapType::Dim2D {
                BitmapType::Dim2D
            }
            else {
                BitmapType::Cubemap
            };

            check_bitmap(renderer, &map.bitmap, expected_type, &format!("map {index}"))?;
        }

        Ok(())
    }
}

#[derive(Default, Clone)]
pub struct AddShaderTransparentChicagoShaderMap {
    pub bitmap: Option<String>,
    pub color_function: ShaderColorFunction,
    pub alpha_function: ShaderColorFunction,
    pub uv_scale: [f32; 2],
    pub uv_offset: [f32; 2],
    pub alpha_replicate: bool
}

#[derive(PartialEq)]
#[repr(u32)]
pub enum ShaderTransparentChicagoFirstMapType {
    Dim2D,
    ReflectionCubemap,
    ObjectCenteredCubemap,
    ViewerCenteredCubemap,
}

#[repr(u32)]
pub enum ShaderTransparentChicagoFramebufferFunction {
    /// framebuffer.rgb = mix(framebuffer.rgb, pixel.rgb, pixel.a)
    AlphaBlend,

    /// framebuffer.rgb *= pixel.rgb
    Multiply,

    /// framebuffer.rgb *= pixel.rgb * pixel.rgb (???)
    DoubleMultiply,

    /// framebuffer.rgb += pixel.rgb
    Add,

    /// framebuffer.rgb -= pixel.rgb
    Subtract,

    /// framebuffer.rgb = min(framebuffer.rgb, pixel.rgb)
    ComponentMin,

    /// framebuffer.rgb = max(framebuffer.rgb, pixel.rgb)
    ComponentMax,

    /// framebuffer.rgb += pixel.rgb * pixel.a
    AlphaMultiplyAdd
}

#[derive(Default, Clone)]
#[repr(u32)]
pub enum ShaderColorFunction {
    #[default]
    Current,
    NextMap,
    Multiply,
    DoubleMultiply,
    Add,
    AddSignedCurrent,
    AddSignedNextMap,
    SubtractCurrent,
    SubtractNextMap,
    BlendCurrentAlpha,
    BlendCurrentAlphaInverse,
    BlendNextMapAlpha,
    BlendNextMapAlphaInverse
}

fn check_bitmap(renderer: &Renderer, reference: &Option<String>, bitmap_type: BitmapType, name: &str) -> MResult<()> {
    let Some(bitmap_path) = reference.as_ref() else {
        return Ok(())
    };

    let Some(bitmap) = renderer.bitmaps.get(bitmap_path) else {
        return Err(Error::from_data_error_string(format!("{name} {bitmap_path} is not loaded")))
    };

    expect_bitmap_or_else(bitmap, bitmap_type, name)
}

fn expect_bitmap_or_else(bitmap: &Bitmap, bitmap_type: BitmapType, name: &str) -> MResult<()> {
    let Some((bad_index, bad_bitmap)) = bitmap.bitmaps
        .iter()
        .enumerate()
        .find(|a| a.1.bitmap_type != bitmap_type) else {
        return Ok(())
    };

    Err(Error::from_data_error_string(format!("Bitmap #{bad_index} of {name} is {:?}, expected {bitmap_type:?}", bad_bitmap.bitmap_type)))
}
