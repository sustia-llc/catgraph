#!/usr/bin/env python3
"""#409 guard: every public type and trait is named in its crate's canonical test.

Each published crate has one canonical integration test,
`<crate>/tests/canonical.rs`, whose `//!` header carries a `covers:` section and
a `not-covered:` section. Every `pub struct|enum|trait|type` declared under the
crate's `src` must be named, in backticks, in exactly one of the two.

Enumeration matches the ruling on #409:
`rg -n '^\\s*pub (struct|enum|trait|type) ' <crate>/src --glob '!**/tests/**'`,
with the declared identifier taken as the token after the keyword.

Header parsing: the `covers:` section runs from the `//!` line ending in
`covers:` to the `not-covered:` line, and the `not-covered:` section from there
to the end of the `//!` block. Names are the backtick-quoted tokens in each.

A crate with no `tests/canonical.rs` is reported and skipped, so the rows of
G18 can land one at a time. Run from anywhere: pass the repo root as argv[1],
default `.`.
"""

import pathlib
import re
import sys

DECLARATION = re.compile(r"^\s*pub (?:struct|enum|trait|type) ([A-Za-z_][A-Za-z0-9_]*)")
BACKTICKED = re.compile(r"`([A-Za-z_][A-Za-z0-9_]*)`")
COVERS = re.compile(r"^//!.*\bcovers:\s*$")
NOT_COVERED = re.compile(r"^//!.*\bnot-covered:\s*$")


def declared_items(src: pathlib.Path) -> dict[str, str]:
    """Every declared public type or trait, mapped to the `file:line` it is at."""
    items: dict[str, str] = {}
    for path in sorted(src.rglob("*.rs")):
        if "tests" in path.parts[len(src.parts) :]:
            continue
        for lineno, line in enumerate(
            path.read_text(encoding="utf-8").splitlines(), start=1
        ):
            match = DECLARATION.match(line)
            if match:
                items.setdefault(match.group(1), f"{path}:{lineno}")
    return items


def header_sections(canonical: pathlib.Path) -> tuple[set[str], set[str]]:
    """The `covers:` and `not-covered:` name sets from the file's `//!` header."""
    covers: set[str] = set()
    not_covered: set[str] = set()
    target: set[str] | None = None
    for line in canonical.read_text(encoding="utf-8").splitlines():
        if not line.startswith("//!"):
            if line.strip() == "":
                continue
            break
        if COVERS.match(line):
            target = covers
            continue
        if NOT_COVERED.match(line):
            target = not_covered
            continue
        if target is not None:
            target.update(BACKTICKED.findall(line))
    return covers, not_covered


def check(crate: pathlib.Path) -> list[str]:
    canonical = crate / "tests" / "canonical.rs"
    if not canonical.is_file():
        print(f"  {crate.name}: no tests/canonical.rs yet, skipped")
        return []

    items = declared_items(crate / "src")
    covers, not_covered = header_sections(canonical)
    named = covers | not_covered

    problems = []
    for name, locus in sorted(items.items()):
        if name not in named:
            problems.append(f"{canonical}: `{name}` ({locus}) is in neither list")
    for name in sorted(covers & not_covered):
        problems.append(f"{canonical}: `{name}` is in both lists")
    for name in sorted(named - set(items)):
        problems.append(f"{canonical}: `{name}` is listed but not declared in src")
    if not problems:
        print(
            f"  {crate.name}: {len(items)} declared, "
            f"{len(covers)} covered, {len(not_covered)} not-covered"
        )
    return problems


def main() -> int:
    root = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else pathlib.Path(".")
    crates = sorted(
        path
        for path in root.glob("catgraph*")
        if (path / "src").is_dir() and path.name != "catgraph-testutil"
    )
    if not crates:
        print(f"no crates found under {root}")
        return 1

    print("canonical-test guard (#409):")
    problems: list[str] = []
    for crate in crates:
        problems.extend(check(crate))
    if problems:
        print("every public type and trait must be in `covers:` or `not-covered:`:")
        print("\n".join(problems))
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
