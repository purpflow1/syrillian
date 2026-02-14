// TODO: Refactor Shader Input Output Chain

use crate::cache::AssetCache;
use crate::passes::post_process::PostProcessData;
use crate::rendering::offscreen_surface::OffscreenSurface;
use crate::rendering::render_data::RenderUniformData;
use wgpu::{
    Color, CommandEncoder, Device, LoadOp, Operations, RenderPassColorAttachment,
    RenderPassDescriptor, StoreOp, SurfaceConfiguration, TextureFormat, TextureView,
};

#[derive(Debug, Copy, Clone)]
pub enum FxaaInputSource {
    Base = 0,
    Ssr = 1,
    Ssao = 2,
    Bloom = 3,
}

pub struct FxaaRenderPass {
    pub output: OffscreenSurface,
    uniforms: [PostProcessData; 4],
}

impl FxaaRenderPass {
    pub fn new(
        device: &Device,
        config: &SurfaceConfiguration,
        post_process_bgl: &wgpu::BindGroupLayout,
        color_base_view: TextureView,
        color_ssr_view: TextureView,
        color_ssao_view: TextureView,
        color_bloom_view: TextureView,
        depth_view: TextureView,
        g_normal_view: TextureView,
        g_material_view: TextureView,
    ) -> Self {
        let output = OffscreenSurface::new_with(
            device,
            config,
            TextureFormat::Rgba8Unorm,
            wgpu::TextureUsages::empty(),
        );

        let uniforms = [
            PostProcessData::new(
                device,
                post_process_bgl.clone(),
                color_base_view,
                depth_view.clone(),
                g_normal_view.clone(),
                g_material_view.clone(),
            ),
            PostProcessData::new(
                device,
                post_process_bgl.clone(),
                color_ssr_view,
                depth_view.clone(),
                g_normal_view.clone(),
                g_material_view.clone(),
            ),
            PostProcessData::new(
                device,
                post_process_bgl.clone(),
                color_ssao_view,
                depth_view.clone(),
                g_normal_view.clone(),
                g_material_view.clone(),
            ),
            PostProcessData::new(
                device,
                post_process_bgl.clone(),
                color_bloom_view,
                depth_view,
                g_normal_view,
                g_material_view,
            ),
        ];

        Self { output, uniforms }
    }

    fn uniform(&self, source: FxaaInputSource) -> &PostProcessData {
        &self.uniforms[source as usize]
    }

    pub fn render(
        &self,
        render_data: &RenderUniformData,
        encoder: &mut CommandEncoder,
        cache: &AssetCache,
        source: FxaaInputSource,
    ) {
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("FXAA Render Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: self.output.view(),
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

        let shader = cache.shader_post_process_fxaa();
        let groups = shader.bind_groups();
        pass.set_pipeline(shader.solid_pipeline());
        pass.set_bind_group(groups.render, render_data.uniform.bind_group(), &[]);
        if let Some(idx) = groups.post_process {
            pass.set_bind_group(idx, self.uniform(source).uniform.bind_group(), &[]);
        }
        pass.draw(0..6, 0..1);
    }
}
