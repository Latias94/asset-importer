use asset_importer::{
    Scene, TextureType,
    material::{ShadingModel, TextureMapMode, TextureMapping, TextureOperation},
};

fn material_gltf(material: &str) -> String {
    format!(
        r#"{{
  "asset": {{ "version": "2.0" }},
  "extensionsUsed": ["KHR_materials_pbrSpecularGlossiness"],
  "buffers": [{{
    "uri": "data:application/octet-stream;base64,AAAAAAAAAAAAAAAAAACAPwAAAAAAAAAAAAAAAAAAgD8AAAAA",
    "byteLength": 36
  }}],
  "bufferViews": [{{ "buffer": 0, "byteOffset": 0, "byteLength": 36, "target": 34962 }}],
  "accessors": [{{
    "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3",
    "min": [0, 0, 0], "max": [1, 1, 0]
  }}],
  "images": [{{
    "uri": "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/p9sAAAAASUVORK5CYII="
  }}],
  "textures": [{{ "source": 0 }}],
  "materials": [{material}],
  "meshes": [{{ "primitives": [{{ "attributes": {{ "POSITION": 0 }}, "material": 0 }}] }}],
  "nodes": [{{ "mesh": 0 }}],
  "scenes": [{{ "nodes": [0] }}],
  "scene": 0
}}"#
    )
}

#[test]
fn gltf_shading_mode_preserves_generic_pbr_for_both_workflows() {
    for (definition, specular_glossiness) in [
        (
            r#"{
              "pbrMetallicRoughness": { "metallicFactor": 0.6, "roughnessFactor": 0.4 }
            }"#,
            false,
        ),
        (
            r#"{
              "extensions": {
                "KHR_materials_pbrSpecularGlossiness": {
                  "diffuseFactor": [1, 1, 1, 1],
                  "specularFactor": [0.5, 0.5, 0.5],
                  "glossinessFactor": 0.7
                }
              }
            }"#,
            true,
        ),
    ] {
        let gltf = material_gltf(definition);
        let scene = Scene::from_memory(gltf.as_bytes(), Some("gltf")).expect("import PBR glTF");
        let material = scene.material(0).expect("material 0");
        assert_eq!(material.shading_model_enum(), Some(ShadingModel::PbrBrdf));
        if specular_glossiness {
            assert!((material.glossiness_factor().unwrap() - 0.7).abs() < 1e-5);
        } else {
            assert!((material.metallic_factor().unwrap() - 0.6).abs() < 1e-5);
            assert!((material.roughness_factor().unwrap() - 0.4).abs() < 1e-5);
        }
    }
}

#[test]
fn gltf_emission_aliases_find_emissive_slot_with_optional_output_defaults() {
    let gltf = material_gltf(
        r#"{
          "pbrMetallicRoughness": {},
          "emissiveFactor": [1, 1, 1],
          "emissiveTexture": { "index": 0 }
        }"#,
    );
    let scene = Scene::from_memory(gltf.as_bytes(), Some("gltf")).expect("import emissive glTF");
    let material = scene.material(0).expect("material 0");
    assert_eq!(material.texture_count(TextureType::Emissive), 1);
    assert_eq!(material.texture_count(TextureType::EmissionColor), 0);

    let exact = material.texture(TextureType::Emissive, 0).unwrap();
    assert_eq!(material.emission_texture(0).unwrap().path, exact.path);
    assert_eq!(material.emissive_texture(0).unwrap().path, exact.path);
    assert!(material.emission_texture(1).is_none());
    assert!(material.emissive_texture(1).is_none());
    assert_eq!(exact.mapping, TextureMapping::UV);
    assert_eq!(exact.uv_index, 0);
    assert_eq!(exact.blend_factor, 1.0);
    assert_eq!(exact.operation, TextureOperation::Multiply);
    assert_eq!(exact.map_modes, [TextureMapMode::Wrap; 3]);
    assert!(exact.flags.is_empty());
}
