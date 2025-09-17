#!/bin/bash

# Parse version from git tag for asset-importer workspace
# Supports formats:
# - asset-importer-sys-v0.1.0 (sys crate specific)
# - asset-importer-v0.1.0 (main crate specific)  
# - v0.1.0 (unified versioning)

set -euo pipefail

TAG="${1:-}"
if [ -z "$TAG" ]; then
    echo "Error: No tag provided"
    echo "Usage: $0 <tag>"
    exit 1
fi

echo "Parsing tag: $TAG"

# Parse version and determine target crate
if [[ "$TAG" =~ ^asset-importer-sys-v(.+)$ ]]; then
    VERSION="${BASH_REMATCH[1]}"
    CRATE="asset-importer-sys"
    echo "Detected sys crate release: $VERSION"
elif [[ "$TAG" =~ ^asset-importer-v(.+)$ ]]; then
    VERSION="${BASH_REMATCH[1]}"
    CRATE="asset-importer"
    echo "Detected main crate release: $VERSION"
elif [[ "$TAG" =~ ^v(.+)$ ]]; then
    VERSION="${BASH_REMATCH[1]}"
    CRATE="asset-importer-sys"  # Default to sys crate for unified versioning
    echo "Detected unified release: $VERSION (defaulting to sys crate)"
else
    echo "Error: Tag format not recognized"
    echo "Expected formats:"
    echo "  - asset-importer-sys-v0.1.0 (sys crate)"
    echo "  - asset-importer-v0.1.0 (main crate)"
    echo "  - v0.1.0 (unified versioning)"
    exit 1
fi

# Validate version format (basic semver check)
if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.-]+)?(\+[a-zA-Z0-9.-]+)?$ ]]; then
    echo "Error: Invalid version format: $VERSION"
    echo "Expected semantic version format: MAJOR.MINOR.PATCH[-PRERELEASE][+BUILD]"
    exit 1
fi

# Verify the crate exists
CRATE_DIR="$CRATE"
if [ ! -d "$CRATE_DIR" ]; then
    echo "Error: Crate directory not found: $CRATE_DIR"
    exit 1
fi

# Check if Cargo.toml exists
if [ ! -f "$CRATE_DIR/Cargo.toml" ]; then
    echo "Error: Cargo.toml not found in: $CRATE_DIR"
    exit 1
fi

# Optionally verify version matches Cargo.toml (if running in CI)
if [ "${VERIFY_CARGO_VERSION:-false}" = "true" ]; then
    CARGO_VERSION=$(grep '^version = ' "$CRATE_DIR/Cargo.toml" | head -1 | sed 's/version = "\(.*\)"/\1/')
    if [ "$VERSION" != "$CARGO_VERSION" ]; then
        echo "Warning: Tag version ($VERSION) doesn't match Cargo.toml version ($CARGO_VERSION)"
        echo "This might be expected if the tag was created before updating Cargo.toml"
    fi
fi

echo "✅ Version parsing successful"
echo "Version: $VERSION"
echo "Crate: $CRATE"

# Output for GitHub Actions
if [ "${GITHUB_OUTPUT:-}" ]; then
    echo "version=$VERSION" >> "$GITHUB_OUTPUT"
    echo "crate=$CRATE" >> "$GITHUB_OUTPUT"
    echo "crate_dir=$CRATE_DIR" >> "$GITHUB_OUTPUT"
fi
