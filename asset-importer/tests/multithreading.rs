// Integration tests for multi-threading support in asset-importer
use asset_importer::{Importer, postprocess::PostProcessSteps};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

fn create_test_model(filename: &str) -> String {
    let test_obj_content = r#"
# Multi-threading test model
v -1.0 -1.0  1.0
v  1.0 -1.0  1.0
v  1.0  1.0  1.0
v -1.0  1.0  1.0

f 1 2 3 4
"#;

    std::fs::write(filename, test_obj_content).expect("Failed to write test file");
    filename.to_string()
}

// Test 1: Multiple threads processing the same scene
#[test]
fn test_shared_scene_processing() {
    println!("🧵 Test: Multiple threads processing shared scene data");

    let importer = Importer::new();
    let file_path = create_test_model("multithread_shared_test.obj");

    match importer.import_file(&file_path) {
        Ok(scene) => {
            // Wrap scene in Arc to share between threads
            let scene = Arc::new(scene);
            let mut handles = vec![];

            // Spawn multiple threads to process the same scene
            for thread_id in 0..4 {
                let scene_clone = Arc::clone(&scene);
                let handle = thread::spawn(move || {
                    println!("  🔧 Thread {}: Starting processing", thread_id);

                    // Each thread processes meshes
                    for (mesh_idx, mesh) in scene_clone.meshes().enumerate() {
                        println!(
                            "  🔧 Thread {}: Processing mesh {} - {}",
                            thread_id,
                            mesh_idx,
                            mesh.name()
                        );

                        let vertices = mesh.vertices();
                        let faces = mesh.faces();

                        // Simulate some work
                        thread::sleep(Duration::from_millis(10));

                        println!(
                            "  🔧 Thread {}: Mesh {} has {} vertices, {} faces",
                            thread_id,
                            mesh_idx,
                            vertices.len(),
                            faces.len()
                        );
                    }

                    // Process materials
                    for (mat_idx, material) in scene_clone.materials().enumerate() {
                        println!(
                            "  🔧 Thread {}: Material {} - {}",
                            thread_id,
                            mat_idx,
                            material.name()
                        );
                        thread::sleep(Duration::from_millis(5));
                    }

                    println!("  ✅ Thread {}: Completed processing", thread_id);
                    thread_id
                });

                handles.push(handle);
            }

            // Wait for all threads to complete
            for handle in handles {
                let thread_id = handle.join().expect("Thread panicked");
                println!("  ✅ Thread {} joined successfully", thread_id);
            }

            println!("✅ Test passed: Multiple threads successfully processed shared scene");
        }
        Err(e) => {
            panic!("Failed to load scene: {}", e);
        }
    }

    let _ = std::fs::remove_file(file_path);
}

// Test 2: Each thread loads and processes its own scene
#[test]
fn test_independent_scene_processing() {
    println!("🧵 Test: Independent scene processing per thread");

    let mut handles = vec![];

    // Create multiple test files and spawn threads
    for i in 0..3 {
        let file_content = format!(
            r#"
# Test model {}
v {} 0.0 0.0
v {} 1.0 0.0
v {} 0.5 1.0
f 1 2 3
"#,
            i,
            i as f32,
            i as f32 + 1.0,
            i as f32 + 0.5
        );

        let filename = format!("test_independent_model_{}.obj", i);
        std::fs::write(&filename, file_content).expect("Failed to write test file");

        let handle = thread::spawn(move || {
            println!("  🔧 Thread {}: Loading independent scene", i);

            let importer = Importer::new();
            match importer.import_file(&filename) {
                Ok(scene) => {
                    println!(
                        "  🔧 Thread {}: Scene loaded with {} meshes",
                        i,
                        scene.num_meshes()
                    );

                    // Process the scene
                    for mesh in scene.meshes() {
                        let vertices = mesh.vertices();
                        println!(
                            "  🔧 Thread {}: Mesh '{}' has {} vertices",
                            i,
                            mesh.name(),
                            vertices.len()
                        );

                        // Simulate processing time
                        thread::sleep(Duration::from_millis(20));
                    }

                    println!("  ✅ Thread {}: Completed independent processing", i);
                    true
                }
                Err(e) => {
                    println!("  ❌ Thread {}: Failed to load scene: {}", i, e);
                    false
                }
            }
        });

        handles.push((handle, format!("test_independent_model_{}.obj", i)));
    }

    // Wait for all threads and clean up
    let mut success_count = 0;
    for (handle, filename) in handles {
        if handle.join().expect("Thread panicked") {
            success_count += 1;
        }
        let _ = std::fs::remove_file(filename);
    }

    assert_eq!(success_count, 3, "All threads should succeed");
    println!(
        "✅ Test passed: {}/3 threads successfully processed independent scenes",
        success_count
    );
}

