use crate::assets::{H, HandleName, StoreType, TextureAsset};
use crate::rendering::TextureFormat;
use wgpu::{
    AddressMode, FilterMode, MipmapFilterMode, TextureDimension, TextureUsages,
    TextureViewDimension,
};

#[derive(Debug, Clone)]
pub struct RenderCubemap {
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
}

impl StoreType for RenderCubemap {
    fn name() -> &'static str {
        "Render Cubemap"
    }

    fn ident_fmt(handle: H<Self>) -> HandleName<Self> {
        HandleName::Id(handle)
    }

    fn is_builtin(_handle: H<Self>) -> bool {
        false
    }
}

impl TextureAsset for RenderCubemap {
    fn layer_count(&self) -> u32 {
        6
    }

    fn flags(&self) -> TextureUsages {
        TextureUsages::TEXTURE_BINDING
            | TextureUsages::RENDER_ATTACHMENT
            | TextureUsages::COPY_SRC
            | TextureUsages::COPY_DST
    }

    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn format(&self) -> TextureFormat {
        self.format
    }

    fn view_formats(&self) -> &[TextureFormat] {
        std::slice::from_ref(&self.format)
    }

    fn mip_level_count(&self) -> u32 {
        1
    }

    fn sample_count(&self) -> u32 {
        1
    }

    fn dimensions(&self) -> TextureDimension {
        TextureDimension::D2
    }

    fn view_dimension(&self) -> TextureViewDimension {
        TextureViewDimension::Cube
    }

    fn repeat_mode(&self) -> AddressMode {
        AddressMode::Repeat
    }

    fn filter_mode(&self) -> FilterMode {
        FilterMode::Nearest
    }

    fn mip_filter_mode(&self) -> MipmapFilterMode {
        MipmapFilterMode::Nearest
    }

    fn data(&self) -> Option<&[u8]> {
        None
    }

    fn has_transparency(&self) -> bool {
        false
    }
}
