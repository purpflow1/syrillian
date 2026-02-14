use crate::cache::AssetCache;
use crate::passes::post_process::{
    BloomInputSource, BloomRenderPass, BloomSettings, FxaaInputSource, FxaaRenderPass,
    PostProcessData, ScreenSpaceReflectionRenderPass,
};
use crate::passes::ui_pass::UiRenderPass;
use crate::rendering::offscreen_surface::OffscreenSurface;
use crate::rendering::render_data::RenderUniformData;
use crate::rendering::renderer::RenderedFrame;
use crate::rendering::state::State;
use crate::rendering::viewport::{RenderViewport, ViewportId};
use crate::strobe::StrobeRenderer;
use syrillian_utils::{AntiAliasingMode, EngineArgs};
use tracing::info;
use wgpu::{
    Color, CommandEncoder, Device, Extent3d, LoadOp, Operations, Queue, RenderPassColorAttachment,
    RenderPassDescriptor, StoreOp, SurfaceConfiguration, Texture, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages, TextureViewDescriptor,
};
use winit::dpi::PhysicalSize;

#[derive(Debug, Copy, Clone)]
enum SceneSource {
    Base,
    Ssr,
    Bloom,
}

#[derive(Debug, Copy, Clone)]
enum FinalSource {
    Base = 0,
    Ssr = 1,
    Bloom = 2,
    Fxaa = 3,
}

pub struct FinalFrameContext<'a> {
    pub render_data: &'a RenderUniformData,
    pub target: ViewportId,
    pub size: PhysicalSize<u32>,
    pub format: TextureFormat,
    pub frame_count: usize,
}

pub struct RenderPipeline {
    pub depth_texture: Texture,
    pub offscreen_surface: OffscreenSurface,
    pub ssr_pass: ScreenSpaceReflectionRenderPass,
    pub fxaa_pass: FxaaRenderPass,
    pub bloom_pass: BloomRenderPass,
    pub final_surfaces: [OffscreenSurface; 2],
    pub g_normal: Texture,
    pub g_material: Texture,
    pub g_velocity: Texture,

    final_uniforms: [PostProcessData; 4],
    bloom_settings: BloomSettings,
    bloom_settings_dirty: bool,
}

impl RenderPipeline {
    pub fn new(device: &Device, cache: &AssetCache, config: &SurfaceConfiguration) -> Self {
        let pp_bgl = cache.bgl_post_process();
        let pp_compute_bgl = cache.bgl_post_process_compute();
        let bloom_compute_bgl = cache.bgl_bloom_compute();

        info!("Render Pipeline AA mode: {:?}", EngineArgs::aa_mode());

        let normal_texture = Self::create_g_buffer("GBuffer (Normals)", device, config);
        let material_texture = Self::create_material_texture(device, config);
        let velocity_texture = Self::create_velocity_texture(device, config);
        let depth_texture = Self::create_depth_texture(device, config);

        let depth_view = depth_texture.create_view(&TextureViewDescriptor::default());
        let normal_view = normal_texture.create_view(&TextureViewDescriptor::default());
        let material_view = material_texture.create_view(&TextureViewDescriptor::default());

        let offscreen_surface = OffscreenSurface::new_with(
            device,
            config,
            TextureFormat::Rgba8Unorm,
            TextureUsages::empty(),
        );
        let final_surfaces = [
            OffscreenSurface::new(device, config),
            OffscreenSurface::new(device, config),
        ];

        let ssr_pass = ScreenSpaceReflectionRenderPass::new(
            device,
            config,
            pp_compute_bgl,
            &offscreen_surface,
            depth_view.clone(),
            normal_view.clone(),
            material_view.clone(),
        );

        let bloom_settings = BloomSettings::from_engine_args();

        let bloom_pass = BloomRenderPass::new(
            device,
            config,
            bloom_compute_bgl,
            offscreen_surface.view().clone(),
            ssr_pass.output.view().clone(),
            &bloom_settings,
        );

        let fxaa_pass = FxaaRenderPass::new(
            device,
            config,
            &pp_bgl,
            offscreen_surface.view().clone(),
            ssr_pass.output.view().clone(),
            bloom_pass.output.view().clone(),
            depth_view.clone(),
            normal_view.clone(),
            material_view.clone(),
        );

        let final_uniforms = [
            PostProcessData::new(
                device,
                pp_bgl.clone(),
                offscreen_surface.view().clone(),
                depth_view.clone(),
                normal_view.clone(),
                material_view.clone(),
            ),
            PostProcessData::new(
                device,
                pp_bgl.clone(),
                ssr_pass.output.view().clone(),
                depth_view.clone(),
                normal_view.clone(),
                material_view.clone(),
            ),
            PostProcessData::new(
                device,
                pp_bgl.clone(),
                bloom_pass.output.view().clone(),
                depth_view.clone(),
                normal_view.clone(),
                material_view.clone(),
            ),
            PostProcessData::new(
                device,
                pp_bgl.clone(),
                fxaa_pass.output.view().clone(),
                depth_view.clone(),
                normal_view.clone(),
                material_view.clone(),
            ),
        ];

        Self {
            depth_texture,
            offscreen_surface,
            ssr_pass,
            fxaa_pass,
            bloom_pass,
            final_surfaces,
            g_normal: normal_texture,
            g_material: material_texture,
            g_velocity: velocity_texture,
            final_uniforms,
            bloom_settings,
            bloom_settings_dirty: false,
        }
    }

