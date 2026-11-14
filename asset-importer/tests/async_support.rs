//! Scene-backed views retain their owners when moved through asynchronous tasks.

use asset_importer::Scene;

const TRIANGLE: &[u8] = b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scene_views_survive_suspension_and_owner_drop() {
    let scene = Scene::from_memory(TRIANGLE, Some("obj")).expect("import triangle");
    let mesh = scene.mesh(0).expect("triangle mesh");
    let root = scene.root_node().expect("root node");
    drop(scene);

    tokio::spawn(async move {
        let vertices = mesh.vertices_raw();
        let face = mesh.faces_iter().next().expect("triangle face");
        let indices = face.indices_raw();

        tokio::task::yield_now().await;

        assert_eq!(vertices.len(), 3);
        assert_eq!(vertices[1].x, 1.0);
        assert_eq!(indices, &[0, 1, 2]);
        assert!(root.num_children() > 0);
    })
    .await
    .expect("scene views remain usable in a spawned task");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn blocking_import_returns_scene_to_async_task() {
    let scene = tokio::task::spawn_blocking(|| Scene::from_memory(TRIANGLE, Some("obj")))
        .await
        .expect("import task completes")
        .expect("import triangle");

    tokio::task::yield_now().await;

    assert_eq!(scene.num_meshes(), 1);
    let mesh = scene.mesh(0).expect("triangle mesh");
    assert_eq!(mesh.num_vertices(), 3);
    assert_eq!(mesh.num_faces(), 1);
}
