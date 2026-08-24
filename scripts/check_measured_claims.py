#!/usr/bin/env python3
"""Prose-consistency guard: every cited measured figure equals what the tests measured.

The audit sweep's dominant defect class is not wrong code, it is prose about
code that no gate can see. On #351 (G2-T7), fifteen review findings across four
rounds were *all* invisible to `cargo test`, clippy, fmt, rustdoc and the three
existing guards, while the production logic went unchanged after the first
commit — 0 non-comment lines added or deleted across the whole review arc. The
recurring sub-class is a measured figure restated in four or five places (test
failure message, test docstring, CHANGELOG, commit message) with nothing
checking that the restatements still match, or still describe the same quantity:

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
  2. Prose cites a fact by placing an HTML comment immediately after the
     number:  **14 473** composable triples <!--m:assoc.triples-->
     The marker is invisible in rendered Markdown and in rustdoc, so it costs
     the reader nothing. Digit-group separators (space, thin space, nbsp,
     underscore, comma) are ignored when comparing.
  3. Every marker must resolve to an emitted fact, and the number in front of
     it must equal that fact's value.

A marker whose key no test emits is an error, not a skip: that is exactly the
"the test was renamed and the prose still claims its number" case.

WHY IT TAKES A FILE RATHER THAN RUNNING CARGO

Running the suite inside the guard would let a build failure or an empty run
read as a pass. `mutants-knowledge.md` §1.3 records the same trap in
cargo-mutants, where exit 0 can mean "nothing was tested". So this consumes a
captured `cargo test -- --nocapture` log and FAILS when the log yields no facts
at all. Capture and check are separate commands, per the repo's gate rules.

USAGE

  cargo test --workspace -- --nocapture > /tmp/t.log
  python3 scripts/check_measured_claims.py /tmp/t.log

Prose roots default to the tree this script lives in; pass more paths to
override. Defaults are `__file__`-relative for the reason recorded in
check_version_refs.py: a cwd default silently validates whatever tree it was
invoked from.
"""

import pathlib
import re
import sys

FACT = re.compile(r"^\s*MEASURED\s+([A-Za-z0-9_.]+)\s*=\s*(-?[\d_,\s  ]*\d)\s*$")
MARKER = re.compile(r"<!--\s*m:([A-Za-z0-9_.]+)\s*-->")
# The cited number, immediately before the marker: optional bold/backticks, then digits
# with any of the usual group separators.
CITED = re.compile(r"(-?\d[\d_,\s  ]*)[`*_\s]*$")

SEPARATORS = str.maketrans("", "", " _,   ")
PROSE_SUFFIXES = (".md", ".rs")


def normalize(text):
    """Strip digit-group separators so '14 473', '14_473' and '14473' compare equal."""
    return text.translate(SEPARATORS)


def collect_facts(log_path):
    facts = {}
    conflicts = []
    for line in log_path.read_text(encoding="utf-8", errors="replace").splitlines():
        match = FACT.match(line)
        if not match:
            continue
        key, raw = match.group(1), normalize(match.group(2))
        if key in facts and facts[key] != raw:
            conflicts.append(f"{key}: emitted as {facts[key]!r} and {raw!r}")
        facts[key] = raw
    return facts, conflicts


def check_prose(root, facts):
    errors = []
    cited = set()
    for path in sorted(root.rglob("*")):
        if path.suffix not in PROSE_SUFFIXES or not path.is_file():
            continue
        if "target" in path.parts or ".git" in path.parts:
            continue
        try:
            text = path.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            continue
        if "<!--m:" not in text and "<!-- m:" not in text:
            continue
        for line_no, line in enumerate(text.splitlines(), 1):
            for marker in MARKER.finditer(line):
                key = marker.group(1)
                cited.add(key)
                where = f"{path.relative_to(root)}:{line_no}"
                if key not in facts:
                    errors.append(
                        f"{where}: cites {key!r}, which no test emitted. "
                        f"Either the emitting test is gone/renamed, or the run was filtered."
                    )
                    continue
                before = CITED.search(line[: marker.start()])
                if not before:
                    errors.append(f"{where}: marker {key!r} has no number in front of it")
                    continue
                claimed = normalize(before.group(1))
                if claimed != facts[key]:
                    errors.append(
                        f"{where}: cites {key!r} as {claimed} — measured {facts[key]}"
                    )
    return errors, cited


def main() -> int:
    if len(sys.argv) < 2:
        print(__doc__.strip().splitlines()[0])
        print("usage: check_measured_claims.py <cargo-test-log> [prose-root ...]")
        return 1

    log_path = pathlib.Path(sys.argv[1])
    if not log_path.is_file():
        print(f"measured-claims guard: no such log {log_path}")
        return 1

    default_root = pathlib.Path(__file__).resolve().parent.parent
    roots = [pathlib.Path(p) for p in sys.argv[2:]] or [default_root]

    facts, conflicts = collect_facts(log_path)
    if not facts:
        print(
            f"measured-claims guard: {log_path} contains no 'MEASURED key = value' lines. "
            "A run that measured nothing cannot verify anything — re-capture with "
            "`-- --nocapture` and without a test filter."
        )
        return 1

    errors = list(conflicts)
    all_cited = set()
    for root in roots:
        root_errors, cited = check_prose(root, facts)
        errors.extend(root_errors)
        all_cited |= cited

    for error in errors:
        print(f"measured-claims guard: {error}")
    if errors:
        return 1

    uncited = len(facts) - len(all_cited & set(facts))
    print(
        f"measured-claims guard ok: {len(all_cited)} citation(s) match "
        f"{len(facts)} measured fact(s) ({uncited} emitted but not cited)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
