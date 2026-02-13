use wgpu::{
    Device, Extent3d, SurfaceConfiguration, Texture, TextureDescriptor, TextureDimension,
    TextureUsages, TextureView, TextureViewDescriptor,
};

pub struct OffscreenSurface {
    texture: Texture,
    view: TextureView,
}

impl OffscreenSurface {
    pub fn new(device: &Device, config: &SurfaceConfiguration) -> Self {
        Self::new_with(device, config, config.format, TextureUsages::empty())
    }

    pub fn new_with(
        device: &Device,
        config: &SurfaceConfiguration,
        format: wgpu::TextureFormat,
        extra_usage: TextureUsages,
    ) -> Self {
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("Offscreen Texture"),
            size: Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format,
            usage: TextureUsages::RENDER_ATTACHMENT
                | TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_SRC
                | TextureUsages::COPY_DST
                | extra_usage,
            view_formats: &[],
        });

        let view = texture.create_view(&TextureViewDescriptor::default());

        OffscreenSurface { texture, view }
    }

    #[profiling::function]
    pub fn recreate(&mut self, device: &Device, config: &SurfaceConfiguration) {
        *self = Self::new(device, config);
    }

    pub fn view(&self) -> &TextureView {
        &self.view
    }

    pub fn texture(&self) -> &Texture {
        &self.texture
    }
}
