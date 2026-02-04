mod pipeline;
mod ssr;
mod ui_pass;

pub use pipeline::RenderPipeline;
pub use ssr::ScreenSpaceReflectionRenderPass;

pub trait PostProcess {
    fn render(&self);
}
