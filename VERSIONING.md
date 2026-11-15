# Versioning and releases

`asset-importer-sys` and `asset-importer` use Semantic Versioning independently. The sys crate tracks native bindings, bridge contracts, and build support; the high-level crate tracks its safe Rust API. Before 1.0, incompatible public API changes require a minor version bump. Both crates are versioned 0.9.0 for the Assimp 6.0.6 upgrade.

## Version sources

- Each crate's `Cargo.toml` declares its package version. Keep the high-level crate's `asset-importer-sys` dependency compatible with the sys release and refresh `Cargo.lock`.
- `asset-importer-sys/assimp-version.txt` declares the bundled native version. The Assimp submodule, pregenerated bindings, source builds, and prebuilt archives must agree with it. System builds generate bindings from their installed headers instead.
- Each crate owns its `CHANGELOG.md`. Finalize the release section with its version and date, leaving an empty `Unreleased` section for subsequent changes.
- Unified releases use `v<version>` tags, such as `v0.9.0`. The prebuilt workflow also accepts `asset-importer-sys-v<version>` for an independent sys release. High-level-only tags use `asset-importer-v<version>` and do not require new native packages.

## Local validation

Run Cargo commands sequentially and reuse the workspace's `target` directory. Limit build jobs when other local projects are building. The following commands cover the default source provider, optional integrations, examples, documentation, and package contents:

```console
cargo fmt --all -- --check
cargo nextest run --workspace --locked
cargo nextest run --workspace --locked --features build-assimp,export,raw-sys,type-extensions,glam,mint,bytemuck
cargo clippy --workspace --all-targets --locked --features build-assimp,export,raw-sys,type-extensions,glam,mint,bytemuck,demo -- -D warnings
cargo test --workspace --doc --locked --features build-assimp,export,raw-sys,type-extensions,glam,mint,bytemuck
cargo +1.85.0 check --workspace --all-targets --locked --features build-assimp,export,raw-sys,type-extensions,glam,mint,bytemuck,demo
python3 tools/update_bindings.py --check
cargo package --workspace --locked
```

The binding regeneration check requires libclang; the minimum-version check uses the shipped bindings and needs no libclang. Run `python3 tools/update_bindings.py` to update the shipped file after an Assimp upgrade, review the diff, and rerun the check. Use current stable Cargo to package both unpublished workspace crates together. Do not combine the mutually exclusive `build-assimp`, `prebuilt`, and `system` features or use `--all-features`. GitHub CI covers Windows, Linux, macOS, static linking, and Windows CRT variants; wait for the checks on the intended release commit.

Workflows use maintained Action version tags and explicit runner labels: Ubuntu 24.04 LTS, Windows Server 2025, and macOS 15, including `macos-15-intel` for Intel release archives. Keep the native release builders on a reviewed OS baseline: moving a Linux builder to a newer LTS can raise the system library requirements of its archives. Review runner upgrades separately from Action tag updates.

Generate a native archive locally before testing the prebuilt provider. From the workspace root, set `ASSET_IMPORTER_PACKAGE_DIR` to an absolute directory under `target`, run the package binary, then keep that variable set for the matching prebuilt tests:

```console
cargo run -p asset-importer-sys --bin package --locked --features build-assimp,package,static-link
cargo nextest run --workspace --locked --features prebuilt,static-link,export,raw-sys,type-extensions,glam,mint,bytemuck
```

Omit `static-link` from both commands to exercise shared libraries. Unset `ASSET_IMPORTER_FORCE_BUILD` for prebuilt tests so the source provider cannot mask a packaging failure. The archive must contain `manifest.txt`, the matching Assimp libraries, installed headers including `include/assimp/revision.h`, and runtime DLLs for Windows shared builds.

## crates.io Trusted Publishing

Configure a GitHub trusted publisher in **Settings → Trusted Publishing** for **both** existing crates, `asset-importer-sys` and `asset-importer`:

| Field | Value |
| --- | --- |
| Repository owner | `Latias94` |
| Repository name | `asset-importer` |
| Workflow filename | `publish.yml` |
| Environment | `crates-io` |

