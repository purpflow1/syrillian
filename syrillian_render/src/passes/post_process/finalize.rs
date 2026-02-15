use crate::passes::post_process::{
    PostProcessData, PostProcessPass, PostProcessPassContext, PostProcessRoute,
    PostProcessSharedViews,
};
use wgpu::{
    BindGroupLayout, Color, Device, LoadOp, Operations, RenderPassColorAttachment,
    RenderPassDescriptor, StoreOp, TextureView,
};

pub struct FinalRenderPass {
    uniform: PostProcessData,
}

impl FinalRenderPass {
    pub fn new(
        device: &Device,
        post_process_bgl: &BindGroupLayout,
        shared_views: &PostProcessSharedViews,
        route: &PostProcessRoute,
    ) -> Self {
        let uniform = PostProcessData::new(
            device,
            post_process_bgl.clone(),
            route.input_color.clone(),
            shared_views.depth.clone(),
            shared_views.g_normal.clone(),
            shared_views.g_material.clone(),
        );

        Self { uniform }
    }
}

impl PostProcessPass for FinalRenderPass {
    fn name(&self) -> &'static str {
        "Final"
    }

    fn execute(&mut self, ctx: &mut PostProcessPassContext<'_>, output_color: &TextureView) {
        let mut pass = ctx.encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Final Render Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: output_color,
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

        let shader = ctx.cache.shader_post_process();
        let groups = shader.bind_groups();
        pass.set_pipeline(shader.solid_pipeline());
        pass.set_bind_group(
            groups.render,
            ctx.camera_render_data.uniform.bind_group(),
            &[],
        );
        if let Some(idx) = groups.post_process {
            pass.set_bind_group(idx, self.uniform.uniform.bind_group(), &[]);
        }
        pass.draw(0..6, 0..1);
    }
}
