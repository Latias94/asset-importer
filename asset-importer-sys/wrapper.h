/*
 * Wrapper header for Assimp C API bindings
 *
 * This file includes the core Assimp headers for bindgen to generate
 * Rust bindings.
 */

#pragma once

#include <stdbool.h>

// Core Assimp headers for complete API coverage
#include <assimp/cimport.h>
#include <assimp/scene.h>
#include <assimp/postprocess.h>
#include <assimp/types.h>
#include <assimp/matrix4x4.h>
#include <assimp/cexport.h>
#include <assimp/cfileio.h>
#include <assimp/material.h>
#include <assimp/anim.h>
#include <assimp/mesh.h>
#include <assimp/light.h>
#include <assimp/camera.h>
#include <assimp/texture.h>
#include <assimp/metadata.h>
#include <assimp/version.h>

// === Rust bridging types for progress + properties ===

// A simple property kind to avoid C unions in the public header
typedef enum aiRustPropertyKind {
    aiRustPropertyKind_Integer = 0,
    aiRustPropertyKind_Float = 1,
    aiRustPropertyKind_String = 2,
    aiRustPropertyKind_Matrix4x4 = 3,
    aiRustPropertyKind_Boolean = 4
} aiRustPropertyKind;

// A property descriptor used to pass Importer properties from Rust to C++
typedef struct aiRustProperty {
    const char* name;                 // property key name
    aiRustPropertyKind kind;          // active kind below
    int          int_value;           // also used for bool (0/1)
    float        float_value;
    const char*  string_value;        // UTF-8, null-terminated
    void*        matrix_value;        // pointer to aiMatrix4x4, row-major, as in Assimp
} aiRustProperty;

// Progress callback signature used by the bridge. Return false to cancel.
typedef bool (*aiRustProgressCallback)(float percentage, const char* message, void* user);

// Owns the C++ Importer and its scene. Access is read-only except for exclusive post-processing.
typedef struct aiRustScene aiRustScene;

#ifdef __cplusplus
extern "C" {
#endif

// Import a file with optional custom IO, properties and a progress callback.
// Release the returned owner with aiReleaseSceneRust.
// The custom IO table and its user data must outlive the owner. Progress state is
// only used during this call and is detached before returning.
struct aiRustScene* aiImportFileRust(
    const char* path,
    unsigned int flags,
    const struct aiFileIO* file_io, // nullable; must outlive the returned owner
    const struct aiRustProperty* props,
    size_t props_count,
    aiRustProgressCallback progress_cb, // nullable
    void* progress_user // nullable
);

// Import from memory with custom IO for external resources, properties, and progress.
struct aiRustScene* aiImportFileFromMemoryRust(
    const char* data,
    unsigned int length,
    unsigned int flags,
    const char* hint, // nullable
    const struct aiFileIO* file_io, // nullable; must outlive the returned owner
    const struct aiRustProperty* props,
    size_t props_count,
    aiRustProgressCallback progress_cb, // nullable
    void* progress_user // nullable
);

// Borrow the scene while its owner is alive.
const struct aiScene* aiGetSceneRust(const struct aiRustScene* owner);

// Release the owner, including the scene. A null owner is allowed.
void aiReleaseSceneRust(struct aiRustScene* owner);

// Requires exclusive access to the owner and all scene data. Failure invalidates the scene,
// but the owner remains valid and must still be released with aiReleaseSceneRust.
bool aiApplyPostProcessingRust(struct aiRustScene* owner, unsigned int flags);

// Query the current scene's memory requirements without relying on C API private data.
bool aiGetMemoryRequirementsRust(const struct aiRustScene* owner, struct aiMemoryInfo* info);

// === Export bridging ===

// Export a scene to a file with optional custom IO and exporter properties.
// preprocessing must be zero; prepare the scene with its importer before exporting.
// Returns aiReturn_SUCCESS on success.
enum aiReturn aiExportSceneExWithPropertiesRust(
    const struct aiScene* scene,
    const char* format_id,
    const char* path,
    const struct aiFileIO* file_io, // nullable
    unsigned int preprocessing,
    const struct aiRustProperty* props,
    size_t props_count
);

// Export a scene to a blob with exporter properties.
// preprocessing must be zero; prepare the scene with its importer before exporting.
// Returns an owned blob chain; free with aiReleaseExportBlob.
const struct aiExportDataBlob* aiExportSceneToBlobWithPropertiesRust(
    const struct aiScene* scene,
    const char* format_id,
    unsigned int preprocessing,
    const struct aiRustProperty* props,
    size_t props_count
);

// Get the last error message produced by the Rust C++ bridge (thread-local).
const char* aiGetLastErrorStringRust(void);

#ifdef __cplusplus
}
#endif
