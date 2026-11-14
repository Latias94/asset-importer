use asset_importer::{Scene, TextureType, animation::AnimInterpolation, version};

const GLTF_PNG_1X1: &str =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/p9sAAAAASUVORK5CYII=";

const GLTF_POSITIONS_BASE64: &str = "AAAAAAAAAAAAAAAAAACAPwAAAAAAAAAAAAAAAAAAgD8AAAAA";

const GLTF_CUBIC_TRANSLATION_BASE64: &str = "AAAAAAAAgD8AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAIA/AAAAAAAAAAAAAAAAAAAAAAAAAAAAAABAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 1e-5,
        "expected {expected}, got {actual}"
    );
}

fn regressions_available(minimum: (u32, u32, u32)) -> bool {
    let runtime = (
        version::assimp_version_major(),
        version::assimp_version_minor(),
        version::assimp_version_patch(),
    );

    if runtime >= minimum {
        return true;
    }

    let message = format!(
        "glTF regression requires Assimp >= {}.{}.{}, but runtime is {}.{}.{}",
        minimum.0, minimum.1, minimum.2, runtime.0, runtime.1, runtime.2
    );

    if !cfg!(feature = "system") {
        panic!(
            "{message}; source/prebuilt builds must match the bundled Assimp version. \
             If this happens in CI, clear restored target artifacts and verify submodules."
        );
    }

    eprintln!("{message}; skipping regression for this system library");
    false
}

fn material_texture_metadata_gltf() -> String {
    format!(
        r#"{{
  "asset": {{ "version": "2.0" }},
  "buffers": [
    {{
      "uri": "data:application/octet-stream;base64,{positions}",
      "byteLength": 36
    }}
  ],
  "bufferViews": [
    {{ "buffer": 0, "byteOffset": 0, "byteLength": 36, "target": 34962 }}
  ],
  "accessors": [
    {{
      "bufferView": 0,
      "componentType": 5126,
      "count": 3,
      "type": "VEC3",
      "min": [0, 0, 0],
      "max": [1, 1, 0]
    }}
  ],
  "images": [
    {{ "uri": "data:image/png;base64,{png}" }}
  ],
  "textures": [
    {{ "source": 0 }}
  ],
  "materials": [
    {{
      "name": "Material",
      "pbrMetallicRoughness": {{ "baseColorFactor": [1, 1, 1, 1] }},
      "normalTexture": {{ "index": 0, "scale": 0.42 }},
      "occlusionTexture": {{ "index": 0, "strength": 0.73 }}
    }}
  ],
  "meshes": [
    {{ "primitives": [{{ "attributes": {{ "POSITION": 0 }}, "material": 0 }}] }}
  ],
  "nodes": [
    {{ "mesh": 0 }}
  ],
  "scenes": [
    {{ "nodes": [0] }}
  ],
  "scene": 0
}}"#,
        positions = GLTF_POSITIONS_BASE64,
        png = GLTF_PNG_1X1
    )
}

fn cubic_spline_animation_gltf() -> String {
    format!(
        r#"{{
  "asset": {{ "version": "2.0" }},
  "buffers": [
    {{
      "uri": "data:application/octet-stream;base64,{animation}",
      "byteLength": 80
    }}
  ],
  "bufferViews": [
    {{ "buffer": 0, "byteOffset": 0, "byteLength": 8 }},
    {{ "buffer": 0, "byteOffset": 8, "byteLength": 72 }}
  ],
  "accessors": [
    {{
      "bufferView": 0,
      "componentType": 5126,
      "count": 2,
      "type": "SCALAR",
      "min": [0],
      "max": [1]
    }},
    {{
      "bufferView": 1,
      "componentType": 5126,
      "count": 6,
      "type": "VEC3"
    }}
  ],
  "nodes": [
    {{ "name": "AnimatedNode" }}
  ],
  "animations": [
    {{
      "name": "CubicTranslation",
      "samplers": [
        {{ "input": 0, "output": 1, "interpolation": "CUBICSPLINE" }}
      ],
      "channels": [
        {{ "sampler": 0, "target": {{ "node": 0, "path": "translation" }} }}
      ]
    }}
  ],
  "scenes": [
    {{ "nodes": [0] }}
  ],
  "scene": 0
}}"#,
        animation = GLTF_CUBIC_TRANSLATION_BASE64
    )
}

#[test]
fn gltf_import_preserves_normal_scale_and_occlusion_strength() {
    if !regressions_available((6, 0, 5)) {
        return;
    }

    let gltf = material_texture_metadata_gltf();
    let scene = Scene::from_memory(gltf.as_bytes(), Some("gltf")).expect("import glTF material");
    let material = scene.material(0).expect("material 0");

    assert_eq!(material.texture_count(TextureType::Normals), 1);
    assert_eq!(material.texture_count(TextureType::Lightmap), 1);
    assert!(material.normal_texture(0).is_some());
    assert!(material.occlusion_texture(0).is_some());
    assert!(material.ambient_occlusion_texture(0).is_none());

    assert_close(material.normal_texture_scale(0).unwrap(), 0.42);
    assert_close(
        material.texture_strength(TextureType::Lightmap, 0).unwrap(),
        0.73,
    );
    assert_close(material.occlusion_texture_strength(0).unwrap(), 0.73);
}

