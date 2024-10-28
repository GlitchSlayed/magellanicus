use crate::error::{Error, MResult};
use crate::renderer::mipmap_iterator::{MipmapFaceIterator, MipmapMetadata, MipmapTextureIterator, MipmapType};
use crate::renderer::vulkan::{default_allocation_create_info, VulkanRenderer};
use crate::renderer::{decode_p8_to_a8r8g8b8le, AddBitmapBitmapParameter, BitmapFormat, BitmapType};
use std::num::NonZeroUsize;
use std::string::ToString;
use std::sync::Arc;
use std::vec::Vec;
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage, Subbuffer};
use vulkano::command_buffer::{AutoCommandBufferBuilder, BufferImageCopy, CommandBufferUsage, CopyBufferToImageInfo, PrimaryAutoCommandBuffer};
use vulkano::format::Format;
use vulkano::image::{Image, ImageAspects, ImageCreateFlags, ImageCreateInfo, ImageSubresourceLayers, ImageType, ImageUsage};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryAllocatePreference, MemoryTypeFilter};
use vulkano::DeviceSize;

pub struct VulkanBitmapData {
    pub image: Arc<Image>
}

impl VulkanBitmapData {
    pub fn new(vulkan_renderer: &mut VulkanRenderer, parameter: &AddBitmapBitmapParameter) -> MResult<Self> {
        let (image_type, depth) = match parameter.bitmap_type {
            BitmapType::Dim3D { depth } => (ImageType::Dim3d, depth),
            _ => (ImageType::Dim2d, 1)
        };

        let mut transcoded_pixels: Vec<u8> = Vec::new();

        let (bitmap_format, format, bytes) = match parameter.format {
            BitmapFormat::DXT1 => (parameter.format, Format::BC1_RGBA_UNORM_BLOCK, &parameter.data),
            BitmapFormat::DXT3 => (parameter.format, Format::BC2_UNORM_BLOCK, &parameter.data),
            BitmapFormat::DXT5 => (parameter.format, Format::BC3_UNORM_BLOCK, &parameter.data),
            BitmapFormat::BC7 => (parameter.format, Format::BC7_UNORM_BLOCK, &parameter.data),

            BitmapFormat::A8B8G8R8 => (parameter.format, Format::R8G8B8A8_UNORM, &parameter.data),
            BitmapFormat::A8R8G8B8 => (parameter.format, Format::B8G8R8A8_UNORM, &parameter.data),
            BitmapFormat::X8R8G8B8 => (parameter.format, Format::B8G8R8A8_UNORM, &parameter.data),
            BitmapFormat::R5G6B5 => (parameter.format, Format::R5G6B5_UNORM_PACK16, &parameter.data),
            BitmapFormat::A1R5G5B5 => (parameter.format, Format::A1R5G5B5_UNORM_PACK16, &parameter.data),
            BitmapFormat::B4G4R4A4 => (parameter.format, Format::B4G4R4A4_UNORM_PACK16, &parameter.data),
            BitmapFormat::A4R4G4B4 => {
                if vulkan_renderer.device.enabled_extensions().ext_4444_formats {
                    (parameter.format, Format::A4R4G4B4_UNORM_PACK16, &parameter.data)
                }
                else {
                    transcoded_pixels.reserve_exact(parameter.data.len());
                    for color in parameter.data.chunks_exact(2).map(|c| u16::from_le_bytes(c.try_into().unwrap())) {
                        let b = color & 0b1111;
                        let g = (color >> 4) & 0b1111;
                        let r = (color >> 8) & 0b1111;
                        let a = (color >> 12) & 0b1111;
                        let bgra = ((r << 4) | a) | (((b << 4) | g) << 8);
                        transcoded_pixels.extend_from_slice(&bgra.to_le_bytes());
                    }
                    (BitmapFormat::B4G4R4A4, Format::B4G4R4A4_UNORM_PACK16, &transcoded_pixels)
                }
            },
            BitmapFormat::R32G32B32A32SFloat => (parameter.format, Format::R32G32B32A32_SFLOAT, &parameter.data),

            BitmapFormat::A8 => {
                transcoded_pixels.reserve_exact(parameter.data.len() * 4);
                for pixel in parameter.data.iter() {
                    transcoded_pixels.push(0xFF);
                    transcoded_pixels.push(0xFF);
                    transcoded_pixels.push(0xFF);
                    transcoded_pixels.push(*pixel);
                }
                (BitmapFormat::A8R8G8B8, Format::B8G8R8A8_UNORM, &transcoded_pixels)
            },

            BitmapFormat::Y8 => {
                transcoded_pixels.reserve_exact(parameter.data.len() * 4);
                for pixel in parameter.data.iter() {
                    transcoded_pixels.push(*pixel);
                    transcoded_pixels.push(*pixel);
                    transcoded_pixels.push(*pixel);
                    transcoded_pixels.push(0xFF);
                }
                (BitmapFormat::A8R8G8B8, Format::B8G8R8A8_UNORM, &transcoded_pixels)
            },

            BitmapFormat::AY8 => {
                transcoded_pixels.reserve_exact(parameter.data.len() * 4);
                for pixel in parameter.data.iter() {
                    transcoded_pixels.push(*pixel);
                    transcoded_pixels.push(*pixel);
                    transcoded_pixels.push(*pixel);
                    transcoded_pixels.push(*pixel);
                }
                (BitmapFormat::A8R8G8B8, Format::B8G8R8A8_UNORM, &transcoded_pixels)
            },

            BitmapFormat::A8Y8 => {
                transcoded_pixels.reserve_exact(parameter.data.len() * 2);
                for p in parameter.data.chunks(2) {
                    let &[a, y] = p else {
                        unreachable!()
                    };
                    transcoded_pixels.push(y);
                    transcoded_pixels.push(y);
                    transcoded_pixels.push(y);
                    transcoded_pixels.push(a);
                }
                (BitmapFormat::A8R8G8B8, Format::B8G8R8A8_UNORM, &transcoded_pixels)
            },

            // TODO: P8
            BitmapFormat::P8 => {
                transcoded_pixels.reserve_exact(parameter.data.len() * 4);
                for pixel in parameter.data.iter() {
                    transcoded_pixels.extend_from_slice(&decode_p8_to_a8r8g8b8le(*pixel));
                }
                (BitmapFormat::A8R8G8B8, Format::B8G8R8A8_UNORM, &transcoded_pixels)
            }
        };

        let image = Image::new(
            vulkan_renderer.memory_allocator.clone(),
            ImageCreateInfo {
                image_type,
                format,
                extent: [parameter.resolution.width, parameter.resolution.height, depth],
                mip_levels: parameter.mipmap_count + 1,
                array_layers: if parameter.bitmap_type == BitmapType::Cubemap { 6 } else { 1 },
                usage: ImageUsage::TRANSFER_DST | ImageUsage::SAMPLED,
                flags: if parameter.bitmap_type == BitmapType::Cubemap {
                    ImageCreateFlags::CUBE_COMPATIBLE
                }
                else {
                    ImageCreateFlags::empty()
                },
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
                allocate_preference: MemoryAllocatePreference::AlwaysAllocate,
                ..Default::default()
            },
        )?;

        let upload_buffer = Buffer::new_slice(
            vulkan_renderer.memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::TRANSFER_SRC,
                ..Default::default()
            },
            default_allocation_create_info(),
            bytes.len() as DeviceSize,
        )?;