The workflow filename is the basename, not `.github/workflows/publish.yml` or the display name. Create the matching `crates-io` environment in the GitHub repository settings. If configuring deployment branch/tag rules, allow the release tags and the default branch for manual recovery runs. Merge the workflow into the default branch before publishing a release so GitHub can deliver the release event.

The [Publish to crates.io workflow](.github/workflows/publish.yml) runs when a GitHub release is published. It checks that the tag matches the selected crate versions, the same commit passed main-branch CI, and native releases have all ten public archives. It checks out the validated commit, verifies the packages with `cargo publish --dry-run`, then uses `rust-lang/crates-io-auth-action@v1` and job-local `id-token: write` to obtain a temporary token. The token is passed only to the upload step and is revoked by the action when the job ends. No crates.io API token secret is required.

Unified `v<version>` releases use Cargo's workspace publishing to upload the sys crate before its dependent high-level crate and wait for registry indexing. Crate-specific tags publish only that crate. Upload uses `--no-verify` because the immediately preceding dry run already built the packaged sources before the 30-minute token was requested; it does not skip package verification.

For a partial release, manually dispatch this workflow with the **same published tag** and select the missing crate in `package`. For example, if the sys crate was uploaded but the high-level upload failed, select `asset-importer` to avoid uploading the immutable sys version again. A failed run can have already uploaded a crate; inspect crates.io before retrying. Keep the default `auto` selection for a fresh release.

Trusted Publishing requires the crate to exist already and the person configuring it to be an owner. See the [crates.io documentation](https://crates.io/docs/trusted-publishing) and the [official authentication action](https://github.com/rust-lang/crates-io-auth-action).

## Publication order

1. Review and commit the release changes, including both changelogs. Confirm the Assimp submodule is at the intended tag and account for any local submodule modifications before packaging. For 0.9.0 the expected commit is `a79ff6bbce54d3e01186a918dc6374ed55317426` (`v6.0.6`).
2. Run the validation above and inspect the package contents. Current stable Cargo can package and verify both unpublished crates with `cargo package --workspace --locked`. Packaging the high-level crate on its own still requires its new sys dependency to be available from crates.io.
3. Create and push the release tag at the reviewed commit, then dispatch `Build Prebuilt Binaries` with that tag. The workflow resolves the tag once and builds all native packages from that exact commit. Each matrix job stores its archive as an Actions artifact only after consuming it in offline prebuilt tests. The upload job waits for the whole matrix, verifies all 10 expected archive names, and creates or updates a draft GitHub release. It refuses to modify an already published release. Attach the prepared release notes before publishing. High-level-only releases do not run the native workflow.
4. Check every required prebuilt matrix job, including the tests against its packaged library. Release 0.9.0 requires Linux x86_64, macOS Intel and Apple Silicon, and Windows MSVC archives for both link kinds; Windows also requires both MD and MT CRT variants.
5. Publish the GitHub release with the prepared [0.9.0 release notes](docs/releases/v0.9.0.md) after all 10 archives have passed validation. Draft attachments are unavailable to anonymous downloads; make the validated archives public before publishing crates so their opt-in `prebuilt` feature works immediately. Publication does not rebuild or replace the validated attachments.
6. Publishing the GitHub release triggers `Publish to crates.io`. With both trusted publishers configured as above, wait for its package verification and ordered uploads to finish. For manual recovery, dispatch it with the same release tag and select only the missing crate. If using a local API token instead, publish the sys crate first and wait until crates.io can resolve it before publishing the high-level crate:

   ```console
   cargo publish -p asset-importer-sys --locked --dry-run
   cargo publish -p asset-importer-sys --locked
   cargo package -p asset-importer --locked
   cargo publish -p asset-importer --locked --dry-run
   cargo publish -p asset-importer --locked
   ```

7. Confirm the expected release archives remain attached and check both crates.io pages and docs.rs builds.

Source builds are the default and do not depend on GitHub release artifacts. The opt-in `prebuilt` feature requires artifacts for the exact sys crate version; never reuse archives from an older release, even when their Assimp version matches.
