#!/usr/bin/env python3
"""Prose-consistency guard: every cited measured figure equals what the tests measured.

The audit sweep's dominant defect class is not wrong code, it is prose about
code that no gate can see. On #351, twenty-one review findings across four
rounds were *all* invisible to `cargo test`, clippy, fmt, rustdoc and the three
existing guards, while the production logic went unchanged after the first
commit — 0 non-comment lines added and 0 deleted across the whole review arc.
The recurring sub-class is a measured figure restated in several places with
nothing checking that the restatements still match, or still describe the same
quantity:

  - #350: "three terms moved" was the net change in DISTINCT-FORM count while
    eleven terms had moved; the follow-up "eight already-present forms" was
    11 - 3 and matched no reading of the data.
  - #351: a falsification record named one perturbation and reported a
    different, broader one's redden counts.
  - #351: "456 of 456 ... 320 at the outer composition alone" against a
    measured 120 — two reviewers, two predicates, one undifferentiated word.

This recomputes the invariant instead of asking the next writer to be careful.

CONTRACT

  1. A test emits a fact on its own line:  MEASURED <key> = <value>
     `key` is [A-Za-z0-9_.]+ and should name the quantity, not the test.
     Values are integers; digit-group separators are allowed on both sides.
     A test-harness prefix before MEASURED is tolerated (`--test-threads=1`
     prepends `test <name> ... ` to the first line a test writes).
  2. Prose cites a fact by placing an HTML comment IMMEDIATELY after the
     number, with no words in between:
         **14 473**<!--m:assoc.triples--> composable triples
     The marker is invisible in rendered Markdown and in rustdoc.
  3. Every marker must resolve to an emitted fact, and the number touching it
     must equal that fact's value.

A marker whose key no test emits is an error, not a skip: that is exactly the
"the test was renamed and the prose still claims its number" case.

TO SHOW THE SYNTAX WITHOUT TRIPPING THE GUARD, put it in a fenced code block —
fenced blocks are skipped, and that is the only escape hatch.

WHY IT TAKES A FILE RATHER THAN RUNNING CARGO

Running the suite inside the guard would let a build failure or an empty run
read as a pass. This consumes a captured `cargo test -- --nocapture` log and
FAILS when the log yields no facts at all, and again when the prose yields no
citations — a guard that checked nothing must not report success. Capture and
check are separate commands, per the repo's gate rules.

USAGE

  cargo test --workspace -- --nocapture > /tmp/t.log
  python3 scripts/check_measured_claims.py /tmp/t.log

Prose roots default to the repository root (the parent of `scripts/`); pass
paths to override. Defaults are `__file__`-relative for the reason recorded in
check_version_refs.py: a cwd default silently validates whatever tree it was
invoked from.
"""

import os
import pathlib
import re
import sys

SEPS = "   ,_"

# `MEASURED key = value`, tolerating a test-harness prefix but requiring the
# value to end the line.
FACT = re.compile(
    r"(?:^|\s)MEASURED\s+([A-Za-z0-9_.]+)\s*=\s*(-?\d[\d" + SEPS + r"]*)\s*$"
)
MARKER = re.compile(r"<!--\s*m:([A-Za-z0-9_.]+)\s*-->")
# The cited number, touching the marker. Grouped form (single-character
# separators only) is tried first so "14 473" is one number, while "#350, 456"
# — comma AND space, two characters — does not merge. The lookbehind rejects a
# decimal tail, so "0.456" fails loudly rather than reading as 456.
CITED = re.compile(
    r"(?<![\d.])(-?(?:\d{1,3}(?:[" + SEPS + r"]\d{3})+|\d+))[`*_\s]*$"
)
FENCE = re.compile(r"^\s*(```|~~~)")

PROSE_SUFFIXES = {".md", ".rs", ".txt", ".toml", ".yml", ".yaml", ".py", ".sh"}
# `testdata` holds this guard's own fixtures, which cite deliberately-unemitted
# keys. It is skipped during a walk, but naming it as an explicit root still
# works (the skip applies to child directories, not to the root itself), which
# is how check_measured_claims.test.sh reaches them.
SKIP_DIRS = {"target", ".git", "node_modules", "testdata"}

