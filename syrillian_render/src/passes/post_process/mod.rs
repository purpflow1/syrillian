mod bloom;
mod finalize;
mod fxaa;
mod ssao;
mod ssr;

use crate::cache::AssetCache;
use crate::rendering::render_data::RenderUniformData;
use crate::rendering::uniform::ShaderUniform;
pub use bloom::{BloomRenderPass, BloomSettings};
pub use finalize::FinalRenderPass;
pub use fxaa::FxaaRenderPass;
pub use ssao::ScreenSpaceAmbientOcclusionRenderPass;
pub use ssr::ScreenSpaceReflectionRenderPass;
use syrillian_macros::UniformIndex;
use wgpu::{
    AddressMode, BindGroupLayout, CommandEncoder, Device, FilterMode, MipmapFilterMode,
    SamplerDescriptor, TextureView,
};

#[derive(Clone)]
pub struct PostProcessSharedViews {
    pub depth: TextureView,
    pub g_normal: TextureView,
    pub g_material: TextureView,
    pub g_velocity: TextureView,
}

pub struct PostProcessPassContext<'a> {
    pub camera_render_data: &'a RenderUniformData,
    pub encoder: &'a mut CommandEncoder,
    pub cache: &'a AssetCache,
}

pub trait PostProcessPass {
    fn name(&self) -> &'static str;
    fn execute(&mut self, ctx: &mut PostProcessPassContext<'_>, output_color: &TextureView);
}

#[derive(Clone)]
pub struct PostProcessRoute {
    pub input_id: u32,
    pub output_id: u32,
    pub input_color: TextureView,
    pub output_color: TextureView,
}

#[repr(u8)]
#[derive(Debug, Copy, Clone, UniformIndex)]
pub enum PostProcessUniformIndex {
    Color = 0,
    Sampler = 1,
    Depth = 2,
    GNormal = 3,
    GMaterial = 4,
}

pub struct PostProcessData {
    pub uniform: ShaderUniform<PostProcessUniformIndex>,
}

impl PostProcessData {
    pub fn new(
        device: &Device,
        layout: BindGroupLayout,
        color_view: TextureView,
        depth_view: TextureView,
        g_normal_view: TextureView,
        g_material_view: TextureView,
    ) -> Self {
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("PostProcess Sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_filter: MipmapFilterMode::Nearest,
            ..SamplerDescriptor::default()
        });

        let uniform = ShaderUniform::<PostProcessUniformIndex>::builder(layout)
            .with_texture(color_view)
            .with_sampler(sampler)
            .with_texture(depth_view)
            .with_texture(g_normal_view)
            .with_texture(g_material_view)
            .build(device);

        Self { uniform }
    }
}
