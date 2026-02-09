use std::error::Error;
use std::f32::consts::PI;
use syrillian::SyrillianApp;
use syrillian::assets::material_inputs::{
    MaterialImmediateDef, MaterialInputLayout, MaterialTextureDef,
};
use syrillian::assets::store::StoreType;
use syrillian::assets::{HMaterialInstance, HTexture2D, MaterialInstance, Texture2D};
use syrillian::core::GameObjectExt;
#[cfg(debug_assertions)]
use syrillian::rendering::DebugRenderer;
use syrillian::shadergen::function::{
    ExpressionInput, ExpressionTexture, MaterialExpression, MaterialExpressionValue,
};
use syrillian::shadergen::value::{MaterialValue, MaterialValueType};
use syrillian::shadergen::{MaterialCompiler, NodeId};
use syrillian::{AppState, World};
use syrillian_components::RotateComponent;
use syrillian_components::prefabs::CubePrefab;

#[derive(Clone, Copy, Debug)]
struct WavyBlendMaterial {
    texture_a: ExpressionTexture,
    texture_b: ExpressionTexture,
    blend_amount: ExpressionInput<f32>,
    wave_phase: ExpressionInput<f32>,
}

impl Default for WavyBlendMaterial {
    fn default() -> Self {
        Self {
            texture_a: ExpressionTexture::material("texture_a"),
            texture_b: ExpressionTexture::material("texture_b"),
            blend_amount: ExpressionInput::material("blend_amount"),
            wave_phase: ExpressionInput::material("wave_phase"),
        }
    }
}

impl MaterialExpression for WavyBlendMaterial {
    fn bind_inputs(&mut self, compiler: &mut MaterialCompiler) {
        self.texture_a.bind(compiler);
        self.texture_b.bind(compiler);
        self.blend_amount.bind(compiler);
        self.wave_phase.bind(compiler);
    }

    fn outputs(&self) -> Vec<MaterialExpressionValue> {
        vec![MaterialExpressionValue {
            name: "out",
            value_type: MaterialValueType::Vec4,
        }]
    }

    fn compile(&self, compiler: &mut MaterialCompiler, _output_index: u32) -> NodeId {
        let uv = compiler.vertex_uv();

        let zero = compiler.constant_f32(0.0);
        let half = compiler.constant_f32(0.5);
        let one = compiler.constant_f32(1.0);
        let phase = self.wave_phase.node();

        let wave_freq_x = compiler.constant_f32(18.0);
        let wave_freq_y = compiler.constant_f32(26.0);
        let curl_freq = compiler.constant_f32(22.0);
        let phase_scale_b = compiler.constant_f32(1.3);
        let phase_scale_c = compiler.constant_f32(0.7);
        let wave_b_weight = compiler.constant_f32(0.75);
        let curl_weight = compiler.constant_f32(0.5);
        let wave_norm = compiler.constant_f32(2.25);

        let offset_main = compiler.constant_f32(0.05);
        let offset_minor = compiler.constant_f32(0.03);
        let offset_curl = compiler.constant_f32(0.045);

        let uv_x = compiler.swizzle(uv, "x");
        let uv_y = compiler.swizzle(uv, "y");

        let wave_a_phase_x = compiler.mul(uv_x, wave_freq_x);
        let wave_a_phase = compiler.add(wave_a_phase_x, phase);
        let wave_a = compiler.call("sin", vec![wave_a_phase]);

        let phase_b = compiler.mul(phase, phase_scale_b);
        let wave_b_phase_y = compiler.mul(uv_y, wave_freq_y);
        let wave_b_phase = compiler.sub(wave_b_phase_y, phase_b);
        let wave_b = compiler.call("cos", vec![wave_b_phase]);

        let uv_sum = compiler.add(uv_x, uv_y);
        let curl_phase_uv = compiler.mul(uv_sum, curl_freq);
        let phase_c = compiler.mul(phase, phase_scale_c);
        let curl_phase = compiler.add(curl_phase_uv, phase_c);
        let curl_wave = compiler.call("sin", vec![curl_phase]);

        let wave_b_weighted = compiler.mul(wave_b, wave_b_weight);
        let curl_weighted = compiler.mul(curl_wave, curl_weight);
        let wave_mix_ab = compiler.add(wave_a, wave_b_weighted);
        let wave_mix_total = compiler.add(wave_mix_ab, curl_weighted);
        let wave_mix_norm = compiler.div(wave_mix_total, wave_norm);

        let offset_x_main = compiler.mul(wave_a, offset_main);
        let offset_x_minor = compiler.mul(wave_b, offset_minor);
        let offset_x = compiler.add(offset_x_main, offset_x_minor);
        let offset_y_curl = compiler.mul(curl_wave, offset_curl);
        let offset_y_minor = compiler.mul(wave_a, offset_minor);
        let offset_y = compiler.sub(offset_y_curl, offset_y_minor);

        let uv_offset = compiler.call("vec2<f32>", vec![offset_x, offset_y]);
        let uv_wavy = compiler.add(uv, uv_offset);

        let sample_a = compiler.call(
            "textureSample",
            vec![
                self.texture_a.texture_node(),
                self.texture_a.sampler_node(),
                uv,
            ],
        );
        let sample_b = compiler.call(
            "textureSample",
            vec![
                self.texture_b.texture_node(),
                self.texture_b.sampler_node(),
                uv_wavy,
            ],
        );

        let shifted_wave = compiler.add(wave_mix_norm, one);
        let wave_01 = compiler.mul(shifted_wave, half);
        let blend_threshold = compiler.call("clamp", vec![self.blend_amount.node(), zero, one]);
        let blend_edge = compiler.constant_f32(0.08);
        let blend_low = compiler.sub(blend_threshold, blend_edge);
        let blend_high = compiler.add(blend_threshold, blend_edge);
        let blend = compiler.call("smoothstep", vec![blend_low, blend_high, wave_01]);
        let inverse_blend = compiler.sub(one, blend);

        let color_a = compiler.mul(sample_a, inverse_blend);
        let color_b = compiler.mul(sample_b, blend);
        let base_rgba = compiler.add(color_a, color_b);

        let normal = compiler.call("vec3<f32>", vec![zero, zero, one]);
        let roughness = compiler.constant_f32(0.6);
        let metallic = compiler.constant_f32(0.0);
        let alpha = one;
        let false_u32 = compiler.call("u32", vec![zero]);

        compiler.pbr_shader(
            base_rgba, normal, roughness, metallic, alpha, false_u32, false_u32, false_u32,
        )
    }
}