STRIP = str.maketrans("", "", SEPS)


def normalize(text):
    """Strip digit-group separators so '14 473', '14_473' and '14473' compare equal."""
    return text.translate(STRIP)


def collect_facts(log_path):
    facts, conflicts = {}, []
    for line in log_path.read_text(encoding="utf-8", errors="replace").splitlines():
        match = FACT.search(line)
        if not match:
            continue
        key, raw = match.group(1), normalize(match.group(2))
        if key in facts and facts[key] != raw:
            conflicts.append(f"{key}: emitted as {facts[key]!r} and {raw!r}")
        facts[key] = raw
    return facts, conflicts


def prose_files(root):
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = [d for d in dirnames if d not in SKIP_DIRS]
        for name in sorted(filenames):
            if pathlib.PurePath(name).suffix.lower() in PROSE_SUFFIXES:
                yield pathlib.Path(dirpath) / name


def check_prose(root, facts):
    errors, cited, sites = [], set(), 0
    for path in prose_files(root):
        try:
            text = path.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            continue
        if "<!--" not in text:
            continue
        in_fence = False
        for line_no, line in enumerate(text.splitlines(), 1):
            if FENCE.match(line):
                in_fence = not in_fence
                continue
            if in_fence:
                continue
            for marker in MARKER.finditer(line):
                key = marker.group(1)
                cited.add(key)
                sites += 1
                try:
                    where = f"{path.relative_to(root)}:{line_no}"
                except ValueError:
                    where = f"{path}:{line_no}"
                if key not in facts:
                    errors.append(
                        f"{where}: cites {key!r}, which no test emitted. Either the "
                        f"emitting test is gone/renamed, or the run was filtered."
                    )
                    continue
                before = CITED.search(line[: marker.start()])
                if not before:
                    errors.append(
                        f"{where}: marker {key!r} has no number touching it "
                        f"(the marker goes immediately after the digits)"
                    )
                    continue
                claimed = normalize(before.group(1))
                if claimed != facts[key]:
                    errors.append(
                        f"{where}: cites {key!r} as {claimed} — measured {facts[key]}"
                    )
    return errors, cited, sites


def main() -> int:
    if len(sys.argv) < 2:
        print("usage: check_measured_claims.py <cargo-test-log> [prose-root ...]")
        return 1

    log_path = pathlib.Path(sys.argv[1])
    if not log_path.is_file():
        print(f"measured-claims guard: no such log {log_path}")
        return 1

    repo_root = pathlib.Path(__file__).resolve().parent.parent
    roots = [pathlib.Path(p) for p in sys.argv[2:]] or [repo_root]

    facts, conflicts = collect_facts(log_path)
    if not facts:
        print(
            f"measured-claims guard: {log_path} contains no 'MEASURED key = value' "
            "lines. A run that measured nothing cannot verify anything — re-capture "
            "with `-- --nocapture` and without a test filter."
        )
        return 1

    errors = list(conflicts)
    all_cited = set()
    all_sites = 0
    for root in roots:
        root_errors, cited, sites = check_prose(root, facts)
        errors.extend(root_errors)
        all_cited |= cited
        all_sites += sites

    if not all_cited:
        errors.append(
            "no prose cited any measured fact. Either every marker was deleted or the "
            "prose roots are wrong — a guard that checked nothing must not report "
            f"success. Roots scanned: {', '.join(str(r) for r in roots)}"
        )

    for error in errors:
        print(f"measured-claims guard: {error}")
    if errors:
        return 1

    print(
        f"measured-claims guard ok: {all_sites} citation site(s) across "
        f"{len(all_cited)} key(s) match {len(facts)} measured fact(s) "
        f"({len(facts) - len(all_cited)} emitted but uncited)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
