use alloc::string::String;

mod bitmap;
mod geometry;
mod pipeline;
mod bsp;
mod sky;
mod helper;
mod player_viewport;
mod vertex;
mod material;

use crate::error::{Error, MResult};
use crate::renderer::data::{BSPGeometry, BSP};
use crate::renderer::vulkan::helper::{build_swapchain, LoadedVulkan};
use crate::renderer::vulkan::vertex::{VulkanFogData, VulkanModelData, VulkanModelVertex};
use crate::renderer::{Camera, Renderer, RendererParameters, Resolution, MSAA};
pub use bitmap::*;
pub use bsp::*;
pub use geometry::*;
use glam::{Mat3, Mat4, Vec3};
pub use material::*;
pub use pipeline::*;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use std::boxed::Box;
use std::collections::BTreeMap;
use std::fmt::Display;
use std::sync::Arc;
use std::time::Duration;
use std::vec::Vec;
use std::{eprintln, format, println, vec};
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage};
use vulkano::command_buffer::allocator::{StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo};
use vulkano::command_buffer::{AutoCommandBufferBuilder, BlitImageInfo, ClearDepthStencilImageInfo, CommandBufferInheritanceInfo, CommandBufferInheritanceRenderPassType, CommandBufferInheritanceRenderingInfo, CommandBufferUsage, PrimaryAutoCommandBuffer, PrimaryCommandBufferAbstract, RenderPassBeginInfo, RenderingAttachmentInfo, RenderingInfo, ResolveImageInfo, SecondaryAutoCommandBuffer, SubpassBeginInfo, SubpassContents, SubpassEndInfo};
use vulkano::descriptor_set::allocator::{StandardDescriptorSetAllocator, StandardDescriptorSetAllocatorCreateInfo};
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::device::{Device, DeviceOwned, Queue};
use vulkano::format::{ClearDepthStencilValue, Format};
use vulkano::image::sampler::{Filter, Sampler, SamplerCreateInfo};
use vulkano::image::view::ImageView;
use vulkano::image::{Image, ImageCreateInfo, ImageType, ImageUsage, SampleCount};
use vulkano::instance::Instance;
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator};
use vulkano::padded::Padded;
use vulkano::pipeline::graphics::rasterization::CullMode;
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::pipeline::{Pipeline, PipelineBindPoint};
use vulkano::render_pass::{AttachmentLoadOp, AttachmentStoreOp, Framebuffer, FramebufferCreateInfo};
use vulkano::swapchain::{acquire_next_image, Surface, Swapchain, SwapchainAcquireFuture, SwapchainCreateInfo, SwapchainPresentInfo};
use vulkano::sync::GpuFuture;
use vulkano::{single_pass_renderpass, Validated, ValidationError, VulkanError};

pub(crate) static OFFLINE_PIPELINE_COLOR_FORMAT: Format = Format::R8G8B8A8_UNORM;

pub struct VulkanRenderer {
    current_resolution: Resolution,
    instance: Arc<Instance>,
    device: Arc<Device>,
    memory_allocator: Arc<StandardMemoryAllocator>,
    command_buffer_allocator: StandardCommandBufferAllocator,
    descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,
    queue: Arc<Queue>,
    future: Option<Box<dyn GpuFuture + Send + Sync>>,
    pipelines: BTreeMap<VulkanPipelineType, Arc<dyn VulkanPipelineData>>,
    swapchain: Arc<Swapchain>,
    surface: Arc<Surface>,
    swapchain_image_views: Vec<Arc<SwapchainImages>>,
    default_2d_sampler: Arc<Sampler>,
    samples_per_pixel: SampleCount
}

#[derive(Clone)]
pub struct SwapchainImages {
    output: Arc<ImageView>,
    color: Arc<ImageView>,
    depth: Arc<ImageView>,
    resolve: Option<Arc<ImageView>>,
    framebuffer: Option<Arc<Framebuffer>>
}

