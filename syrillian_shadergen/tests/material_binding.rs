use syrillian_shadergen::MaterialCompiler;
use syrillian_shadergen::function::PbrShader;
use syrillian_shadergen::generator::MeshPass;

#[test]
fn recompiles_with_rebound_inputs() {
    let mut pbr = PbrShader::default();

    let compiled = MaterialCompiler::compile_mesh(&mut pbr, 0, MeshPass::Base);
    let recompiled = MaterialCompiler::compile_mesh(&mut pbr, 0, MeshPass::Base);

    assert!(!compiled.is_empty());
    assert_eq!(compiled, recompiled);
}
