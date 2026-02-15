use crate::passes::post_process::{
    PostProcessPass, PostProcessPassContext, PostProcessRoute, PostProcessSharedViews,
};
use crate::rendering::offscreen_surface::OffscreenSurface;
use crate::rendering::uniform::ShaderUniform;
use syrillian_asset::HComputeShader;
use syrillian_macros::UniformIndex;
use wgpu::{
    BindGroupLayout, ComputePassDescriptor, Device, TextureFormat, TextureUsages, TextureView,
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

pub struct ScreenSpaceAmbientOcclusionRenderPass {
    generate_uniform: ShaderUniform<SsaoComputeUniformIndex>,
    blur_horizontal_uniform: ShaderUniform<SsaoComputeUniformIndex>,
    blur_vertical_uniform: ShaderUniform<SsaoComputeUniformIndex>,
    apply_uniform: ShaderUniform<SsaoApplyUniformIndex>,
    _ao_raw: OffscreenSurface,
    _ao_blur: OffscreenSurface,
}

impl ScreenSpaceAmbientOcclusionRenderPass {
    pub fn new(
        device: &Device,
        width: u32,
        height: u32,
        ssao_compute_bgl: BindGroupLayout,
        ssao_apply_compute_bgl: BindGroupLayout,
        shared: &PostProcessSharedViews,
        apply_route: &PostProcessRoute,
    ) -> Self {
        let ao_raw = OffscreenSurface::new_sized_with(
            device,
            width,
            height,
            TextureFormat::R32Float,
            TextureUsages::STORAGE_BINDING,
        );
        let ao_blur = OffscreenSurface::new_sized_with(
            device,
            width,
            height,
            TextureFormat::R32Float,
            TextureUsages::STORAGE_BINDING,
        );

        let generate_uniform =
            ShaderUniform::<SsaoComputeUniformIndex>::builder(ssao_compute_bgl.clone())
                .with_texture(shared.depth.clone())
                .with_texture(shared.g_normal.clone())
                .with_texture(shared.g_material.clone())
                .with_texture(ao_blur.view().clone())
                .with_texture(ao_raw.view().clone())
                .build(device);

        let blur_horizontal_uniform =
            ShaderUniform::<SsaoComputeUniformIndex>::builder(ssao_compute_bgl.clone())
                .with_texture(shared.depth.clone())
                .with_texture(shared.g_normal.clone())
                .with_texture(shared.g_material.clone())
                .with_texture(ao_raw.view().clone())
                .with_texture(ao_blur.view().clone())
                .build(device);

        let blur_vertical_uniform =
            ShaderUniform::<SsaoComputeUniformIndex>::builder(ssao_compute_bgl)
                .with_texture(shared.depth.clone())
                .with_texture(shared.g_normal.clone())
                .with_texture(shared.g_material.clone())
                .with_texture(ao_blur.view().clone())
                .with_texture(ao_raw.view().clone())
                .build(device);

        let apply_uniform = ShaderUniform::<SsaoApplyUniformIndex>::builder(ssao_apply_compute_bgl)
            .with_texture(apply_route.input_color.clone())
            .with_texture(ao_raw.view().clone())
            .with_texture(apply_route.output_color.clone())
            .build(device);

        Self {
            generate_uniform,
            blur_horizontal_uniform,
            blur_vertical_uniform,
            apply_uniform,
            _ao_raw: ao_raw,
            _ao_blur: ao_blur,
        }
    }
}

impl PostProcessPass for ScreenSpaceAmbientOcclusionRenderPass {
    fn name(&self) -> &'static str {
        "SSAO"
    }

    fn execute(&mut self, ctx: &mut PostProcessPassContext<'_>, _output_color: &TextureView) {
        let width = ctx.camera_render_data.system_data.screen_size.x.max(1);
        let height = ctx.camera_render_data.system_data.screen_size.y.max(1);
        let dispatch_x = width.div_ceil(8);
        let dispatch_y = height.div_ceil(8);

        let mut pass = ctx.encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("SSAO Post Process Compute Pass"),
            ..ComputePassDescriptor::default()
        });

        let generate = ctx.cache.compute_shader(HComputeShader::POST_PROCESS_SSAO);
        pass.set_pipeline(generate.pipeline());
        pass.set_bind_group(0, ctx.camera_render_data.uniform.bind_group(), &[]);
        pass.set_bind_group(1, self.generate_uniform.bind_group(), &[]);
        pass.dispatch_workgroups(dispatch_x, dispatch_y, 1);

        let blur_x = ctx
            .cache
            .compute_shader(HComputeShader::POST_PROCESS_SSAO_BLUR_X);
        pass.set_pipeline(blur_x.pipeline());
        pass.set_bind_group(0, self.blur_horizontal_uniform.bind_group(), &[]);
        pass.dispatch_workgroups(dispatch_x, dispatch_y, 1);

        let blur_y = ctx
            .cache
            .compute_shader(HComputeShader::POST_PROCESS_SSAO_BLUR_Y);
        pass.set_pipeline(blur_y.pipeline());
        pass.set_bind_group(0, self.blur_vertical_uniform.bind_group(), &[]);
        pass.dispatch_workgroups(dispatch_x, dispatch_y, 1);

        let apply = ctx
            .cache
            .compute_shader(HComputeShader::POST_PROCESS_SSAO_APPLY);
        pass.set_pipeline(apply.pipeline());
        pass.set_bind_group(0, self.apply_uniform.bind_group(), &[]);
        pass.dispatch_workgroups(dispatch_x, dispatch_y, 1);
    }
}