impl SwapchainImages {
    fn begin_rendering(&self, command_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>) {
        if let Some(n) = self.framebuffer.as_ref() {
            let begin_render_pass = RenderPassBeginInfo {
                clear_values: vec![None, None],
                ..RenderPassBeginInfo::framebuffer(n.clone())
            };
            let begin_subpass = SubpassBeginInfo {
                contents: SubpassContents::Inline,
                ..Default::default()
            };
            command_builder.begin_render_pass(begin_render_pass, begin_subpass).expect("failed to begin render pass");
        }
        else {
            command_builder.begin_rendering(RenderingInfo {
                color_attachments: vec![Some(RenderingAttachmentInfo {
                    load_op: AttachmentLoadOp::Load,
                    store_op: AttachmentStoreOp::Store,
                    ..RenderingAttachmentInfo::image_view(self.color.clone())
                })],
                depth_attachment: Some(RenderingAttachmentInfo {
                    load_op: AttachmentLoadOp::Load,
                    store_op: AttachmentStoreOp::Store,
                    ..RenderingAttachmentInfo::image_view(self.depth.clone())
                }),
                ..Default::default()
            }).expect("failed to begin rendering");
        }
    }
    fn end_rendering(&self, command_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>) {
        if self.framebuffer.is_some() {
            command_builder.end_render_pass(SubpassEndInfo::default()).expect("failed to end render pass");
        }
        else {
            command_builder.end_rendering().expect("failed to end rendering");
        }
    }
}

