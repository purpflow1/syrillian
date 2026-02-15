use crate::passes::post_process::{
    PostProcessPass, PostProcessPassContext, PostProcessRoute, PostProcessSharedViews,
};
use crate::rendering::uniform::ShaderUniform;
use syrillian_asset::HComputeShader;
use syrillian_macros::UniformIndex;
use wgpu::{
    AddressMode, BindGroupLayout, ComputePassDescriptor, Device, FilterMode, MipmapFilterMode,
    SamplerDescriptor, TextureView,
};

#[repr(u8)]
#[derive(Debug, Copy, Clone, UniformIndex)]
enum SsrComputeUniformIndex {
    Color = 0,
    Sampler = 1,
    Depth = 2,
    GNormal = 3,
    GMaterial = 4,
    Output = 5,
}

pub struct ScreenSpaceReflectionRenderPass {
    uniform: ShaderUniform<SsrComputeUniformIndex>,
}

impl ScreenSpaceReflectionRenderPass {
    pub fn new(
        device: &Device,
        post_process_compute_bgl: BindGroupLayout,
        shared: &PostProcessSharedViews,
        route: &PostProcessRoute,
    ) -> Self {
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("SSR Compute Sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_filter: MipmapFilterMode::Nearest,
            ..SamplerDescriptor::default()
        });

        let uniform = ShaderUniform::<SsrComputeUniformIndex>::builder(post_process_compute_bgl)
            .with_texture(route.input_color.clone())
            .with_sampler(sampler)
            .with_texture(shared.depth.clone())
            .with_texture(shared.g_normal.clone())
            .with_texture(shared.g_material.clone())
            .with_texture(route.output_color.clone())
            .build(device);

        Self { uniform }
    }
}

impl PostProcessPass for ScreenSpaceReflectionRenderPass {
    fn name(&self) -> &'static str {
        "SSR"
    }

    fn execute(&mut self, ctx: &mut PostProcessPassContext<'_>, _output_color: &TextureView) {
        let width = ctx.camera_render_data.system_data.screen_size.x.max(1);
        let height = ctx.camera_render_data.system_data.screen_size.y.max(1);
        let dispatch_x = width.div_ceil(8);
        let dispatch_y = height.div_ceil(8);

        let mut pass = ctx.encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("SSR Post Process Compute Pass"),
            ..ComputePassDescriptor::default()
        });

        let ssr_shader = ctx.cache.compute_shader(HComputeShader::POST_PROCESS_SSR);
        pass.set_pipeline(ssr_shader.pipeline());
        pass.set_bind_group(0, ctx.camera_render_data.uniform.bind_group(), &[]);
        pass.set_bind_group(1, self.uniform.bind_group(), &[]);
        pass.dispatch_workgroups(dispatch_x, dispatch_y, 1);
    }
}
