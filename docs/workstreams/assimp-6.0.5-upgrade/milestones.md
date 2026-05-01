# Milestones

## M1: Source And Planning Baseline

- Review repository structure, build modes, bridge boundary, and safe API ownership model.
- Create this workstream.
- Capture initial risks and validation plan.

Exit criteria:

- Workstream docs are committed or staged with the upgrade branch.

## M2: Vendored Source Upgrade

- [x] Fetch Assimp `v6.0.5`.
- [x] Move the submodule pointer to the release tag.
- [x] Confirm the submodule worktree is clean.

Exit criteria:

- `git submodule status --recursive` shows the new 6.0.5 commit.

## M3: Binding Refresh

- [x] Regenerate `asset-importer-sys/src/bindings_pregenerated.rs`.
- [x] Review generated diff for new/removed symbols; no checked-in binding diff was produced.
- [x] Update build/version checks and packaging metadata.

Exit criteria:

- `asset-importer-sys` compiles with vendored/generated bindings.

## M4: Safe API Compatibility

- [x] Review generated binding changes against the safe API.
- [x] Add or adjust wrappers only where the API is stable, ownership-safe, and useful; none were needed.
- [ ] Update tests for changed property keys, enum variants, or struct layouts.

Exit criteria:

- Safe crate compiles and existing tests pass with `build-assimp`.

## M5: Verification And Release Readiness

- [x] Run `cargo fmt`.
- [x] Prefer `cargo nextest run` for tests.
- [x] Run `cargo test` only for doc tests or scenarios not covered by nextest.
- [x] Record the opt-in prebuilt artifact limitation until release artifacts are rebuilt.

Exit criteria:

- Test results are documented in the final task summary.
- Any remaining prebuilt-release work is explicitly listed.