impl VulkanRenderer {
    pub unsafe fn new(
        renderer_parameters: &RendererParameters,
        surface: &(impl HasRawWindowHandle + HasRawDisplayHandle)
    ) -> MResult<Self> {
        let LoadedVulkan { device, instance, surface, queue} = helper::load_vulkan_and_get_queue(surface, renderer_parameters.anisotropic_filtering)?;

        let samples_per_pixel = match renderer_parameters.msaa {
            MSAA::NoMSAA => SampleCount::Sample1,
            MSAA::MSAA2x => SampleCount::Sample2,
            MSAA::MSAA4x => SampleCount::Sample4,
            MSAA::MSAA8x => SampleCount::Sample8,
            MSAA::MSAA16x => SampleCount::Sample16,
            MSAA::MSAA32x => SampleCount::Sample32,
            MSAA::MSAA64x => SampleCount::Sample64
        };

        if let Some(n) = renderer_parameters.anisotropic_filtering {
            let max = device.physical_device().properties().max_sampler_anisotropy;
            if max < n || n < 1.0 {
                return Err(
                    Error::from_vulkan_impl_error(format!("{n}x AF is unsupported by your device; supported values are 1-{max}"))
                )
            }
        }

        let color = device.physical_device().properties().sampled_image_color_sample_counts;
        let depth = device.physical_device().properties().sampled_image_depth_sample_counts;
        let intersection = color & depth;
        if !intersection.contains_enum(samples_per_pixel) {
            return Err(
                Error::from_vulkan_impl_error(format!("{}x MSAA is unsupported by your device; only these are supported:{}",
                                                      renderer_parameters.msaa as u32,
                                                      intersection.into_iter().map(|s| format!(" {}", s as u32)).collect::<String>())));
        }

        let command_buffer_allocator = StandardCommandBufferAllocator::new(
            device.clone(),
            StandardCommandBufferAllocatorCreateInfo {
                primary_buffer_count: 32,
                secondary_buffer_count: 0,
                ..Default::default()
            }
        );

        let descriptor_set_allocator = Arc::new(StandardDescriptorSetAllocator::new(
            device.clone(),
            StandardDescriptorSetAllocatorCreateInfo {
                set_count: 16 * 1024,
                ..Default::default()
            }
        ));

        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));
        let future = Some(vulkano::sync::now(device.clone()).boxed_send_sync());

        let output_format = device
            .physical_device()
            .surface_formats(surface.as_ref(), Default::default())?[0]
            .0;

        let (swapchain, swapchain_images) = build_swapchain(device.clone(), surface.clone(), output_format, renderer_parameters)?;

        let swapchain_image_views = Self::make_swapchain_images(swapchain_images, memory_allocator.clone(), samples_per_pixel, renderer_parameters.render_scale);
        let pipelines = load_all_pipelines(&swapchain_image_views[0], device.clone())?;

        let default_2d_sampler = Sampler::new(
            device.clone(),
            SamplerCreateInfo {
                anisotropy: renderer_parameters.anisotropic_filtering,
                ..SamplerCreateInfo::simple_repeat_linear()
            }
        )?;

        Ok(Self {
            current_resolution: renderer_parameters.resolution,
            instance,
            command_buffer_allocator,
            descriptor_set_allocator,
            device,
            queue,
            future,
            pipelines,
            swapchain,
            surface,
            swapchain_image_views,
            memory_allocator,
            default_2d_sampler,
            samples_per_pixel
        })
    }

    pub fn draw_frame(renderer: &mut Renderer) -> MResult<bool> {
        let vulkan_renderer = &mut renderer.renderer;

        let (image_index, suboptimal, acquire_future) =
            match acquire_next_image(vulkan_renderer.swapchain.clone(), None).map_err(Validated::unwrap) {
                Ok(r) => r,
                Err(VulkanError::OutOfDate) => return Ok(false),
                Err(e) => panic!("failed to acquire next image: {e}"),
            };

        Ok(Self::draw_frame_infallible(renderer, image_index, acquire_future) && !suboptimal)
    }

    pub fn rebuild_swapchain(&mut self, renderer_parameters: &RendererParameters) -> MResult<()> {
        let (swapchain, swapchain_images) = self.swapchain.recreate(
            SwapchainCreateInfo {
                image_extent: [renderer_parameters.resolution.width, renderer_parameters.resolution.height],
                ..self.swapchain.create_info()
            }
        )?;

        self.swapchain = swapchain;
        self.swapchain_image_views = Self::make_swapchain_images(swapchain_images, self.memory_allocator.clone(), self.samples_per_pixel, renderer_parameters.render_scale);
        self.current_resolution = renderer_parameters.resolution;
        self.pipelines = load_all_pipelines(&self.swapchain_image_views[0], self.device.clone()).expect("failed to reload pipelines...");

        Ok(())
    }

    fn make_swapchain_images(swapchain_images: Vec<Arc<Image>>, memory_allocator: Arc<StandardMemoryAllocator>, samples_per_pixel: SampleCount, render_scale: f32) -> Vec<Arc<SwapchainImages>> {
        assert!(render_scale > 0.0);

        let device = memory_allocator.device();

        swapchain_images.iter().map(|i| {
            let native_width = i.extent()[0];
            let native_height = i.extent()[1];

            let width;
            let height;
            if render_scale != 1.0 {
                let max_width = device.physical_device().properties().max_framebuffer_width;
                let max_height = device.physical_device().properties().max_framebuffer_height;

                let attempted_width = ((native_width as f32) * render_scale.sqrt()) as u32;
                let attempted_height = ((native_height as f32) * render_scale.sqrt()) as u32;

                width = attempted_width.clamp(1, max_width);
                height = attempted_height.clamp(1, max_height);

                if width != attempted_width || height != attempted_height {
                    eprintln!("Resolution {attempted_width}x{attempted_height} is not supported by the GPU... resizing");
                }
            }
            else {
                width = native_width;
                height = native_height;
            }

            println!("Render resolution: {width}x{height} ({native_width}x{native_height}x{:.02}%)", render_scale * 100.0);

            let output = ImageView::new_default(i.clone()).unwrap();
            let color = ImageView::new_default(Image::new(
                memory_allocator.clone(),
                ImageCreateInfo {
                    extent: [width, height, 1],
                    format: OFFLINE_PIPELINE_COLOR_FORMAT,
                    image_type: ImageType::Dim2d,
                    samples: samples_per_pixel,
                    usage: ImageUsage::TRANSFER_SRC | ImageUsage::COLOR_ATTACHMENT,
                    ..Default::default()
                },
                AllocationCreateInfo::default(),
            ).unwrap()).unwrap();

            let depth = ImageView::new_default(Image::new(
                memory_allocator.clone(),
                ImageCreateInfo {
                    extent: [width, height, 1],
                    format: Format::D32_SFLOAT,
                    image_type: ImageType::Dim2d,
                    samples: samples_per_pixel,
                    usage: ImageUsage::DEPTH_STENCIL_ATTACHMENT | ImageUsage::TRANSFER_DST,
                    ..Default::default()
                },
                AllocationCreateInfo::default(),
            ).unwrap()).unwrap();

            let resolve = if samples_per_pixel != SampleCount::Sample1 {
                Some(ImageView::new_default(Image::new(
                    memory_allocator.clone(),
                    ImageCreateInfo {
                        extent: [width, height, 1],
                        format: OFFLINE_PIPELINE_COLOR_FORMAT,
                        image_type: ImageType::Dim2d,
                        samples: SampleCount::Sample1,
                        usage: ImageUsage::TRANSFER_SRC | ImageUsage::TRANSFER_DST | ImageUsage::COLOR_ATTACHMENT,
                        ..Default::default()
                    },
                    AllocationCreateInfo::default(),
                ).unwrap()).unwrap())
            } else {
                None
            };

            let framebuffer = if !device.enabled_extensions().khr_dynamic_rendering {
                let color_format = color.image().format();
                let depth_format = depth.image().format();
                let samples = color.image().samples();

                let render_pass = single_pass_renderpass!(
                    device.clone(),
                    attachments: {
                        color: {
                            format: color_format,
                            samples: samples,
                            load_op: Load,
                            store_op: Store,
                        },
                        depth_stencil: {
                            format: depth_format,
                            samples: samples,
                            load_op: Load,
                            store_op: DontCare,
                        }
                    },
                    pass: {
                        color: [color],
                        depth_stencil: {depth_stencil},
                    },
                ).expect("failed to make render pass");

                let framebuffer = Framebuffer::new(render_pass, FramebufferCreateInfo {
                    attachments: vec![
                        color.clone(),
                        depth.clone()
                    ],
                    extent: [width, height],
                    ..Default::default()
                }).expect("failed to make framebuffer");

                Some(framebuffer)
            }
            else {
                None
            };

            Arc::new(SwapchainImages {
                output,
                color,
                depth,
                resolve,
                framebuffer
            })
        }).collect()
    }

    fn draw_frame_infallible(renderer: &mut Renderer, image_index: u32, image_future: SwapchainAcquireFuture) -> bool {
        let currently_loaded_bsp = renderer
            .current_bsp
            .as_ref()
            .and_then(|f| renderer.bsps.get(f))
            .map(|b| b.clone());

        let mut command_builder = AutoCommandBufferBuilder::primary(
            &renderer.renderer.command_buffer_allocator,
            renderer.renderer.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit
        ).expect("failed to init command builder");

        let images = renderer.renderer.swapchain_image_views[image_index as usize].clone();
        image_future.wait(Some(Duration::from_millis(5000))).expect("waited too long");
        renderer.renderer.future.as_mut().unwrap().cleanup_finished();

        let [width, height, ..] = images.color.image().extent();
        let (width, height) = (width as f32, height as f32);

        command_builder.clear_depth_stencil_image(ClearDepthStencilImageInfo {
            clear_value: ClearDepthStencilValue::from(1.0),
            ..ClearDepthStencilImageInfo::image(images.depth.clone().image().clone())
        }).expect("failed to clear depth image");

        for i in 0..renderer.player_viewports.len() {
            let player_viewport = &renderer.player_viewports[i];

            let viewport = Viewport {
                offset: [player_viewport.rel_x * width, player_viewport.rel_y * height],
                extent: [player_viewport.rel_width * width, player_viewport.rel_height * height],
                depth_range: 0.0..=1.0,
            };

            Self::draw_viewport(
                renderer,
                &images,
                viewport,
                &currently_loaded_bsp,
                &mut command_builder,
                player_viewport.camera.clone()
            );
        }

        if renderer.player_viewports.len() > 1 {
            images.begin_rendering(&mut command_builder);
            Self::draw_split_screen_bars(renderer, &mut command_builder, width, height);
            images.end_rendering(&mut command_builder);
        }

        let staging_image = if let Some(resolved_color_view) = images.resolve.as_ref().map(|iv| iv.image()) {
            command_builder.resolve_image(
                ResolveImageInfo::images(images.color.image().clone(), resolved_color_view.clone())
            ).expect("resolve fail");
            resolved_color_view
        }
        else {
            images.color.image()
        };

        command_builder.blit_image(BlitImageInfo {
            filter: Filter::Linear,
            ..BlitImageInfo::images(staging_image.clone(), images.output.image().clone())
        }).unwrap();

        let commands = command_builder.build().expect("failed to build command builder");

        let future = renderer.renderer
            .future
            .take()
            .expect("there's no future :(");

        let swapchain_present = SwapchainPresentInfo::swapchain_image_index(renderer.renderer.swapchain.clone(), image_index);

        let future = future
            .join(image_future)
            .then_execute(renderer.renderer.queue.clone(), commands.clone())
            .expect("can't execute commands")
            .then_swapchain_present(renderer.renderer.queue.clone(), swapchain_present)
            .then_signal_fence();

        loop {
            match future.flush() {
                Ok(()) => break,
                #[cfg(target_os = "macos")]
                Err(Validated::ValidationError(v)) if v.problem.starts_with("access to a resource has been denied") => {
                    // Workaround for macOS.
                    //
                    // Sometimes even though we called cleanup_finished() and waited for the swapchain image, the images
                    // are still considered in use when they clearly shouldn't be.
                    continue;
                },
                Err(Validated::Error(VulkanError::OutOfDate)) => {
                    renderer.renderer.future = Some(vulkano::sync::now(renderer.renderer.device.clone()).boxed_send_sync());
                    return false
                },
                Err(e) => {
                    panic!("Oh, shit! Some bullshit just happened: {e:?}")
                }
            }
        }

        renderer.renderer.future = Some(future.boxed_send_sync());
        true
    }

    fn draw_viewport(
        renderer: &mut Renderer,
        images: &Arc<SwapchainImages>,
        viewport: Viewport,
        currently_loaded_bsp: &Option<Arc<BSP>>,
        command_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
        camera: Camera
    ) {
        command_builder.set_viewport(0, [viewport.clone()].into_iter().collect()).unwrap();
        images.begin_rendering(command_builder);

        let aspect_ratio = viewport.extent[0] / viewport.extent[1];
        let z_near = 0.0625;

        let fog_data;
        let mut z_far;
        if let Some(bsp) = currently_loaded_bsp {
            let cluster_index = bsp.bsp_data.find_cluster(camera.position);
            let cluster = cluster_index.map(|c| &bsp.bsp_data.clusters[c]);
            let sky = cluster.and_then(|c| c.sky.as_ref()).and_then(|s| renderer.skies.get(s));

            z_far = bsp.draw_distance;
            if !camera.fog || sky.is_none() {
                fog_data = FogData::default();
            }
            else {
                let sky = sky.unwrap();

                // TODO: determine which fog color
                fog_data = FogData {
                    color: [sky.outdoor_fog_color[0], sky.outdoor_fog_color[1], sky.outdoor_fog_color[2], 0.0],
                    distance_from: sky.outdoor_fog_start_distance,
                    distance_to: sky.outdoor_fog_opaque_distance,
                    min_opacity: 0.0,
                    max_opacity: sky.outdoor_fog_maximum_density,
                };

                // Occlude things that won't be visible anyway
                if fog_data.max_opacity == 1.0 {
                    z_far = z_far.min(fog_data.distance_to);
                }
            }

            let sky_color = [fog_data.color[0], fog_data.color[1], fog_data.color[2], 1.0];
            draw_box(
                renderer,
                0.0,
                0.0,
                1.0,
                1.0,
                sky_color,
                command_builder
            ).unwrap();
        }
        else {
            z_far = 2250.0;
            fog_data = FogData::default();
        }

        z_far = z_far.max(z_near + 1.0);
        let proj = Mat4::perspective_lh(
            camera.fov,
            aspect_ratio,
            z_near,
            z_far
        );
        let view = Mat4::look_to_lh(
            camera.position.into(),
            camera.rotation.into(),
            Vec3::new(0.0, 0.0, -1.0)
        );

        let fog = make_fog_uniform(renderer, &fog_data);

        let mut transparent_geometries: Vec<(usize, f32)> = Vec::with_capacity(256);

        if let Some(bsp) = currently_loaded_bsp {
            let mvp = make_model_view_uniform(renderer, camera.position.into(), Vec3::default(), Mat3::IDENTITY, view, proj);

            // Draw non-transparent shaders first
            let mut last_shader = None;

            let get_geometry_shader = |f: &usize| (&bsp.geometries[*f], &renderer.shaders[&bsp.geometries[*f].vulkan.shader].vulkan.pipeline_data);

            for (geometry, shader) in bsp
                .vulkan
                .opaque_geometries
                .iter()
                .map(get_geometry_shader) {
                Self::draw_bsp_geometry(renderer, bsp, command_builder, &camera, &mut last_shader, geometry, fog.clone(), mvp.clone(), shader);
            }

            transparent_geometries.extend(bsp
                .vulkan
                .transparent_geometries
                .iter()
                .map(|i| (*i, Vec3::from(camera.position).distance_squared(Vec3::from(bsp.geometries[*i].centroid))))
            );
            transparent_geometries
                .sort_by(|a,b| b.1.total_cmp(&a.1));

            for (geometry, shader) in transparent_geometries
                .iter()
                .map(|b| &b.0)
                .map(get_geometry_shader) {
                if geometry.vulkan.shader.ends_with("water") {
                    // FIXME: water is not yet supported and the fallback shader is broken for it; should be fixed later
                    continue;
                }
                Self::draw_bsp_geometry(renderer, bsp, command_builder, &camera, &mut last_shader, geometry, fog.clone(), mvp.clone(), shader);
            }
        }

        images.end_rendering(command_builder);
    }

    fn draw_bsp_geometry<'a, 'b>(
        renderer: &Renderer,
        currently_loaded_bsp: &'a BSP,
        mut command_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
        camera: &Camera,
        last_shader: &'b mut Option<&'a Arc<String>>,
        geometry: &'a BSPGeometry,
        fog_data: Arc<PersistentDescriptorSet>,
        mvp: Arc<PersistentDescriptorSet>,
        shader: &Arc<dyn VulkanMaterial>
    ) {
        let this_shader = &geometry.vulkan.shader;
        let repeat_shader = if *last_shader != Some(this_shader) && shader.can_reuse_descriptors() {
            false
        }
        else {
            true
        };
        *last_shader = Some(this_shader);

        let main_pipeline = renderer.renderer.pipelines.get(&shader.get_main_pipeline()).unwrap();
        let mut desired_lightmap = geometry.lightmap_index;
        if !camera.lightmaps {
            desired_lightmap = None;
        }

        if !repeat_shader {
            command_builder
                .bind_pipeline_graphics(main_pipeline.get_pipeline())
                .expect("tried to bind pipeline");
            command_builder.set_cull_mode(CullMode::Back)
                .expect("tried to set cull mode back to Back");
        }

        upload_main_material_uniform(&mut command_builder, main_pipeline.clone(), mvp.clone());
        upload_fog_uniform(&mut command_builder, main_pipeline.clone(), fog_data.clone());
        upload_lightmap_descriptor_set(desired_lightmap, &currently_loaded_bsp, &mut command_builder, main_pipeline.clone());

        let index_buffer = geometry.vulkan.index_buffer.clone();
        let index_count = index_buffer.len() as usize;
        command_builder.bind_index_buffer(index_buffer).expect("can't bind indices");

        if main_pipeline.has_lightmaps() {
            command_builder.bind_vertex_buffers(0, (
                geometry.vulkan.vertex_buffer.clone(),
                geometry.vulkan.texture_coords_buffer.clone(),
                if geometry.vulkan.lightmap_texture_coords_buffer.is_none() {
                    geometry.vulkan.texture_coords_buffer.clone()
                } else {
                    geometry.vulkan.lightmap_texture_coords_buffer.clone().unwrap()
                }
            )).unwrap();
        }
        else {
            command_builder.bind_vertex_buffers(0, (
                geometry.vulkan.vertex_buffer.clone(),
                geometry.vulkan.texture_coords_buffer.clone()
            )).unwrap();
        }

        shader
            .generate_commands(renderer, index_count as u32, repeat_shader, &mut command_builder)
            .expect("can't generate stage commands");
    }

    fn draw_split_screen_bars(renderer: &Renderer, command_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>, width: f32, height: f32) {
        if renderer.player_viewports.len() <= 1 {
            return;
        }

        let color = [0.0, 0.0, 0.0, 1.0];
        let viewport = Viewport {
            offset: [0.0, 0.0],
            extent: [width, height],
            depth_range: 0.0..=1.0,
        };
        command_builder.set_viewport(0, [viewport].into_iter().collect()).unwrap();

        let base_thickness = 2.0;
        let scale = (width / 640.0).min(height / 480.0).max(1.0);
        let line_thickness_horizontal = base_thickness / height * scale;
        let line_thickness_vertical = base_thickness / width * scale;

        draw_box(renderer, 0.0, 0.5 - line_thickness_horizontal / 2.0, 1.0, line_thickness_horizontal, color, command_builder)
            .expect("can't draw split screen vertical bar");

        if renderer.player_viewports.len() > 2 {
            let y;
            let line_height;

            if renderer.player_viewports.len() == 3 {
                y = 0.5;
                line_height = 0.5;
            } else {
                y = 0.0;
                line_height = 1.0;
            }

            draw_box(renderer, 0.5 - line_thickness_vertical / 2.0, y, line_thickness_vertical, line_height, color, command_builder)
                .expect("can't draw split screen horizontal bar");
        }
    }

    fn execute_command_list(&mut self, command_buffer: Arc<impl PrimaryCommandBufferAbstract + 'static>) {
        let execution = command_buffer.execute(self.queue.clone()).unwrap();

        let future = self.future
            .take()
            .expect("no future?")
            .join(execution)
            .then_signal_fence_and_flush()
            .expect("failed to signal/flush")
            .boxed_send_sync();

        self.future = Some(future)
    }

    fn generate_secondary_buffer_builder(&self) -> MResult<AutoCommandBufferBuilder<SecondaryAutoCommandBuffer>> {
        let result = AutoCommandBufferBuilder::secondary(
            &self.command_buffer_allocator,
            self.queue.queue_family_index(),
            CommandBufferUsage::MultipleSubmit,
            CommandBufferInheritanceInfo {
                render_pass: Some(CommandBufferInheritanceRenderPassType::BeginRendering(CommandBufferInheritanceRenderingInfo {
                    color_attachment_formats: vec![Some(OFFLINE_PIPELINE_COLOR_FORMAT)],
                    depth_attachment_format: Some(Format::D32_SFLOAT),
                    ..CommandBufferInheritanceRenderingInfo::default()
                })),
                ..CommandBufferInheritanceInfo::default()
            }
        )?;
        Ok(result)
    }
}