        upload_buffer
            .write()
            .map_err(|e| Error::from_vulkan_error(e.to_string()))?
            .copy_from_slice(bytes);

        let mut command_buffer_builder = AutoCommandBufferBuilder::primary(
            &vulkan_renderer.command_buffer_allocator,
            vulkan_renderer.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )?;

        // Simple bitmaps don't need iterated.
        if parameter.bitmap_type == BitmapType::Dim2D
            && parameter.mipmap_count == 0
            && parameter.format.block_pixel_length() == 1 {
            upload_image(
                &image,
                &upload_buffer,
                &mut command_buffer_builder,
                0,
                0,
                parameter.resolution.width,
                parameter.resolution.height,
                0,
                parameter.resolution.width,
                parameter.resolution.height,
                1
            )?;
            let buffer = command_buffer_builder.build()?;
            vulkan_renderer.execute_command_list(buffer);
            return Ok(Self { image })
        }

        let width_nzus = NonZeroUsize::new(parameter.resolution.width as usize).unwrap();
        let height_nzus = NonZeroUsize::new(parameter.resolution.height as usize).unwrap();
        let bitmap_type = match parameter.bitmap_type {
            BitmapType::Cubemap => MipmapType::Cubemap,
            BitmapType::Dim2D => MipmapType::TwoDimensional,
            BitmapType::Dim3D { depth } => MipmapType::ThreeDimensional(NonZeroUsize::new(depth as usize).unwrap())
        };
        let block_pixel_length_nzus = NonZeroUsize::new(bitmap_format.block_pixel_length()).unwrap();
        let mipmap_count = Some(parameter.mipmap_count as usize);

