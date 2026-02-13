use crate::cache::AssetCache;
use crate::rendering::offscreen_surface::OffscreenSurface;
use crate::rendering::render_data::RenderUniformData;
use crate::rendering::uniform::ShaderUniform;
use syrillian_asset::HComputeShader;
use syrillian_macros::UniformIndex;
use wgpu::{
    AddressMode, BindGroupLayout, CommandEncoder, ComputePassDescriptor, Device, FilterMode,
    MipmapFilterMode, SamplerDescriptor, SurfaceConfiguration, TextureFormat, TextureUsages,
    TextureView,
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

pub struct SsrComputeData {
    uniform: ShaderUniform<SsrComputeUniformIndex>,
}

pub struct ScreenSpaceReflectionRenderPass {
    pub output: OffscreenSurface,
    data: SsrComputeData,
}

impl ScreenSpaceReflectionRenderPass {
    pub fn new(
        device: &Device,
        config: &SurfaceConfiguration,
        post_process_compute_bgl: BindGroupLayout,
        color_input: &OffscreenSurface,
        depth_view: TextureView,
        g_normal_view: TextureView,
        g_material_view: TextureView,
    ) -> Self {
        let ssr_surface_output = OffscreenSurface::new_with(
            device,
            config,
            TextureFormat::Rgba8Unorm,
            TextureUsages::STORAGE_BINDING,
        );

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
            .with_texture(color_input.view().clone())
            .with_sampler(sampler)
            .with_texture(depth_view)
            .with_texture(g_normal_view)
            .with_texture(g_material_view)
            .with_texture(ssr_surface_output.view().clone())
            .build(device);

        Self {
            output: ssr_surface_output,
            data: SsrComputeData { uniform },
        }
    }

    pub fn render(
        &self,
        camera_render_data: &RenderUniformData,
        encoder: &mut CommandEncoder,
        cache: &AssetCache,
    ) {
        let width = camera_render_data.system_data.screen_size.x.max(1);
        let height = camera_render_data.system_data.screen_size.y.max(1);
        let dispatch_x = width.div_ceil(8);
        let dispatch_y = height.div_ceil(8);

        let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("SSR Post Process Compute Pass"),
            ..ComputePassDescriptor::default()
        });

        let ssr_shader = cache.compute_shader(HComputeShader::POST_PROCESS_SSR);
        pass.set_pipeline(ssr_shader.pipeline());
        pass.set_bind_group(0, camera_render_data.uniform.bind_group(), &[]);
        pass.set_bind_group(1, self.data.uniform.bind_group(), &[]);
        pass.dispatch_workgroups(dispatch_x, dispatch_y, 1);
    }
}