extern "C" {
    fn exit(code: i32) -> !;
}

fn default_allocation_create_info() -> AllocationCreateInfo {
    AllocationCreateInfo {
        memory_type_filter: MemoryTypeFilter::PREFER_DEVICE | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
        ..Default::default()
    }
}

impl<T: Display> From<Validated<T>> for Error {
    fn from(value: Validated<T>) -> Self {
        match value {
            Validated::ValidationError(v) => v.into(),
            Validated::Error(e) => Self::from_vulkan_error(format!("Vulkan error! {e}"))
        }
    }
}

impl From<Box<ValidationError>> for Error {
    fn from(value: Box<ValidationError>) -> Self {
        // FIXME: figure out a more graceful way to do this
        eprintln!("Validation error! {value:?}\n\n-----------\n\nBACKTRACE:\n\n{}\n\n-----------\n\n", std::backtrace::Backtrace::force_capture());
        std::process::abort();
    }
}

impl From<vulkano::LoadingError> for Error {
    fn from(value: vulkano::LoadingError) -> Self {
        Self::from_vulkan_error(format!("Loading error! {value:?}"))
    }
}

impl Error {
    fn from_vulkan_error(error: String) -> Self {
        Self::GraphicsAPIError { backend: "Vulkan", error }
    }
    fn from_vulkan_impl_error(error: String) -> Self {
        Self::GraphicsAPIError { backend: "Vulkan-IMPL", error }
    }
}

