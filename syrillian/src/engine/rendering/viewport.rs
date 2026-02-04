use crate::math::UVec2;
use crate::rendering::picking::PickingSurface;
use crate::rendering::post_process::RenderPipeline;
use crate::rendering::render_data::RenderUniformData;
use crate::rendering::{AssetCache, FrameCtx};
use crate::{PhysicalSize, ViewportId};
use std::time::{Duration, Instant};
use tracing::instrument;
use wgpu::{Device, Queue, SurfaceConfiguration, TextureViewDescriptor};

pub struct RenderViewport {
    pub(crate) id: ViewportId,
    pub(super) config: SurfaceConfiguration,
    pub(super) render_pipeline: RenderPipeline,
    pub(super) picking_surface: PickingSurface,
    pub(crate) render_data: RenderUniformData,
    pub(crate) start_time: Instant,
    pub(crate) delta_time: Duration,
    pub(crate) last_frame_time: Instant,
    pub(crate) frame_count: usize,
}

impl RenderViewport {
    pub fn new(
        id: ViewportId,
        mut config: SurfaceConfiguration,
        device: &Device,
        cache: &AssetCache,
    ) -> Self {
        Self::clamp_config(&mut config);

        let render_bgl = cache.bgl_render();

        let picking_surface = PickingSurface::new(device, &config);
        let render_data = RenderUniformData::empty(device, &render_bgl);
        let post_pipeline = RenderPipeline::new(device, cache, &config);

        RenderViewport {
            id,
            config,
            render_pipeline: post_pipeline,
            picking_surface,
            render_data,
            start_time: Instant::now(),
            delta_time: Duration::default(),
            last_frame_time: Instant::now(),
            frame_count: 0,
        }
    }

    fn clamp_config(config: &mut SurfaceConfiguration) {
        config.width = config.width.max(1);
        config.height = config.height.max(1);
    }

    #[instrument(skip_all)]
    #[profiling::function]
    pub fn resize(
        &mut self,
        mut config: SurfaceConfiguration,
        device: &Device,
        cache: &AssetCache,
    ) {
        Self::clamp_config(&mut config);
        self.config = config;

        self.render_pipeline.recreate(device, cache, &self.config);
        self.picking_surface.recreate(device, &self.config);
    }

    #[instrument(skip_all)]
    #[profiling::function]
    pub fn begin_render(&mut self) -> FrameCtx {
        self.frame_count += 1;

        let depth_view = self
            .render_pipeline
            .depth_texture
            .create_view(&TextureViewDescriptor::default());

        FrameCtx { depth_view }
    }

    pub fn update_render_data(&mut self, queue: &Queue) {
        self.update_system_data(queue);
    }

    pub fn update_view_camera_data(&mut self, queue: &Queue) {
        self.render_data.upload_camera_data(queue);
    }

    pub fn update_system_data(&mut self, queue: &Queue) {
        let window_size = UVec2::new(self.config.width.max(1), self.config.height.max(1));

        let system_data = &mut self.render_data.system_data;
        system_data.screen_size = window_size;
        system_data.time = self.start_time.elapsed().as_secs_f32();
        system_data.delta_time = self.delta_time.as_secs_f32();

        self.render_data.upload_system_data(queue);
    }

    /// Updates the delta time based on the elapsed time since the last frame
    pub fn tick_delta_time(&mut self) {
        self.delta_time = self.last_frame_time.elapsed();
        self.last_frame_time = Instant::now();
    }

    pub fn size(&self) -> PhysicalSize<u32> {
        PhysicalSize {
            width: self.config.width,
            height: self.config.height,
        }
    }

    pub fn frame_count(&self) -> usize {
        self.frame_count
    }
}
