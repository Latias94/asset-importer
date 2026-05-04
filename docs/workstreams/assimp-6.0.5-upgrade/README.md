# Assimp 6.0.5 Upgrade Workstream

Created: 2026-05-01

## Goal

Upgrade the workspace from Assimp 6.0.4 to Assimp 6.0.5 while keeping the low-level FFI, C++ bridge, safe Rust API, tests, examples, and prebuilt packaging path coherent.

This workstream also tracks cleanup opportunities found during source review. The project is not Unity-related, so changes should favor correct architecture and removal of obsolete code over narrow patching when the broader fix is still reasonably scoped.

## Current Repository Shape

- `asset-importer-sys` owns Assimp discovery, vendored/source builds, prebuilt package download/extraction, bindgen integration, and the C++ bridge.
- `asset-importer-sys/assimp` was initially pinned to Assimp `v6.0.4`; this workstream moves it to `v6.0.5`.
- `asset-importer-sys/src/bindings_pregenerated.rs` is the default binding source for non-system builds.
- `asset-importer` provides the safe API around immutable `Scene` ownership and scene-backed view objects.
- The safe layer assumes imported scenes are read-only after import; `SharedPtr<T>` centralizes `Send + Sync` for approved scene-backed targets.
- The high-level crate now defaults to the same vendored source build as `asset-importer-sys`; `prebuilt` is an explicit opt-in path.

## Primary External Reference

- Assimp release: <https://github.com/assimp/assimp/releases/tag/v6.0.5>
- Tag: `v6.0.5`
- Commit shown by GitHub release page: `392a658`

## Upgrade Surface

- Submodule pointer: update `asset-importer-sys/assimp` to `v6.0.5`.
- Pregenerated bindings: regenerate from the updated headers and review diff.
- Build scripts: update vendored/prebuilt version expectations and packaging metadata from `6.0.4` to `6.0.5`.
- Feature policy: keep source builds as the default and use `prebuilt` only when matching release artifacts are requested explicitly.
- Safe API: inspect generated binding additions/removals, especially fields, enums, material keys, postprocess flags, export properties, and newly exposed C API helpers.
- Docs: replace user-facing `v6.0.4` references where they describe the bundled Assimp version.
- Tests: verify `build-assimp` first, then feature combinations that do not require unavailable release artifacts.

## 6.0.5 Interface Review

- Public C binding inputs (`cimport.h`, `scene.h`, `material.h`, `mesh.h`, `anim.h`, `cexport.h`,
  `version.h`, and related core headers) did not change between `v6.0.4` and `v6.0.5`.
- Forced bindgen regeneration produced no diff in `asset-importer-sys/src/bindings_pregenerated.rs`.
- The only `include/assimp` changes are C++ helper/header hardening in `LineSplitter.h`,
  `ParsingUtils.h`, `StreamReader.h`, plus the C++ `ai_epsilon` linkage tweak in `defs.h`.
- No required safe Rust wrapper additions were found for new C ABI symbols, struct fields, enums,
  postprocess flags, import properties, or export properties.
- Useful behavior-level additions were still made in the safe API for glTF material metadata fixed
  by 6.0.5:
  `Material::{texture_scale,normal_texture_scale,texture_strength,occlusion_texture,occlusion_texture_strength}`.
- Assimp imports glTF `occlusionTexture` as `aiTextureType_LIGHTMAP`, so the glTF occlusion
  convenience helpers intentionally use `TextureType::Lightmap` rather than
  `TextureType::AmbientOcclusion`.

## Known Risk Areas

- Assimp 6.0.5 is a bugfix release. No bindgen-visible C API additions were found, but the release
  includes importer/exporter hardening that can affect malformed or edge-case assets.
- The C++ bridge relies on `Assimp::Importer`, `Assimp::Exporter`, `Assimp::IOSystem`, `Assimp::ProgressHandler`, and `aiCopyScene`; any signature drift must be handled explicitly.
- The prebuilt path validates exact Assimp metadata, so all version constants must move together.
- Explicit `prebuilt` checks may fail locally until 6.0.5 release artifacts are rebuilt and published for crate version `0.7.0`; default builds are no longer blocked by those artifacts.