fn upload_lightmap_descriptor_set(
    lightmap_index: Option<usize>,
    bsp: &BSP,
    builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    pipeline: Arc<dyn VulkanPipelineData>
) {
    if !pipeline.has_lightmaps() {
        return;
    }

    let set = lightmap_index
        .and_then(|i| bsp.vulkan.lightmap_images.get(&i))
        .map(|b| b.clone())
        .unwrap_or_else(|| bsp.vulkan.null_lightmaps.clone());
    builder.bind_descriptor_sets(
        PipelineBindPoint::Graphics,
        pipeline.get_pipeline().layout().clone(),
        1,
        set
    ).unwrap();
}

struct FogData {
    color: [f32; 4],
    distance_from: f32,
    distance_to: f32,
    min_opacity: f32,
    max_opacity: f32
}

impl Default for FogData {
    fn default() -> Self {
        Self {
            color: [0.0f32; 4],
            distance_from: 0.0,
            distance_to: 1.0,
            min_opacity: 0.0,
            max_opacity: 0.0
        }
    }
}

fn upload_main_material_uniform(
    builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    pipeline: Arc<dyn VulkanPipelineData>,
    set: Arc<PersistentDescriptorSet>
) {
    builder.bind_descriptor_sets(
        PipelineBindPoint::Graphics,
        pipeline.get_pipeline().layout().clone(),
        0,
        set
    ).unwrap();
}