#[test]
fn gltf_import_preserves_cubic_spline_translation_tangents() {
    if !regressions_available((6, 0, 5)) {
        return;
    }

    let gltf = cubic_spline_animation_gltf();
    let scene = Scene::from_memory(gltf.as_bytes(), Some("gltf")).expect("import glTF animation");
    let animation = scene.animation(0).expect("animation 0");
    let channel = animation.channel(0).expect("animation channel 0");
    let keys = channel.position_keys();

    assert_eq!(keys.len(), 6);
    assert!(
        keys.iter()
            .all(|key| key.interpolation == AnimInterpolation::CubicSpline)
    );

    assert_eq!(keys[0].time, 0.0);
    assert_eq!(keys[1].time, 0.0);
    assert_eq!(keys[2].time, 0.0);
    assert_eq!(keys[3].time, 1000.0);
    assert_eq!(keys[4].time, 1000.0);
    assert_eq!(keys[5].time, 1000.0);

    assert_close(keys[0].value.x, 0.0);
    assert_close(keys[1].value.x, 0.0);
    assert_close(keys[2].value.x, 1.0);
    assert_close(keys[3].value.x, 0.0);
    assert_close(keys[4].value.x, 2.0);
    assert_close(keys[5].value.x, 0.0);
}

#[test]
fn gltf_texture_transform_accounts_for_assimp_coordinate_space() {
    if !regressions_available((6, 0, 6)) {
        return;
    }

    let gltf = material_texture_metadata_gltf()
        .replacen('{', r#"{"extensionsUsed":["KHR_texture_transform"],"#, 1)
        .replace(
            "\"baseColorFactor\": [1, 1, 1, 1]",
            r#""baseColorFactor": [1, 1, 1, 1], "baseColorTexture": {
          "index": 0,
          "extensions": { "KHR_texture_transform": {
            "offset": [0.2, 0.3], "scale": [2, 3]
          } }
        }"#,
        );
    let scene =
        Scene::from_memory(gltf.as_bytes(), Some("gltf")).expect("import texture transform");
    let material = scene.material(0).expect("material 0");
    let texture = material.base_color_texture(0).expect("base color texture");
    let transform = texture.uv_transform.expect("UV transform");

    assert_close(transform.translation.x, 0.35);
    assert_close(transform.translation.y, -0.433_333_34);
    assert_close(transform.scaling.x, 2.0);
    assert_close(transform.scaling.y, 3.0);
    assert_close(transform.rotation, 0.0);
}

// Fixtures originate from Assimp v6.0.6's glTF2 tests under its BSD-3-Clause license.
#[test]
fn gltf_normalized_unsigned_short_skin_weights() {
    if !regressions_available((6, 0, 6)) {
        return;
    }

    let scene = Scene::from_memory(
        include_bytes!("fixtures/gltf/normalized_skin_weights.gltf"),
        Some("gltf"),
    )
    .expect("import normalized skin weights");
    let mesh = scene.mesh(0).expect("mesh 0");
    let bones: Vec<_> = mesh.bones().collect();
    assert_eq!(bones.len(), 1);
    assert_eq!(bones[0].name(), "joint0");
    let weights = bones[0].weights_raw();
    assert_eq!(weights.len(), 3);
    for (index, expected) in [1.0, 32768.0 / 65535.0, 16384.0 / 65535.0]
        .into_iter()
        .enumerate()
    {
        assert_eq!(weights[index].mVertexId, index as u32);
        assert_close(weights[index].mWeight, expected);
    }
}

#[test]
fn gltf_mixed_joint_component_types_preserve_weights() {
    if !regressions_available((6, 0, 6)) {
        return;
    }

    let scene = Scene::from_memory(
        include_bytes!("fixtures/gltf/mixed_skin_joints.gltf"),
        Some("gltf"),
    )
    .expect("import mixed joint accessors");
    let mesh = scene.mesh(0).expect("mesh 0");
    let bones: Vec<_> = mesh.bones().collect();
    assert_eq!(bones.len(), 2);
    assert_eq!(bones[0].weights_raw().len(), 3);
    let weights = bones[1].weights_raw();
    assert_eq!(weights.len(), 2);
    assert_eq!(weights[0].mVertexId, 1);
    assert_close(weights[0].mWeight, 32767.0 / 65535.0);
    assert_eq!(weights[1].mVertexId, 2);
    assert_close(weights[1].mWeight, 49151.0 / 65535.0);
}
