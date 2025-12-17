//! Custom IO: import from an in-memory file system (OBJ + MTL).

use std::{
    error::Error,
    sync::{Arc, Mutex},
};

use asset_importer::{Importer, io::MemoryFileSystem, postprocess::PostProcessSteps};

fn main() -> Result<(), Box<dyn Error>> {
    // Build a tiny OBJ that references a material library.
    // Assimp will request both files via our custom FileSystem.
    let obj = r#"
mtllib cube.mtl
o Triangle
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 0.0 1.0 0.0
vt 0.0 0.0
vt 1.0 0.0
vt 0.0 1.0
vn 0.0 0.0 1.0
usemtl Red
f 1/1/1 2/2/1 3/3/1
"#;

    let mtl = r#"
newmtl Red
Kd 1.0 0.0 0.0
Ka 0.0 0.0 0.0
Ks 0.0 0.0 0.0
d 1.0
"#;

    let mut fs = MemoryFileSystem::new();
    fs.add_file("cube.obj", obj.as_bytes().to_vec());
    fs.add_file("cube.mtl", mtl.as_bytes().to_vec());

    let fs = Arc::new(Mutex::new(fs));

    let scene = Importer::new()
        .read_file("cube.obj")
        .with_post_process(PostProcessSteps::TRIANGULATE)
        .with_file_system(fs)
        .import_file("cube.obj")?;

    println!("Imported from MemoryFileSystem.");
    println!(
        "Meshes: {}  Materials: {}  Textures: {}",
        scene.num_meshes(),
        scene.num_materials(),
        scene.num_textures()
    );

    for (i, mat) in scene.materials().enumerate() {
        println!("\n== Material[{}] ==", i);
        println!("name: {}", mat.name());
        if let Some(c) = mat.diffuse_color() {
            println!("diffuse: [{:.3}, {:.3}, {:.3}]", c.x, c.y, c.z);
        }
        let nprops = mat.properties().count();
        println!("properties: {}", nprops);
    }

    Ok(())
}