fn upload_fog_uniform(
    builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    pipeline: Arc<dyn VulkanPipelineData>,
    set: Arc<PersistentDescriptorSet>
) {
    if !pipeline.has_fog() {
        return;
    }

    builder.bind_descriptor_sets(
        PipelineBindPoint::Graphics,
        pipeline.get_pipeline().layout().clone(),
        2,
        set
    ).unwrap();
}

fn make_fog_uniform(
    renderer: &Renderer,
    fog: &FogData
) -> Arc<PersistentDescriptorSet> {
    let pipeline = renderer
        .renderer
        .pipelines[&VulkanPipelineType::ShaderEnvironment]
        .get_pipeline();

    let fog_data = VulkanFogData {
        sky_fog_to: fog.distance_to,
        sky_fog_from: fog.distance_from,
        sky_fog_min_opacity: fog.min_opacity,
        sky_fog_max_opacity: fog.max_opacity,
        sky_fog_color: fog.color
    };

    let fog_uniform_buffer = Buffer::from_data(
        renderer.renderer.memory_allocator.clone(),
        BufferCreateInfo { usage: BufferUsage::UNIFORM_BUFFER, ..Default::default() },
        default_allocation_create_info(),
        fog_data
    ).unwrap();

    PersistentDescriptorSet::new(
        renderer.renderer.descriptor_set_allocator.as_ref(),
        pipeline.layout().set_layouts()[2].clone(),
        [
            WriteDescriptorSet::buffer(0, fog_uniform_buffer),
        ],
        []
    ).unwrap()
}

