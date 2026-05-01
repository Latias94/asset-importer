# Design Notes

## Build And Binding Strategy

The intended source of truth for non-system builds is the vendored Assimp checkout. `bindings_pregenerated.rs` should match that checkout exactly.

The current build script prefers pregenerated bindings even when `generate-bindings` is enabled for non-system builds. That keeps user builds deterministic, but it makes maintainership awkward because regenerating the checked-in binding requires bypassing the default path. A small maintainability improvement is to add an explicit maintainer-only environment override for forced bindgen generation.

Recommended environment variable:

```text
ASSET_IMPORTER_FORCE_GENERATE_BINDINGS=1
```

Expected behavior:

- Non-system builds keep using pregenerated bindings by default.
- System builds still require generated bindings from discovered system headers.
- Maintainers can force vendored bindgen output without deleting or moving checked-in files.

## Safe API Review Policy

Generated bindings are not a public design by themselves. For every new or changed Assimp symbol, classify it as one of:

- Raw-only: no immediate safe wrapper needed.
- Constants/properties: expose as typed or string constants when they are commonly configured by users.
- Scene data: add read-only wrappers only if the ownership model is clear and the data is scene-backed.
- Mutable/native lifecycle API: keep raw-only unless a safe ownership model is explicit.

## Ownership And Threading Contract

The safe crate treats scene-backed memory as immutable after import. This allows:

- `Scene` clones to keep the owning Assimp scene alive.
- View objects such as `Mesh`, `Material`, `Node`, and `Texture` to be `Send + Sync`.
- Zero-copy read APIs to avoid unnecessary allocations.

Do not add safe APIs that mutate scene-backed memory through shared views. If a new Assimp 6.0.5 API mutates, frees, or transfers ownership, wrap it behind an owned type or leave it raw-only.

## Prebuilt Release Strategy

The source upgrade and Rust crate code can be validated with either the default vendored source
build or the explicit source-build feature:

```text
cargo check -p asset-importer
cargo nextest run --workspace --no-default-features --features build-assimp
```

The `prebuilt` feature should not be considered fully green until matching 6.0.5 archives exist
for the current crate version and target matrix. It remains opt-in so release-asset lag does not
break normal source builds.

Prebuilt packages include `manifest.txt`; version validation should prefer this manifest because installed Assimp headers do not always ship `revision.h` or version macros.

## Documentation Strategy

User-facing documentation should state the bundled Assimp version and the source-build default.
Internal workstream docs should also record temporary mismatches, for example "source upgraded but
prebuilt artifacts pending".
