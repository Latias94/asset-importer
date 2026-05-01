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

- Short term: update all constants together during the 6.0.5 upgrade.
- Later: consider deriving package metadata from generated headers or a single checked-in metadata file.

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

- During binding diff review, add constants only for newly relevant 6.0.5 keys or keys already used by examples/tests.
- Keep tests that compare string constants to generated macro values.

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