fn make_model_view_uniform(
    renderer: &Renderer,
    camera: Vec3,
    offset: Vec3,
    rotation: Mat3,
    view: Mat4,
    proj: Mat4,
) -> Arc<PersistentDescriptorSet> {
    let pipeline = renderer.renderer.pipelines[&VulkanPipelineType::ShaderEnvironment].get_pipeline();
    let model = Mat4::IDENTITY;

    let model_data = VulkanModelData {
        camera: Padded::from(camera.to_array()),
        world: model.to_cols_array_2d(),
        view: view.to_cols_array_2d(),
        proj: proj.to_cols_array_2d(),
        offset: Padded::from(offset.to_array()),
        rotation: [
            Padded::from(rotation.x_axis.to_array()),
            Padded::from(rotation.y_axis.to_array()),
            Padded::from(rotation.z_axis.to_array())
        ],
    };

    let model_uniform_buffer = Buffer::from_data(
        renderer.renderer.memory_allocator.clone(),
        BufferCreateInfo { usage: BufferUsage::UNIFORM_BUFFER, ..Default::default() },
        default_allocation_create_info(),
        model_data
    ).unwrap();

    PersistentDescriptorSet::new(
        renderer.renderer.descriptor_set_allocator.as_ref(),
        pipeline.layout().set_layouts()[0].clone(),
        [
            WriteDescriptorSet::buffer(0, model_uniform_buffer),
        ],
        []
    ).unwrap()
}