// Test 3: Producer-Consumer pattern with scene data
#[test]
fn test_producer_consumer() {
    println!("🧵 Test: Producer-Consumer pattern with scene data");

    use std::sync::mpsc;

    let importer = Importer::new();
    let file_path = create_test_model("producer_consumer_test.obj");

    match importer.import_file(&file_path) {
        Ok(scene) => {
            let scene = Arc::new(scene);
            let (tx, rx) = mpsc::channel();

            // Producer thread: extracts mesh data
            let scene_producer = Arc::clone(&scene);
            let producer_handle = thread::spawn(move || {
                println!("  📤 Producer: Starting to extract mesh data");

                for (idx, mesh) in scene_producer.meshes().enumerate() {
                    let vertices = mesh.vertices();
                    let vertex_count = vertices.len();

                    // Send mesh info to consumer
                    tx.send((idx, mesh.name().to_string(), vertex_count))
                        .expect("Failed to send mesh data");

                    println!("  📤 Producer: Sent mesh {} data", idx);
                    thread::sleep(Duration::from_millis(10));
                }

                println!("  ✅ Producer: Finished extracting data");
            });

            // Consumer thread: processes received data
            let consumer_handle = thread::spawn(move || {
                println!("  📥 Consumer: Starting to process mesh data");
                let mut processed_count = 0;

                while let Ok((idx, name, vertex_count)) = rx.recv() {
                    println!(
                        "  📥 Consumer: Processing mesh {} '{}' with {} vertices",
                        idx, name, vertex_count
                    );

                    // Simulate processing
                    thread::sleep(Duration::from_millis(15));
                    processed_count += 1;

                    println!("  📥 Consumer: Completed processing mesh {}", idx);
                }

                println!("  ✅ Consumer: Processed {} meshes", processed_count);
                processed_count
            });

            // Wait for both threads
            producer_handle.join().expect("Producer thread panicked");
            let processed_count = consumer_handle.join().expect("Consumer thread panicked");

            assert!(processed_count > 0, "Should process at least one mesh");
            println!(
                "✅ Test passed: Producer-Consumer processed {} meshes",
                processed_count
            );
        }
        Err(e) => {
            panic!("Failed to load scene for producer-consumer test: {}", e);
        }
    }

    let _ = std::fs::remove_file(file_path);
}

// Test 4: Compilation test for Send/Sync traits
#[test]
fn test_send_sync_compilation() {
    fn assert_send_sync<T: Send + Sync>() {}

    // Test all core types
    assert_send_sync::<asset_importer::Scene>();
    assert_send_sync::<asset_importer::node::Node>();
    assert_send_sync::<asset_importer::mesh::Mesh>();
    assert_send_sync::<asset_importer::mesh::Face>();
    assert_send_sync::<asset_importer::mesh::FaceIterator>();
    assert_send_sync::<asset_importer::mesh::AnimMesh>();
    assert_send_sync::<asset_importer::mesh::AnimMeshIterator>();
    assert_send_sync::<asset_importer::node::NodeIterator>();
    assert_send_sync::<asset_importer::node::MeshIndexIterator>();
    assert_send_sync::<asset_importer::Material>();
    assert_send_sync::<asset_importer::light::Light>();
    assert_send_sync::<asset_importer::camera::Camera>();
    assert_send_sync::<asset_importer::Bone>();
    assert_send_sync::<asset_importer::Animation>();
    assert_send_sync::<asset_importer::Texture>();

    println!("✅ Compilation test passed: All types implement Send + Sync");
}

#[test]
fn test_postprocess_rejects_scene_with_view_on_another_thread() {
    let scene = Importer::new()
        .read_from_memory(b"v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf 1 2 3 4\n")
        .with_memory_hint("obj")
        .with_post_process(PostProcessSteps::empty())
        .import()
        .expect("failed to import quad");
    let mesh = scene.mesh(0).expect("missing mesh");
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let (resume_tx, resume_rx) = std::sync::mpsc::channel();

    let reader = thread::spawn(move || {
        let vertices = mesh.vertices_raw();
        let expected = vertices.to_vec();
        ready_tx
            .send(())
            .expect("reader could not signal readiness");
        resume_rx.recv().expect("reader was not resumed");
        assert_eq!(vertices, expected.as_slice());
        assert_eq!(mesh.num_faces(), 1);
        assert_eq!(mesh.faces().next().expect("missing face").num_indices(), 4);
    });

    ready_rx.recv().expect("reader did not become ready");
    let result = scene.apply_postprocess(PostProcessSteps::TRIANGULATE);
    resume_tx.send(()).expect("could not resume reader");
    reader.join().expect("reader thread panicked");
    assert!(matches!(
        result,
        Err(asset_importer::Error::InvalidParameter { ref message })
            if message.contains("exclusive scene ownership")
    ));
}
