#!/usr/bin/env python3
"""Release-consistency guard: every hand-maintained version reference agrees.

The stale-version class recurred at three consecutive releases (the #230
review's dl links -> v0.10.0's four stale `[Unreleased]` baselines, repaired
in the cut -> the post-v0.10.0 hygiene commit's README/link findings -> the
v0.11.0 release review finding the same three sites again). Each fix was by
hand; this recomputes the invariant set instead:

  1. `[workspace.package].version` equals every intra-workspace
     `workspace.dependencies` pin (the `version = "..."` on path deps).
  2. Every published crate CHANGELOG's `[Unreleased]` compare-link baseline
     is `v<workspace version>` (catgraph-testutil's perpetual `[Unreleased]`
     has no link line and is naturally exempt).
  3. catgraph-magnitude/README.md's status line names the workspace version.

Run from the repo root, or pass the root as argv[1].
"""

import pathlib
import re
import sys


def main() -> int:
    root = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else pathlib.Path(".")
    errors = []

    workspace = (root / "Cargo.toml").read_text(encoding="utf-8")
    version_match = re.search(
        r'^\[workspace\.package\]\nversion = "([^"]+)"', workspace, re.M
    )
    if not version_match:
        print("version-refs guard: cannot find [workspace.package] version")
        return 1
    version = version_match.group(1)

    for pin in re.finditer(
        r'^catgraph[a-z-]*\s*=\s*\{[^}]*\bversion = "([^"]+)"', workspace, re.M
    ):
        if pin.group(1) != version:
            errors.append(
                f"Cargo.toml: intra-workspace pin {pin.group(1)!r} != {version!r}"
            )

    for changelog in sorted(root.glob("catgraph*/CHANGELOG.md")):
        text = changelog.read_text(encoding="utf-8")
        link = re.search(r"^\[Unreleased\]: .*/compare/(v[0-9.]+)\.\.\.HEAD", text, re.M)
        if link and link.group(1) != f"v{version}":
            errors.append(
                f"{changelog}: [Unreleased] baseline {link.group(1)} != v{version}"
            )

    magnitude_readme = (root / "catgraph-magnitude/README.md").read_text(
        encoding="utf-8"
    )
    if f"workspace version `{version}`" not in magnitude_readme:
        errors.append(
            f"catgraph-magnitude/README.md: status line does not name {version!r}"
        )

    if errors:
        print("version-refs guard: hand-maintained version references disagree:")
        print("\n".join(errors))
        return 1
    print(f"version-refs guard ok: all references agree on {version}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