fn draw_box(renderer: &Renderer, x: f32, y: f32, width: f32, height: f32, color: [f32; 4], command_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>) -> MResult<()> {
    let indices = Buffer::from_iter(
        renderer.renderer.memory_allocator.clone(),
        BufferCreateInfo {
            usage: BufferUsage::INDEX_BUFFER,
            ..Default::default()
        },
        default_allocation_create_info(),
        [0u16,1,2,0,2,3]
    )?;
    let vertices = Buffer::from_iter(
        renderer.renderer.memory_allocator.clone(),
        BufferCreateInfo {
            usage: BufferUsage::VERTEX_BUFFER,
            ..Default::default()
        },
        default_allocation_create_info(),
        [
            VulkanModelVertex {
                position: [x, y, 0.5],
                normal: [1.0, 0.0, 0.0],
                binormal: [1.0, 0.0, 0.0],
                tangent: [1.0, 0.0, 0.0]
            },
            VulkanModelVertex {
                position: [x, y + height, 0.5],
                normal: [1.0, 0.0, 0.0],
                binormal: [1.0, 0.0, 0.0],
                tangent: [1.0, 0.0, 0.0]
            },
            VulkanModelVertex {
                position: [x + width, y + height, 0.5],
                normal: [1.0, 0.0, 0.0],
                binormal: [1.0, 0.0, 0.0],
                tangent: [1.0, 0.0, 0.0]
            },
            VulkanModelVertex {
                position: [x + width, y, 0.5],
                normal: [1.0, 0.0, 0.0],
                binormal: [1.0, 0.0, 0.0],
                tangent: [1.0, 0.0, 0.0]
            }
        ]
    )?;

    let pipeline = renderer
        .renderer
        .pipelines[&VulkanPipelineType::ColorBox]
        .get_pipeline();

    let uniform_buffer = Buffer::from_data(
        renderer.renderer.memory_allocator.clone(),
        BufferCreateInfo { usage: BufferUsage::UNIFORM_BUFFER, ..Default::default() },
        default_allocation_create_info(),
        color
    ).unwrap();

    let set = PersistentDescriptorSet::new(
        renderer.renderer.descriptor_set_allocator.as_ref(),
        pipeline.layout().set_layouts()[1].clone(),
        [
            WriteDescriptorSet::buffer(0, uniform_buffer),
        ],
        []
    ).unwrap();

    command_builder.bind_descriptor_sets(
        PipelineBindPoint::Graphics,
        pipeline.layout().clone(),
        1,
        set
    ).unwrap();

    command_builder.set_cull_mode(CullMode::None).unwrap();
    command_builder.bind_index_buffer(indices).unwrap();
    command_builder.bind_vertex_buffers(0, vertices).unwrap();
    command_builder.bind_pipeline_graphics(pipeline).unwrap();
    command_builder.draw_indexed(6, 1, 0, 0, 0).unwrap();

    Ok(())
}
