use asset_importer::{Scene, TextureType, animation::AnimInterpolation};

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
