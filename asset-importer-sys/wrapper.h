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

#ifdef __cplusplus
extern "C" {
#endif

// Import a file with optional custom IO, properties and a progress callback.
// Returns a deep-copied aiScene which can be freed with aiFreeScene.
const struct aiScene* aiImportFileExWithProgressRust(
    const char* path,
    unsigned int flags,
    const struct aiFileIO* file_io, // nullable
    const struct aiRustProperty* props,
    size_t props_count,
    aiRustProgressCallback progress_cb, // nullable
    void* progress_user // nullable
);

// Import from memory with properties + progress callback support.
const struct aiScene* aiImportFileFromMemoryWithProgressRust(
    const char* data,
    unsigned int length,
    unsigned int flags,
    const char* hint, // nullable
    const struct aiRustProperty* props,
    size_t props_count,
    aiRustProgressCallback progress_cb, // nullable
    void* progress_user // nullable
);

// Get the last error message produced by the Rust C++ bridge (thread-local).
const char* aiGetLastErrorStringRust(void);

#ifdef __cplusplus
}
#endif
