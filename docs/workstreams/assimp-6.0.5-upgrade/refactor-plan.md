# Fearless Refactor Plan

## Candidate 1: Maintainer-Friendly Binding Regeneration

Problem:

`generate-bindings` does not currently force bindgen for vendored builds when `bindings_pregenerated.rs` exists. This makes routine Assimp upgrades more manual than necessary.

Plan:

- [x] Add a build config flag derived from `ASSET_IMPORTER_FORCE_GENERATE_BINDINGS`.
- [x] Skip the pregenerated-copy fast path when this flag is set and `generate-bindings` is enabled.
- [x] Keep default end-user behavior unchanged.

Risk:

- Low. The env var is opt-in and only affects build-script behavior.

## Candidate 2: Centralize Assimp Version Constants

Problem:

The expected vendored Assimp version appears in multiple places:

- `asset-importer-sys/build_support/prebuilt.rs`
- `asset-importer-sys/bin/package/main.rs`
- README and changelogs

Plan:

- [x] Add `asset-importer-sys/assimp-version.txt` as the single source used by build/prebuilt
      validation and the package tool.
- [x] Add a build-script rerun trigger for the metadata file.
- Later: consider deriving user-facing docs/release notes from the same metadata during release
  automation, while keeping changelogs human-authored.

Risk:

- Medium if overdone now. Keep this upgrade focused unless duplication causes an immediate bug.

## Candidate 3: Logging API Cleanup

Problem:

`logging.rs` keeps deprecated custom stream APIs that now return errors due to FFI callback safety issues.

Plan:

- Leave public deprecated APIs for compatibility during the 0.x line unless the user approves a breaking cleanup.
- Update docs to steer users toward supported global verbose/error helpers.

Risk:

- Public API break if removed. Defer unless a broader breaking release is planned.

## Candidate 4: Property Key Coverage

Problem:

Assimp exposes many property macros. The safe API currently exposes a curated subset.

Plan:

- [x] Confirm that 6.0.5 did not add bindgen-visible material key macros in the core C headers.
- [x] Add safe constants/helpers for the behavior-level glTF keys that 6.0.5 fixes internally:
  `$tex.scale` and `$tex.strength`.
- [x] Add real in-memory glTF fixture tests for normal texture scale, occlusion texture strength,
  and CUBICSPLINE tangent preservation.
- [x] Correct the glTF occlusion helper to Assimp's actual `aiTextureType_LIGHTMAP` mapping.
- Keep tests focused on safe API behavior rather than mirroring every upstream material macro.

Risk:

- Low for additive constants; moderate if renaming existing public constants.

## Candidate 5: Build Feature Policy Cleanup

Problem:

The high-level crate defaulted to `prebuilt`, while `asset-importer-sys` defaulted to a vendored
source build. This made normal high-level builds depend on release artifacts and exposed users to
stale prebuilt packages during Assimp upgrades.

Plan:

- [x] Set `asset-importer` default features to `[]`.
- [x] Keep `prebuilt` as an explicit opt-in feature.
- [x] Update README, crate docs, examples, changelogs, and workstream notes so they describe the
      source-build default consistently.
- [x] Verify the default high-level build after the docs/code cleanup.

Risk:

- Medium. Changing default features is user-visible, but the new default is more deterministic and
  matches the sys crate behavior.

## Candidate 6: FFI Boundary Hardening

Problem:

The safe Rust API is already structured around checked pointer helpers, panic-catching callbacks,
and read-only scene views. A follow-up audit still found several common FFI hazards:

- C++ exceptions could cross `extern "C"` bridge functions.
- Assimp takes ownership of custom C++ `IOSystem` and `ProgressHandler` objects, but the bridge also
  held them in `unique_ptr`, creating double-free risk.
- Bridge error strings could be stale when read after pure C Assimp calls.
- A few file/texture helper paths used lossy casts or unchecked arithmetic.
- Material property views had one unchecked pointer wrapper path.

Plan:

- [x] Catch C++ exceptions at all bridge entrypoints and store them as bridge errors.
- [x] Transfer custom IO/progress handler ownership to Assimp exactly once.
- [x] Split pure Assimp error lookup from bridge-aware error lookup.
- [x] Replace file callback truncating casts and memory stream arithmetic with checked conversions.
- [x] Route material property views through checked `SharedPtr` construction.

Risk:

- Low to medium. The changes are defensive and should preserve behavior, but they touch native
  bridge ownership and therefore need source-build tests.

## Candidate 7: Vendored Build Cache Invalidation

Problem:

CI target caches can preserve `asset-importer-sys` CMake output across Assimp upgrades. If the Rust
crate is rebuilt against a cached 6.0.4 static library while the source tree and tests expect 6.0.5,
glTF 6.0.5 regression tests fail with old runtime behavior.

Plan:

- [x] Add an `OUT_DIR` stamp that records the expected Assimp version, link kind, CMake profile, and
  Assimp source path.
- [x] When the stamp differs, remove only known CMake output/install children under `OUT_DIR`
  (`build`, `include`, `lib`, `lib64`, `bin`, `share`) before rebuilding.
- [x] Clean local `asset-importer-sys` / `asset-importer` build outputs after restoring CI target
  cache in source-build jobs, so Cargo cannot skip the build script and run old native artifacts.
- [x] Add explicit runtime-version diagnostics to 6.0.5-specific glTF regression tests.
- [x] Verify the CI-style static source-build glTF regression command.

Risk:

- Low. Cleanup is scoped to Cargo's package-specific `OUT_DIR` and only to known CMake output
  directories.