    pub fn recreate(&mut self, device: &Device, cache: &AssetCache, config: &SurfaceConfiguration) {
        let bloom_settings = self.bloom_settings;
        *self = Self::new(device, cache, config);
        self.set_bloom_settings(bloom_settings);
    }

    #[inline]
    fn current_surface_index(frame_count: usize) -> usize {
        frame_count % 2
    }

    fn create_depth_texture(device: &Device, config: &SurfaceConfiguration) -> Texture {
        device.create_texture(&TextureDescriptor {
            label: Some("Depth Texture"),
            size: Extent3d {
                width: config.width.max(1),
                height: config.height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Depth32Float,
            usage: TextureUsages::COPY_SRC
                | TextureUsages::RENDER_ATTACHMENT
                | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
    }

    fn create_g_buffer(
        which: &'static str,
        device: &Device,
        config: &SurfaceConfiguration,
    ) -> Texture {
        device.create_texture(&TextureDescriptor {
            label: Some(which),
            size: Extent3d {
                width: config.width.max(1),
                height: config.height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rg16Float,
            usage: TextureUsages::COPY_SRC
                | TextureUsages::RENDER_ATTACHMENT
                | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
    }

    fn create_material_texture(device: &Device, config: &SurfaceConfiguration) -> Texture {
        device.create_texture(&TextureDescriptor {
            label: Some("Material Property Texture"),
            size: Extent3d {
                width: config.width.max(1),
                height: config.height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8Unorm,
            usage: TextureUsages::COPY_SRC
                | TextureUsages::RENDER_ATTACHMENT
                | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
    }

    fn create_velocity_texture(device: &Device, config: &SurfaceConfiguration) -> Texture {
        device.create_texture(&TextureDescriptor {
            label: Some("GBuffer (Velocity)"),
            size: Extent3d {
                width: config.width.max(1),
                height: config.height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rg16Float,
            usage: TextureUsages::COPY_SRC
                | TextureUsages::RENDER_ATTACHMENT
                | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
    }

    pub fn prepare_frame(
        &mut self,
        render_data: &mut RenderUniformData,
        queue: &Queue,
        _frame_count: usize,
    ) {
        let base_view_proj =
            render_data.camera_data.projection_mat * render_data.camera_data.view_mat;
        let view_proj = base_view_proj;

        render_data.camera_data.proj_view_mat = view_proj;
        render_data.camera_data.inv_proj_view_mat = view_proj.inverse();

        render_data.upload_camera_data(queue);

        if self.bloom_settings_dirty {
            self.bloom_pass.update_settings(queue, &self.bloom_settings);
            self.bloom_settings_dirty = false;
        }
    }

    pub fn set_bloom_settings(&mut self, settings: BloomSettings) {
        self.bloom_settings = settings.sanitized();
        self.bloom_settings_dirty = true;
    }

    pub fn bloom_settings(&self) -> &BloomSettings {
        &self.bloom_settings
    }

    fn final_uniform(&self, source: FinalSource) -> &PostProcessData {
        &self.final_uniforms[source as usize]
    }

    pub fn render_post_process(
        &mut self,
        camera_render_data: &RenderUniformData,
        encoder: &mut CommandEncoder,
        cache: &AssetCache,
    ) {
        let mut source = SceneSource::Base;
        if !EngineArgs::get().no_ssr {
            self.ssr_pass.render(camera_render_data, encoder, cache);
            source = SceneSource::Ssr;
        }

        if self.bloom_settings.enabled {
            let source_bloom = match source {
                SceneSource::Base => BloomInputSource::Base,
                SceneSource::Ssr => BloomInputSource::Ssr,
                SceneSource::Bloom => BloomInputSource::Ssr,
            };
            self.bloom_pass.render(
                camera_render_data,
                encoder,
                cache,
                source_bloom,
                &self.bloom_settings,
            );
            source = SceneSource::Bloom;
        }

        if let AntiAliasingMode::Fxaa = EngineArgs::aa_mode() {
            let source_fxaa = match source {
                SceneSource::Base => FxaaInputSource::Base,
                SceneSource::Ssr => FxaaInputSource::Ssr,
                SceneSource::Bloom => FxaaInputSource::Bloom,
            };
            self.fxaa_pass
                .render(camera_render_data, encoder, cache, source_fxaa);
        }
    }

    pub fn render_ui_onto_final_frame(
        &self,
        encoder: &mut CommandEncoder,
        strobe: &mut StrobeRenderer,
        viewport: &RenderViewport,
        cache: &AssetCache,
        state: &State,
    ) {
        let final_color = &self.final_surfaces[Self::current_surface_index(viewport.frame_count())];

        UiRenderPass::render(encoder, strobe, final_color.view(), viewport, cache, state);
    }

    pub fn finalize_frame(
        &mut self,
        encoder: &mut CommandEncoder,
        cache: &AssetCache,
        context: FinalFrameContext<'_>,
    ) -> RenderedFrame {
        let current_idx = Self::current_surface_index(context.frame_count);
        let final_color = &self.final_surfaces[current_idx];
        let final_frame_texture = final_color.texture().clone();

        let mut source = if EngineArgs::get().no_ssr {
            FinalSource::Base
        } else {
            FinalSource::Ssr
        };

        if self.bloom_settings.enabled {
            source = FinalSource::Bloom;
        }
        if let AntiAliasingMode::Fxaa = EngineArgs::aa_mode() {
            source = FinalSource::Fxaa;
        }

        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Final Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: final_color.view(),
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..RenderPassDescriptor::default()
            });

            let post_shader = cache.shader_post_process();
            let groups = post_shader.bind_groups();
            pass.set_pipeline(post_shader.solid_pipeline());
            pass.set_bind_group(groups.render, context.render_data.uniform.bind_group(), &[]);
            if let Some(idx) = groups.post_process {
                pass.set_bind_group(idx, self.final_uniform(source).uniform.bind_group(), &[]);
            }
            pass.draw(0..6, 0..1);
        }

        RenderedFrame {
            target: context.target,
            frame: final_frame_texture,
            size: context.size,
            format: context.format,
        }
    }
}
