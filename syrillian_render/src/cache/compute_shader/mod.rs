use crate::cache::AssetCache;
use crate::cache::generic_cache::CacheType;
use std::borrow::Cow;
use std::sync::Arc;
use syrillian_asset::ComputeShader;
use syrillian_shadergen::generator::assemble_compute_shader;
use wgpu::{
    ComputePipeline, ComputePipelineDescriptor, Device, PipelineCompilationOptions, Queue,
    ShaderModule, ShaderModuleDescriptor, ShaderSource,
};

#[derive(Debug, Clone)]
pub struct RuntimeComputeShader {
    name: String,
    module: ShaderModule,
    pipeline: ComputePipeline,
}

impl CacheType for ComputeShader {
    type Hot = Arc<RuntimeComputeShader>;

    #[profiling::function]
    fn upload(self, device: &Device, _queue: &Queue, cache: &AssetCache) -> Self::Hot {
        let code = assemble_compute_shader(self.code());
        let module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some(self.name()),
            source: ShaderSource::Wgsl(Cow::Owned(code)),
        });

        let bgls = self
            .bind_group_layouts()
            .iter()
            .map(|h| {
                cache
                    .bgl(*h)
                    .expect("Compute shader bind group layout should exist")
            })
            .collect::<Vec<_>>();
        let bgl_refs = bgls.iter().collect::<Vec<_>>();

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(&format!("{} Pipeline Layout", self.name())),
            bind_group_layouts: &bgl_refs,
            immediate_size: 0,
        });

        let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some(&format!("{} Pipeline", self.name())),
            layout: Some(&layout),
            module: &module,
            entry_point: Some(self.entry_point()),
            compilation_options: PipelineCompilationOptions::default(),
            cache: None,
        });

        Arc::new(RuntimeComputeShader {
            name: self.name().to_string(),
            module,
            pipeline,
        })
    }
}

impl RuntimeComputeShader {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn module(&self) -> &ShaderModule {
        &self.module
    }

    pub fn pipeline(&self) -> &ComputePipeline {
        &self.pipeline
    }
}