## Status

- [x] Initial repository and architecture review.
- [x] Workstream documentation created.
- [x] Update submodule to `v6.0.5`.
- [x] Regenerate and review bindings.
- [x] Update safe API for new/changed Assimp surface; no bindgen-visible safe API changes were required for this patch release.
- [x] Harden prebuilt version validation so stale 6.0.4 packages are rejected instead of warning-only header fallback.
- [x] Switch the high-level crate default away from prebuilt artifacts and back to vendored source builds.
- [x] Run focused formatting and tests.
- [x] Add behavior-level safe helpers for glTF texture scale/strength metadata fixed by 6.0.5.
- [ ] Decide whether this change is ready for a conventional commit.

## Verification Notes

- `cargo check -p asset-importer-sys --no-default-features --features build-assimp,generate-bindings` passed with `ASSET_IMPORTER_FORCE_GENERATE_BINDINGS=1`.
- Regenerated bindings produced no checked-in diff against `src/bindings_pregenerated.rs`.
- `cargo nextest run --workspace --no-default-features --features build-assimp` passed: 96 tests.
- `cargo check --workspace --no-default-features --features build-assimp,export,type-extensions,raw-sys,glam,mint,bytemuck,tokio` passed.
- `cargo nextest run -p asset-importer --no-default-features --features build-assimp,export` passed: 100 tests.
- `cargo test --doc -p asset-importer --no-default-features --features build-assimp` passed: 6 doctests.
- `cargo test --doc --workspace --no-default-features --features build-assimp` is not a useful gate while `asset-importer-sys` includes bindgen output; explicit workspace doctests force generated C-header comments to compile as Rust doctests.
- `cargo check -p asset-importer` passed after switching the high-level default to the vendored source build.
- `cargo nextest run --workspace` passed with the default vendored source build: 87 tests.
- `cargo check --workspace --features export,type-extensions,raw-sys,glam,mint,bytemuck,tokio` passed with the default vendored source build.
- `cargo test --doc -p asset-importer` passed with the default vendored source build: 6 doctests.
- `cargo check -p asset-importer --features prebuilt` failed as expected because the cached/package manifest reports `assimp_version=6.0.4` while the crate expects `6.0.5`.
- Follow-up hardening verification after the FFI audit:
  - `cargo check -p asset-importer` passed.
  - `cargo nextest run --workspace` passed: 90 tests.
  - `cargo check --workspace --features export,type-extensions,raw-sys,glam,mint,bytemuck,tokio` passed.
  - `cargo nextest run -p asset-importer --features export` passed: 94 tests.
  - `cargo test --doc -p asset-importer` passed: 6 doctests.
  - `cargo check -p asset-importer-sys --features build-assimp,package --bin package` passed.
  - `cargo check -p asset-importer --features prebuilt` still fails as expected on stale 6.0.4 prebuilt artifacts.
- Follow-up glTF texture helper verification:
  - `cargo nextest run -p asset-importer texture_scale_and_strength_read_texture_scoped_float_properties` passed.
  - `cargo check -p asset-importer` passed.
  - `cargo nextest run --workspace` passed: 91 tests.
  - `cargo check --workspace --features export,type-extensions,raw-sys,glam,mint,bytemuck,tokio` passed.
  - `cargo nextest run -p asset-importer --features export` passed: 95 tests.
  - `cargo test --doc -p asset-importer` passed: 6 doctests.
- Follow-up real glTF fixture verification:
  - `cargo nextest run -p asset-importer gltf_import_preserves` passed: 2 tests.
  - `cargo check -p asset-importer` passed.
  - `cargo nextest run --workspace` passed: 93 tests.
  - `cargo check --workspace --features export,type-extensions,raw-sys,glam,mint,bytemuck,tokio` passed.
  - `cargo nextest run -p asset-importer --features export` passed: 97 tests.
  - `cargo test --doc -p asset-importer` passed: 6 doctests.
