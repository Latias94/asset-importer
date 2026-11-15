"""Regenerate vendored Assimp bindings, or check that the shipped file is current."""

import argparse
import difflib
import json
import os
from pathlib import Path
import subprocess
import sys


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="Report drift without changing files.")
    args = parser.parse_args()
    if sys.platform not in ("linux", "darwin") or "windows" in os.environ.get("CARGO_BUILD_TARGET", ""):
        parser.error("Update the shared bindings on native Linux or macOS; CI checks MSVC generation separately.")
    root = Path(__file__).resolve().parents[1]
    metadata = json.loads(
        subprocess.check_output(
            ["cargo", "metadata", "--locked", "--no-deps", "--format-version=1"],
            cwd=root,
            text=True,
        )
    )
    package_id = next(
        package["id"]
        for package in metadata["packages"]
        if package["name"] == "asset-importer-sys"
    )
    environment = os.environ.copy()
    environment["ASSET_IMPORTER_FORCE_GENERATE_BINDINGS"] = "1"
    environment.pop("DOCS_RS", None)
    generated = None
    with subprocess.Popen(
        [
            "cargo", "check", "-p", "asset-importer-sys", "--locked",
            "--no-default-features", "--features", "build-assimp,generate-bindings",
            "--message-format=json",
        ],
        cwd=root,
        env=environment,
        stdout=subprocess.PIPE,
        text=True,
    ) as process:
        for line in process.stdout:
            message = json.loads(line)
            if message["reason"] == "compiler-message":
                rendered = message["message"].get("rendered")
                if rendered:
                    print(rendered, end="", file=sys.stderr)
            if message["reason"] == "build-script-executed" and message["package_id"] == package_id:
                generated = Path(message["out_dir"]) / "bindings.rs"
        if process.wait() != 0:
            return 1
    if generated is None:
        raise SystemExit("Cargo did not report the asset-importer-sys build output.")

    destination = root / "asset-importer-sys/src/bindings_pregenerated.rs"
    actual = generated.read_text(encoding="utf-8")
    expected = destination.read_text(encoding="utf-8")
    if actual == expected:
        print("Pregenerated bindings are current.")
        return 0
    if args.check:
        sys.stdout.writelines(
            difflib.unified_diff(
                expected.splitlines(keepends=True),
                actual.splitlines(keepends=True),
                fromfile=str(destination.relative_to(root)),
                tofile="regenerated bindings",
            )
        )
        print("Run python3 tools/update_bindings.py and review the changes.", file=sys.stderr)
        return 1
    destination.write_text(actual, encoding="utf-8")
    print(f"Updated {destination.relative_to(root)}.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