        let mut mipmap_face_iterator = MipmapFaceIterator::new(
            width_nzus,
            height_nzus,
            bitmap_type,
            block_pixel_length_nzus,
            mipmap_count,
        );

        let mut mipmap_texture_iterator = MipmapTextureIterator::new(
            width_nzus,
            height_nzus,
            bitmap_type,
            block_pixel_length_nzus,
            mipmap_count,
        );

        let iterator_to_use: &mut dyn Iterator<Item = MipmapMetadata> = if parameter.bitmap_type != BitmapType::Cubemap {
            &mut mipmap_texture_iterator
        }
        else {
            &mut mipmap_face_iterator
        };

        let mut offset = 0;
        let block_size = bitmap_format.block_byte_size();
        let pixel_size = bitmap_format.block_pixel_length();
        for i in iterator_to_use {
            let size = block_size * i.block_count;
            let actual_face_index = if parameter.bitmap_type != BitmapType::Cubemap {
                0
            }
            else {
                // TODO: IS THIS RIGHT?????? I THINK IT IS BUT IDK :(
                match i.face_index {
                    0 => 0,
                    1 => 2,
                    2 => 1,
                    3 => 3,
                    4 => 4,
                    5 => 5,
                    _ => continue
                }
            };

            let mip_height_physical = (i.block_height * pixel_size) as u32;
            let mip_width_physical = (i.block_width * pixel_size) as u32;
            let mip_level = i.mipmap_index as u32;
            let mip_width_logical = i.width as u32;
            let mip_height_logical = i.height as u32;
            let mip_depth_logical = i.depth as u32;

            upload_image(&image, &upload_buffer, &mut command_buffer_builder, offset, actual_face_index, mip_width_physical, mip_height_physical, mip_level, mip_width_logical, mip_height_logical, mip_depth_logical)?;

            offset += size as DeviceSize;
        }

        let buffer = command_buffer_builder.build()?;
        vulkan_renderer.execute_command_list(buffer);

        Ok(Self { image })
    }
}

fn upload_image(image: &Arc<Image>, upload_buffer: &Subbuffer<[u8]>, command_buffer_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>, offset: DeviceSize, actual_face_index: u32, mip_width_physical: u32, mip_height_physical: u32, mip_level: u32, mip_width_logical: u32, mip_height_logical: u32, mip_depth_logical: u32) -> Result<(), Error> {
    command_buffer_builder.copy_buffer_to_image(CopyBufferToImageInfo {
        regions: [
            BufferImageCopy {
                image_subresource: ImageSubresourceLayers {
                    aspects: ImageAspects::COLOR,
                    array_layers: actual_face_index..(actual_face_index + 1),
                    mip_level,
                },
                buffer_offset: offset,
                buffer_image_height: mip_height_physical,
                buffer_row_length: mip_width_physical,
                image_offset: [0, 0, 0],
                image_extent: [mip_width_logical, mip_height_logical, mip_depth_logical],
                ..Default::default()
            }
        ].into(),
        ..CopyBufferToImageInfo::buffer_image(
            upload_buffer.clone(),
            image.clone()
        )
    })?;
    Ok(())
}
