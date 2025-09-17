# Asset Importer Examples

This directory contains practical examples demonstrating how to use the asset-importer library.

## Running

```
cargo run -p asset-importer --example <name> -- <args>
```

Tips:
- Set `AI_EX_VERBOSE=1` to enable verbose Assimp logging.
- Set `AI_EX_STDERR=1` to log to stderr (default is stdout).
- Set `AI_EX_LOGFILE=/path/to/log.txt` to also log to a file.

Most examples accept a model path as the first argument. If missing, some examples fallback to
`examples/models/<fallback>` for convenience.

## Structure

- `common.rs` – shared helpers: logging setup, CLI helpers, importer defaults.
- Basics:
  - `simple_load.rs` – minimal loading and scene traversal
  - `format_info.rs` – supported import/export formats
  - `model_loading_demo.rs` – end‑to‑end walkthrough (longer)
- Materials & Textures:
  - `pbr_material_inspector.rs` – inspect PBR material fields
  - `advanced_material_properties.rs` – setting import properties
  - `material_property_dump.rs` – dump raw/typed material properties (new)
- Animations:
  - `morph_animation_inspector.rs` – mesh/morph channels overview
  - (future) `node_animation_inspector.rs` – key interpolation/behaviour
- Tools:
  - `convert_model.rs` – export to another format
  - `batch_process.rs` – batch import with post‑process presets
- Introspection:
  - `importer_discovery_demo.rs` – list available importers
  - `russimp_compatible_interface.rs` – russimp compatibility surface

## Test Models

The `models/` directory contains simple test models for experimentation:

- `box.obj` – cube without materials
- `cube_with_materials.obj/.mtl` – cube with multiple materials
- `cube.ply` – PLY format cube
- `triangle.stl` – simple STL triangle

## Common Usage Patterns

Load and inspect a model:
```
cargo run -p asset-importer --example simple_load --features build-assimp -- path/to/model.fbx
```

Convert formats:
```
cargo run -p asset-importer --example convert_model --features "build-assimp,export" -- model.dae model.obj
```

Check supported formats:
```
cargo run -p asset-importer --example format_info --features build-assimp
```

Process multiple files:
```
cargo run -p asset-importer --example batch_process --features build-assimp -- models/
```

## Conventions

- Consistent CLI: examples either require a `<model_file>` argument or fall back to
  `examples/models/<name>`. See `common::resolve_model_path`.
- Logging: controlled via environment variables (see above) and attached in `common::init_logging_from_env`.
- Post‑processing: `common::import_scene` applies a small default set; examples can pass extra flags.
- Output: keep output human‑readable and stable; prefer truncated previews for long arrays.
