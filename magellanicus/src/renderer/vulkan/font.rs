use vulkano::image::view::ImageView;
use std::sync::Arc;

pub struct VulkanCharacterData {
    pub image: Arc<ImageView>
}