fn wavy_material_layout() -> MaterialInputLayout {
    MaterialInputLayout {
        immediates: vec![
            MaterialImmediateDef {
                name: "blend_amount".to_string(),
                ty: MaterialValueType::F32,
                default: MaterialValue::F32(0.5),
            },
            MaterialImmediateDef {
                name: "wave_phase".to_string(),
                ty: MaterialValueType::F32,
                default: MaterialValue::F32(0.0),
            },
        ],
        textures: vec![
            MaterialTextureDef {
                name: "texture_a".to_string(),
                default: HTexture2D::FALLBACK_DIFFUSE,
            },
            MaterialTextureDef {
                name: "texture_b".to_string(),
                default: HTexture2D::FALLBACK_DIFFUSE,
            },
        ],
    }
}

#[derive(Debug, SyrillianApp)]
struct MaterialShaderInstances {
    white_to_blue: HMaterialInstance,
    blue_to_white: HMaterialInstance,
    wave_time: f32,
}

impl Default for MaterialShaderInstances {
    fn default() -> Self {
        Self {
            white_to_blue: HMaterialInstance::DEFAULT,
            blue_to_white: HMaterialInstance::DEFAULT,
            wave_time: 0.0,
        }
    }
}

impl AppState for MaterialShaderInstances {
    fn init(&mut self, world: &mut World) -> Result<(), Box<dyn Error>> {
        let camera = world.new_camera();
        let mut camera_object = camera.parent();
        camera_object.transform.set_position(0.0, 0.0, 2.5);

        let texture_format = world
            .assets
            .textures
            .get(HTexture2D::FALLBACK_DIFFUSE)
            .format;

        let blue_texture = Texture2D::load_pixels_with_transparency(
            vec![255, 0, 0, 255],
            1,
            1,
            texture_format,
            false,
        )
        .store(world);
        let white_texture = Texture2D::load_pixels_with_transparency(
            vec![255, 255, 255, 255],
            1,
            1,
            texture_format,
            false,
        )
        .store(world);

        let wavy_material = world.assets.register_custom_material_with_layout(
            "Wavy Texture Blend Material",
            WavyBlendMaterial::default(),
            wavy_material_layout(),
        );

        let white_to_blue = MaterialInstance::builder()
            .name("Wavy White -> Blue")
            .material(wavy_material)
            .texture("texture_a", white_texture)
            .texture("texture_b", blue_texture)
            .value("blend_amount", MaterialValue::F32(0.5))
            .value("wave_phase", MaterialValue::F32(0.0))
            .build()
            .store(world);

        let blue_to_white = MaterialInstance::builder()
            .name("Wavy Blue -> White")
            .material(wavy_material)
            .texture("texture_a", blue_texture)
            .texture("texture_b", white_texture)
            .value("blend_amount", MaterialValue::F32(0.5))
            .value("wave_phase", MaterialValue::F32(PI))
            .build()
            .store(world);

        self.white_to_blue = white_to_blue;
        self.blue_to_white = blue_to_white;

        world
            .spawn(&CubePrefab::new(white_to_blue))
            .at(-1.7, 0.0, -6.0)
            .scale(1.4)
            .build_component::<RotateComponent>()
            .speed(45.0);

        world
            .spawn(&CubePrefab::new(blue_to_white))
            .at(1.7, 0.0, -6.0)
            .scale(1.4)
            .build_component::<RotateComponent>()
            .speed(-45.0);

        #[cfg(debug_assertions)]
        DebugRenderer::off();

        Ok(())
    }

    fn update(&mut self, world: &mut World) -> Result<(), Box<dyn Error>> {
        self.wave_time += world.delta_time().as_secs_f32() * 1.35;
        let blend_amount = 0.5 + (self.wave_time * 0.35).sin() * 0.08;

        if let Some(mut material) = world
            .assets
            .material_instances
            .try_get_mut(self.white_to_blue)
        {
            material
                .values
                .insert("wave_phase".to_string(), MaterialValue::F32(self.wave_time));
            material
                .values
                .insert("blend_amount".to_string(), MaterialValue::F32(blend_amount));
        }

        if let Some(mut material) = world
            .assets
            .material_instances
            .try_get_mut(self.blue_to_white)
        {
            material.values.insert(
                "wave_phase".to_string(),
                MaterialValue::F32(self.wave_time + PI),
            );
            material
                .values
                .insert("blend_amount".to_string(), MaterialValue::F32(blend_amount));
        }

        world.input.auto_quit_on_escape();
        Ok(())
    }
}
