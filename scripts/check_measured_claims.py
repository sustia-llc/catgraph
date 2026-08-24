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
     number, with nothing between them — not a space, not a closing bracket,
     not a unit. Digits, then the marker:

     ```
     **14 473**<!--m:assoc.triples--> composable triples
     ```

     The marker is an HTML comment, so it does not render in Markdown. rustdoc
     renders Markdown, so the same holds there; no marker currently lives in a
     doc comment that rustdoc processes, so that half is reasoned rather than
     measured.
  3. Every marker must resolve to an emitted fact, and the number touching it
     must equal that fact's value.

A marker whose key no test emits is an error, not a skip: that is exactly the
"the test was renamed and the prose still claims its number" case.

TO SHOW THE SYNTAX WITHOUT TRIPPING THE GUARD, put it in a fenced code block,
as just above — that is the only escape hatch. Fences behind `///`, `//!`, `#`
or `>` count, so it works in rustdoc and in scripts too. An UNTERMINATED fence
is an error: silently skipping every marker after a stray fence is the same
content-dependent hole this tool exists to close.

KNOWN LIMITS, all failing loud rather than silent:
  - Punctuation between number and marker — `(456)`, `456%`, `456th`, `**456**,`
    — reads as "no number touching it". Move the marker onto the digits.
  - Two distinct numbers separated by a single space before a marker merge into
    one, because that is indistinguishable from a grouped `12 456`.
  - Values are integers. A decimal is rejected rather than read as its tail.

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

# Thin, figure and hair spaces, added after review: a thin space silently
# truncated `14<U+2009>473` to `473`, loud only because the fact did not happen
# to equal the tail. Written as escapes so an edit cannot drop one invisibly.
SEPS += "   "

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
# A code fence, optionally behind a doc-comment or blockquote prefix so the
# escape hatch also exists in `///` / `//!` rustdoc, `#` scripts and `>` quotes.
# An earlier version matched only column-0 Markdown, which meant the documented
# escape hatch did not exist in `.rs` — one of the two suffixes this was written
# for.
FENCE = re.compile(r"^\s*(?:(?:///|//!|//|#+|>)\s*)?(```+|~~~+)")

PROSE_SUFFIXES = {".md", ".rs", ".txt", ".toml", ".yml", ".yaml", ".py", ".sh"}
SKIP_DIRS = {"target", ".git", "node_modules"}
# This guard's own fixtures cite deliberately-unemitted keys, so the CI walk must
# not see them. Scoped to the ONE path rather than the bare name `testdata`:
# a name-based skip would silently exempt any future `<crate>/testdata/*.md` from
# checking, which is the hole this tool exists to close. Naming it as an explicit
# root still works — the skip applies to child directories, not to the root
# itself — which is how check_measured_claims.test.sh reaches these fixtures.
SKIP_RELATIVE = ("scripts/testdata",)

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
        dirnames[:] = [
            d
            for d in dirnames
            if d not in SKIP_DIRS
            and not pathlib.Path(dirpath, d).as_posix().endswith(SKIP_RELATIVE)
        ]
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
        # A fence is closed only by the same character it opened with, and an
        # unterminated one is an ERROR rather than a silent skip of every marker
        # after it. A bare toggle here was a false green: one stray ``` line
        # disabled checking for the rest of the file, in silence — the same
        # content-dependent-skip class this tool exists to catch.
        fence_char = None
        for line_no, line in enumerate(text.splitlines(), 1):
            fence = FENCE.match(line)
            if fence:
                token = fence.group(1)[0]
                if fence_char is None:
                    fence_char, fence_line = token, line_no
                    continue
                if token == fence_char:
                    fence_char = None
                    continue
            if fence_char is not None:
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
        if fence_char is not None:
            try:
                shown = path.relative_to(root)
            except ValueError:
                shown = path
            errors.append(
                f"{shown}:{fence_line}: unterminated code fence — every marker "
                f"after this line was skipped without being checked"
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
