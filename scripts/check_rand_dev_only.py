#!/usr/bin/env python3
"""#239 guard: `rand` must be dev-only in every workspace member.

The published randomness edge is `rand_core` (catgraph-applied's supply
contract); no member's non-dev dependency section may name `rand`. The wasm
CI lanes cannot catch a re-added featureless `rand` lib edge — it compiles
cleanly — so this guard pins the actual invariant at the manifest level.

Section-aware line scan (not a TOML parse): a violation is a `rand = ...` or
`rand.workspace = ...` key inside any `[...dependencies]`-like section other
than `[dev-dependencies]` / `[target.*.dev-dependencies]`. The root
manifest's `[workspace.dependencies]` declaration itself is exempt (it is a
version registry, not an edge). Run from anywhere: pass the repo root as
argv[1], default `.`.
"""

import pathlib
import re
import sys


def main() -> int:
    root = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else pathlib.Path(".")
    violations = []
    for manifest in sorted(root.glob("catgraph*/Cargo.toml")):
        section = ""
        for lineno, line in enumerate(
            manifest.read_text(encoding="utf-8").splitlines(), start=1
        ):
            stripped = line.strip()
            header = re.fullmatch(r"\[+([^]]+)\]+", stripped)
            if header:
                section = header.group(1)
                continue
            if re.match(r"rand\s*[=.]", stripped) and "dev-dependencies" not in section:
                violations.append(f"{manifest}:{lineno}: `rand` in [{section}]")
    if violations:
        print("rand must be dev-only in member manifests (#239):")
        print("\n".join(violations))
        return 1
    print("rand-dev-only guard ok: no member ships rand outside [dev-dependencies]")
    return 0


if __name__ == "__main__":
    sys.exit(main())
