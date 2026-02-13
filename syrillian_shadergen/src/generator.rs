use wgpu::TextureFormat;

pub const PICKING_TEXTURE_FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;

const POST_PROCESS_VERTEX: &str = include_str!("functions/vertex_postprocess_quad.wgsl");
const MESH3D_NO_BONES_GROUP: &str = include_str!("groups/mesh3d.wgsl");
const MESH3D_VERTEX_NO_BONES: &str = include_str!("functions/vertex_mesh3d.wgsl");
const MATH_HELPERS: &str = include_str!("functions/helpers/math.wgsl");
const MESH3D_PBR: &str = include_str!("functions/pbr_mesh3d.wgsl");

const POST_PROCESS_GROUP: &str = include_str!("groups/post_process.wgsl");
const RENDER_GROUP: &str = include_str!("groups/render.wgsl");
const MODEL_GROUP: &str = include_str!("groups/model.wgsl");
const MATERIAL_GROUP: &str = include_str!("groups/material.wgsl");
const MATERIAL_TEXTURES_GROUP: &str = include_str!("groups/material_textures.wgsl");
const LIGHT_GROUP: &str = include_str!("groups/light.wgsl");

#[derive(Debug, Copy, Clone)]
pub enum MeshPass {
    Base,
    Picking,
    Shadow,
}

pub struct MaterialShaderSetCode {
    pub base: String,
    pub picking: String,
    pub shadow: String,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ShaderKind {
    Default,
    Custom,
    PostProcess,
    Compute,
}

#[derive(Debug, Copy, Clone)]
pub struct MaterialGroupOverrides<'a> {
    pub material: &'a str,
    pub material_textures: &'a str,
}

#[derive(Debug, Clone)]
pub struct ShaderCompilationOutput {
    pub lines: Vec<String>,
    pub result_expr: String,
}

pub struct ShaderGenerator;

impl ShaderGenerator {
    pub fn assemble_shader(
        source: &str,
        fragment_only: bool,
        kind: ShaderKind,
        depth_enabled: bool,
        material_groups: Option<MaterialGroupOverrides<'_>>,
    ) -> String {
        let (material_group, material_textures_group) = material_groups
            .map(|g| (g.material, g.material_textures))
            .unwrap_or((MATERIAL_GROUP, MATERIAL_TEXTURES_GROUP));

        let mut out = String::new();

        match kind {
            ShaderKind::PostProcess => {
                out.push_str(RENDER_GROUP);
                out.push('\n');
                out.push_str(POST_PROCESS_GROUP);
                out.push('\n');

                if fragment_only {
                    out.push_str(POST_PROCESS_VERTEX);
                    out.push('\n');
                }

                out.push_str(source);
                out
            }
            ShaderKind::Default => {
                out.push_str(RENDER_GROUP);
                out.push('\n');
                push_default_mesh_groups(&mut out);
                out.push('\n');
                out.push_str(MODEL_GROUP);
                out.push('\n');
                out.push_str(material_group);
                out.push('\n');
                out.push_str(material_textures_group);
                out.push('\n');

                if depth_enabled {
                    out.push_str(LIGHT_GROUP);
                    out.push('\n');
                }

                if fragment_only {
                    out.push_str(MESH3D_VERTEX_NO_BONES);
                    out.push('\n');
                }

                out.push_str(source);
                out
            }
            ShaderKind::Custom => {
                let uses_directive = source.lines().any(|line| line.contains("#use "));
                let has_render_group = source.contains("@group(0)");
                if !has_render_group {
                    out.push_str(RENDER_GROUP);
                    out.push('\n');
                }

                for line in source.lines() {
                    let Some(import) = line.find("#use ") else {
                        out.push_str(line);
                        out.push('\n');
                        continue;
                    };

                    let group = line[import + 5..].trim();
                    match group {
                        "model" => out.push_str(MODEL_GROUP),
                        "material" => out.push_str(material_group),
                        "material_textures" => out.push_str(material_textures_group),
                        "light" | "shadow" => out.push_str(LIGHT_GROUP),
                        "default_vertex" => push_default_header(&mut out),
                        "post_process" => out.push_str(POST_PROCESS_GROUP),
                        "render" => out.push_str(RENDER_GROUP),
                        _ => {}
                    }
                    out.push('\n');
                }

                if fragment_only {
                    out.push_str(MESH3D_VERTEX_NO_BONES);
                    out.push('\n');
                }

                if !uses_directive && has_render_group && !fragment_only {
                    // If we didn't insert anything, just return the original source.
                    return source.to_string();
                }

                out
            }
            ShaderKind::Compute => {
                let uses_directive = source.lines().any(|line| line.contains("#use "));
                if !uses_directive {
                    return source.to_string();
                }

                for line in source.lines() {
                    let Some(import) = line.find("#use ") else {
                        out.push_str(line);
                        out.push('\n');
                        continue;
                    };

                    let group = line[import + 5..].trim();
                    match group {
                        "render" => out.push_str(RENDER_GROUP),
                        "model" => out.push_str(MODEL_GROUP),
                        "material" => out.push_str(material_group),
                        "material_textures" => out.push_str(material_textures_group),
                        "light" | "shadow" => out.push_str(LIGHT_GROUP),
                        "post_process" => out.push_str(POST_PROCESS_GROUP),
                        _ => {}
                    }
                    out.push('\n');
                }

                out
            }
        }
    }

