/*
 * Wrapper header for Assimp C API bindings
 *
 * This file includes the core Assimp headers for bindgen to generate
 * Rust bindings.
 */

#pragma once

// Core Assimp headers for complete API coverage
#include <assimp/cimport.h>
#include <assimp/scene.h>
#include <assimp/postprocess.h>
#include <assimp/types.h>
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
