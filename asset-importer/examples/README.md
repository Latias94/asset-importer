# Asset Importer Examples

This directory contains practical examples demonstrating how to use the asset-importer library.

## Examples

### Basic Usage

#### `simple_load.rs`
Demonstrates basic model loading and inspection.
```bash
cargo run --example simple_load --features build-assimp -- asset-importer/examples/models/box.obj
```

Shows how to:
- Load a 3D model file
- Inspect scene structure (meshes, materials, animations)
- Access vertex data and calculate bounding boxes
- Traverse the scene graph hierarchy

#### `format_info.rs`
Displays supported file formats and library information.
```bash
cargo run --example format_info --features build-assimp
```

Shows:
- Assimp version information
- All supported import/export formats
- Format categories (3D graphics, CAD, game engines, etc.)

### File Conversion

#### `convert_model.rs`
Converts between different 3D model formats.
```bash
cargo run --example convert_model --features "build-assimp,export" -- asset-importer/examples/models/box.obj asset-importer/examples/models/box_converted.ply
```

Features:
- Automatic format detection from file extensions
- Optimization during conversion
- Conversion statistics and tips

### Batch Processing

#### `batch_process.rs`
Processes multiple model files and generates statistics.
```bash
# Process all models in a directory
cargo run --example batch_process --features build-assimp -- asset-importer/examples/models/

# Process specific file types
cargo run --example batch_process --features build-assimp -- "*.obj"
```

Provides:
- Batch file processing
- Aggregate statistics (total vertices, faces, etc.)
- Format distribution analysis
- Error reporting for failed files

### Advanced Features

#### `model_loading_demo.rs` 🎯 **NEW: Complete 3D Rendering Demo**
A comprehensive 3D model viewer showcasing the complete integration of asset-importer with modern OpenGL rendering.
```bash
cargo run --example model_loading_demo --features build-assimp -- path/to/model.obj
```

**🌟 This is the flagship example demonstrating asset-importer as a complete Assimp binding!**

Features:
- ✅ **Complete Model Loading**: Supports all Assimp formats (OBJ, FBX, GLTF, DAE, 3DS, PLY, etc.)
- ✅ **Modern OpenGL Rendering**: VAO/VBO/EBO with shader programs
- ✅ **Mesh & Texture Support**: Multi-mesh models with diffuse and specular textures
- ✅ **Phong Lighting**: Ambient, diffuse, and specular lighting
- ✅ **FPS Camera**: WASD movement, mouse look, scroll zoom
- ✅ **Real-time Rendering**: Complete render loop with proper MVP matrices

Controls:
- **WASD**: Camera movement
- **Mouse**: Look around
- **Mouse wheel**: Zoom in/out
- **ESC**: Exit

This example follows the LearnOpenGL tutorial architecture and demonstrates the full power of asset-importer for real-world 3D applications.

#### `advanced_material_properties.rs`
Demonstrates the enhanced material system and PropertyStore API.
```bash
cargo run --example advanced_material_properties --features build-assimp -- path/to/model.fbx
```

Features:
- PropertyStore for advanced import configuration
- Comprehensive material property access (colors, textures, physical properties)
- Material flags and texture information analysis
- Enhanced scene metadata accessors
- Import property constants for common settings

## Test Models

The `models/` directory contains simple test models for experimentation:

- **`box.obj`** - Simple cube without materials
- **`cube_with_materials.obj/.mtl`** - Cube with multiple materials
- **`cube.ply`** - PLY format cube
- **`triangle.stl`** - Simple STL triangle

These models are copied from the official Assimp test suite and are suitable for:
- Testing basic functionality
- Learning the API
- Verifying format support
- Performance benchmarking

## Running Examples

### Prerequisites
Choose a build method by adding the appropriate feature:

```bash
# Using prebuilt binaries (fastest)
cargo run --example <name> --features prebuilt -- <args>

# Using system-installed assimp
cargo run --example <name> --features system -- <args>

# Building from source (recommended for development)
cargo run --example <name> --features build-assimp -- <args>
```

### Common Usage Patterns

#### Load and inspect a model:
```bash
cargo run --example simple_load --features build-assimp -- path/to/model.fbx
```

#### Convert formats:
```bash
cargo run --example convert_model --features "build-assimp,export" -- model.dae model.obj
```

#### Check what formats are supported:
```bash
cargo run --example format_info --features build-assimp
```

#### Process multiple files:
```bash
cargo run --example batch_process --features build-assimp -- models/
```

## Adding Your Own Models

You can test with your own 3D models by placing them in the `models/` directory or specifying full paths:

```bash
cargo run --example simple_load --features build-assimp -- /path/to/your/model.fbx
```

Supported formats include: OBJ, FBX, DAE, 3DS, BLEND, PLY, STL, glTF, and many more.

## Performance Notes

- The examples use basic post-processing options suitable for most use cases
- For production use, consider optimizing post-processing steps based on your needs
- Large models may take time to process; the batch processor shows progress

## Troubleshooting

If you encounter issues:

1. **File not found**: Check the file path and ensure the model exists
2. **Format not supported**: Run `format_info` to see supported formats
3. **Build errors**: Ensure you have the required dependencies for your chosen build method
4. **Memory issues**: Large models may require more memory; try simpler models first

## Contributing

Feel free to add more examples! Good candidates include:
- Animation processing
- Texture extraction
- Scene optimization
- Custom post-processing
- Integration with graphics libraries
