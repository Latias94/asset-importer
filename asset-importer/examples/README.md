# Asset Importer Examples

This directory contains practical examples demonstrating how to use the asset-importer library.

## Running

```
cargo run -p asset-importer --example <name> -- <args>
```

Build modes (choose exactly one):

- Prebuilt (default):
  - `cargo run -p asset-importer --example 01_quickstart -- <model>`
- Build from source:
  - `cargo run -p asset-importer --example 01_quickstart --no-default-features --features build-assimp -- <model>`
- System Assimp (pkg-config/vcpkg + libclang/bindgen required):
  - `cargo run -p asset-importer --example 01_quickstart --no-default-features --features system -- <model>`

Tips:
- Set `AI_EX_VERBOSE=1` to enable verbose Assimp logging.
- Set `AI_EX_STDERR=1` to log to stderr (default is stdout).
- Set `AI_EX_LOGFILE=/path/to/log.txt` to also log to a file.

Most examples accept a model path as the first argument. If missing, some examples fallback to
`examples/models/<fallback>` for convenience.

## Structure

- `common/mod.rs` – shared helpers: logging setup, CLI helpers, importer defaults.

Examples (recommended order):

- Basics:
  - `01_quickstart.rs` – load a model and print a concise summary
  - `02_formats.rs` – list supported import/export formats
  - `03_scene.rs` – inspect scene graph + memory info
- Materials:
  - `04_materials.rs` – inspect materials and raw/typed properties
- Animations:
  - `05_animations.rs` – inspect animation channels and keys
- Export:
  - `06_convert.rs` – convert a model to another format (requires `export` feature)
- Zero-copy & performance:
  - `08_zero_copy_mesh.rs` – raw mesh buffers + allocation-free iterators
- Custom IO:
  - `09_custom_io_memory_fs.rs` – in-memory file system for embedded assets
- Multithreading:
  - `10_multithreading.rs` – share `Scene` behind `Arc` and process meshes in parallel
- Multithreading (practical):
  - `12_parallel_mesh_stats.rs` – parallel per-mesh stats + AABB via zero-copy vertex slices
- Math integration:
  - `07_mint_integration.rs` – mint interoperability (requires `mint`)
  - `11_glam_integration.rs` – glam interoperability (requires `glam`)
- Larger walkthrough:
  - `model_loading_demo.rs` – end-to-end walkthrough (requires `demo`)

## Test Models

The `models/` directory contains simple test models for experimentation:

- `box.obj` – cube without materials
- `cube_with_materials.obj/.mtl` – cube with multiple materials
- `cube.ply` – PLY format cube
- `triangle.stl` – simple STL triangle

## Common Usage Patterns

Load and inspect a model:
```
cargo run -p asset-importer --example 01_quickstart --no-default-features --features build-assimp -- path/to/model.fbx
```

Convert formats:
```
cargo run -p asset-importer --example 06_convert --no-default-features --features "build-assimp,export" -- model.dae model.obj
```

Check supported formats:
```
cargo run -p asset-importer --example 02_formats --no-default-features --features build-assimp
```

Test mint math library integration:
```
cargo run -p asset-importer --example 07_mint_integration --no-default-features --features "build-assimp,mint"
```

## Conventions

- Consistent CLI: examples either require a `<model_file>` argument or fall back to
  `examples/models/<name>`. See `common::resolve_model_path`.
- Logging: controlled via environment variables (see above) and attached in `common::init_logging_from_env`.
- Post‑processing: `common::import_scene` applies a small default set; examples can pass extra flags.
- Output: keep output human‑readable and stable; prefer truncated previews for long arrays.
