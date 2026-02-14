use crate::cache::AssetCache;
use crate::rendering::offscreen_surface::OffscreenSurface;
use crate::rendering::render_data::RenderUniformData;
use crate::rendering::uniform::ShaderUniform;
use syrillian_asset::HComputeShader;
use syrillian_macros::UniformIndex;
use wgpu::{
    BindGroupLayout, CommandEncoder, ComputePassDescriptor, Device, SurfaceConfiguration,
    TextureFormat, TextureUsages, TextureView,
};

#[repr(u8)]
#[derive(Debug, Copy, Clone, UniformIndex)]
enum SsaoComputeUniformIndex {
    Depth = 0,
    GNormal = 1,
    GMaterial = 2,
    AoInput = 3,
    Output = 4,
}

#[repr(u8)]
#[derive(Debug, Copy, Clone, UniformIndex)]
enum SsaoApplyUniformIndex {
    Color = 0,
    Ao = 1,
    Output = 2,
}

#[derive(Debug, Copy, Clone)]
pub enum SsaoInputSource {
    Base = 0,
    Ssr = 1,
}

pub struct ScreenSpaceAmbientOcclusionRenderPass {
    pub output: OffscreenSurface,
    _ao_raw: OffscreenSurface,
    _ao_blur: OffscreenSurface,
    generate_uniform: ShaderUniform<SsaoComputeUniformIndex>,
    blur_horizontal_uniform: ShaderUniform<SsaoComputeUniformIndex>,
    blur_vertical_uniform: ShaderUniform<SsaoComputeUniformIndex>,
    apply_uniforms: [ShaderUniform<SsaoApplyUniformIndex>; 2],
}

impl ScreenSpaceAmbientOcclusionRenderPass {
    pub fn new(
        device: &Device,
        config: &SurfaceConfiguration,
        ssao_compute_bgl: BindGroupLayout,
        ssao_apply_compute_bgl: BindGroupLayout,
        color_base_view: TextureView,
        color_ssr_view: TextureView,
        depth_view: TextureView,
        g_normal_view: TextureView,
        g_material_view: TextureView,
    ) -> Self {
        let output = OffscreenSurface::new_with(
            device,
            config,
            TextureFormat::Rgba8Unorm,
            TextureUsages::STORAGE_BINDING,
        );

        let ao_raw = OffscreenSurface::new_with(
            device,
            config,
            TextureFormat::R32Float,
            TextureUsages::STORAGE_BINDING,
        );
        let ao_blur = OffscreenSurface::new_with(
            device,
            config,
            TextureFormat::R32Float,
            TextureUsages::STORAGE_BINDING,
        );

        let generate_uniform =
            ShaderUniform::<SsaoComputeUniformIndex>::builder(ssao_compute_bgl.clone())
                .with_texture(depth_view.clone())
                .with_texture(g_normal_view.clone())
                .with_texture(g_material_view.clone())
                .with_texture(ao_blur.view().clone())
                .with_texture(ao_raw.view().clone())
                .build(device);

        let blur_horizontal_uniform =
            ShaderUniform::<SsaoComputeUniformIndex>::builder(ssao_compute_bgl.clone())
                .with_texture(depth_view.clone())
                .with_texture(g_normal_view.clone())
                .with_texture(g_material_view.clone())
                .with_texture(ao_raw.view().clone())
                .with_texture(ao_blur.view().clone())
                .build(device);

        let blur_vertical_uniform =
            ShaderUniform::<SsaoComputeUniformIndex>::builder(ssao_compute_bgl)
                .with_texture(depth_view)
                .with_texture(g_normal_view)
                .with_texture(g_material_view)
                .with_texture(ao_blur.view().clone())
                .with_texture(ao_raw.view().clone())
                .build(device);

        let apply_uniforms = [
            ShaderUniform::<SsaoApplyUniformIndex>::builder(ssao_apply_compute_bgl.clone())
                .with_texture(color_base_view)
                .with_texture(ao_raw.view().clone())
                .with_texture(output.view().clone())
                .build(device),
            ShaderUniform::<SsaoApplyUniformIndex>::builder(ssao_apply_compute_bgl)
                .with_texture(color_ssr_view)
                .with_texture(ao_raw.view().clone())
                .with_texture(output.view().clone())
                .build(device),
        ];

        Self {
            output,
            _ao_raw: ao_raw,
            _ao_blur: ao_blur,
            generate_uniform,
            blur_horizontal_uniform,
            blur_vertical_uniform,
            apply_uniforms,
        }
    }

    fn apply_uniform(&self, source: SsaoInputSource) -> &ShaderUniform<SsaoApplyUniformIndex> {
        &self.apply_uniforms[source as usize]
    }

    pub fn render(
        &self,
        camera_render_data: &RenderUniformData,
        encoder: &mut CommandEncoder,
        cache: &AssetCache,
        source: SsaoInputSource,
    ) {
        let width = camera_render_data.system_data.screen_size.x.max(1);
        let height = camera_render_data.system_data.screen_size.y.max(1);
        let dispatch_x = width.div_ceil(8);
        let dispatch_y = height.div_ceil(8);

        let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("SSAO Post Process Compute Pass"),
            ..ComputePassDescriptor::default()
        });

        let generate = cache.compute_shader(HComputeShader::POST_PROCESS_SSAO);
        pass.set_pipeline(generate.pipeline());
        pass.set_bind_group(0, camera_render_data.uniform.bind_group(), &[]);
        pass.set_bind_group(1, self.generate_uniform.bind_group(), &[]);
        pass.dispatch_workgroups(dispatch_x, dispatch_y, 1);

        let blur_x = cache.compute_shader(HComputeShader::POST_PROCESS_SSAO_BLUR_X);
        pass.set_pipeline(blur_x.pipeline());
        pass.set_bind_group(0, self.blur_horizontal_uniform.bind_group(), &[]);
        pass.dispatch_workgroups(dispatch_x, dispatch_y, 1);

        let blur_y = cache.compute_shader(HComputeShader::POST_PROCESS_SSAO_BLUR_Y);
        pass.set_pipeline(blur_y.pipeline());
        pass.set_bind_group(0, self.blur_vertical_uniform.bind_group(), &[]);
        pass.dispatch_workgroups(dispatch_x, dispatch_y, 1);

        let apply = cache.compute_shader(HComputeShader::POST_PROCESS_SSAO_APPLY);
        pass.set_pipeline(apply.pipeline());
        pass.set_bind_group(0, self.apply_uniform(source).bind_group(), &[]);
        pass.dispatch_workgroups(dispatch_x, dispatch_y, 1);
    }
}