    pub fn build_mesh_shader(compiled: &ShaderCompilationOutput, pass: MeshPass) -> String {
        let mut out = String::new();

        out.push_str(MATH_HELPERS);
        out.push_str(MESH3D_NO_BONES_GROUP);
        out.push('\n');

        out.push_str(RENDER_GROUP);
        out.push('\n');
        out.push_str(MODEL_GROUP);
        out.push('\n');

        match pass {
            MeshPass::Base | MeshPass::Shadow => {
                out.push_str("#use material\n");
                out.push_str("#use material_textures\n");
                out.push_str("#use light\n");
            }
            MeshPass::Picking => {}
        }
        out.push('\n');

        if matches!(pass, MeshPass::Base | MeshPass::Shadow) {
            out.push_str(MESH3D_PBR);
            out.push('\n');
        }

        if matches!(pass, MeshPass::Picking) {
            out.push_str("struct PickColor {\n    color: vec4<f32>,\n};\n\n");
            out.push_str("var<immediate> pick: PickColor;\n\n");
        }

        out.push_str(MESH3D_VERTEX_NO_BONES);
        out.push('\n');

        let ret = match pass {
            MeshPass::Picking => "@location(0) vec4f",
            MeshPass::Base | MeshPass::Shadow => "FOutput",
        };

        out.push_str("@fragment\nfn fs_main(in: FInput) -> ");
        out.push_str(ret);
        out.push_str(" {\n");
        append_compilation_output(&mut out, compiled);
        out.push_str("}\n");

        out
    }

    pub fn build_post_process_shader(compiled: &ShaderCompilationOutput) -> String {
        let mut out = String::new();
        out.push_str(POST_PROCESS_GROUP);
        out.push('\n');
        out.push_str(POST_PROCESS_VERTEX);
        out.push('\n');
        out.push_str("@fragment\nfn fs_main(in: FInput) -> @location(0) vec4f {\n");
        append_compilation_output(&mut out, compiled);
        out.push_str("}\n");
        out
    }

    pub fn build_post_process_fragment(compiled: &ShaderCompilationOutput) -> String {
        let mut out = String::new();
        out.push_str("@fragment\nfn fs_main(in: FInput) -> @location(0) vec4f {\n");
        append_compilation_output(&mut out, compiled);
        out.push_str("}\n");
        out
    }
}

pub fn assemble_shader(
    source: &str,
    fragment_only: bool,
    kind: ShaderKind,
    depth_enabled: bool,
    material_groups: Option<MaterialGroupOverrides<'_>>,
) -> String {
    ShaderGenerator::assemble_shader(source, fragment_only, kind, depth_enabled, material_groups)
}

pub fn assemble_compute_shader(source: &str) -> String {
    ShaderGenerator::assemble_shader(source, false, ShaderKind::Compute, false, None)
}

fn push_default_header(out: &mut String) {
    out.push_str(MATH_HELPERS);
    out.push('\n');
    out.push_str(MESH3D_NO_BONES_GROUP);
    out.push('\n');
}

fn push_default_mesh_groups(out: &mut String) {
    out.push_str(MATH_HELPERS);
    out.push('\n');
    out.push_str(MESH3D_NO_BONES_GROUP);
    out.push('\n');
}

fn append_compilation_output(out: &mut String, compiled: &ShaderCompilationOutput) {
    for stmt in &compiled.lines {
        for line in stmt.lines() {
            out.push_str("    ");
            out.push_str(line);
            out.push('\n');
        }
    }

    out.push_str("    return ");
    out.push_str(&compiled.result_expr);
    out.push_str(";\n");
}
