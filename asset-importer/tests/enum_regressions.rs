use asset_importer::{Importer, animation::AnimBehaviour, light::LightType};

#[test]
fn imported_native_enums_preserve_unknown_values() {
    // This minimal ASSBIN 1.0 fixture stores light type 7 and animation behavior 7/8.
    // It exercises the real loader rather than constructing an invalid Rust enum.
    let scene = Importer::new()
        .import_from_memory(
            include_bytes!("fixtures/assbin/unknown-enums.assbin"),
            Some("assbin"),
        )
        .expect("import native enum values outside the known constants");
    assert_eq!(scene.light(0).unwrap().light_type(), LightType::Unknown(7));
    let channel = scene.animation(0).unwrap().channel(0).unwrap();
    assert_eq!(channel.pre_state(), AnimBehaviour::Unknown(7));
    assert_eq!(channel.post_state(), AnimBehaviour::Unknown(8));
}
