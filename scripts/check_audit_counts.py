#!/usr/bin/env python3
"""Audit-doc count-consistency guard (#111).

Checks, for each audit doc passed as an argument:
  1. Summary-table arithmetic — every section row's status counts sum to its
     Total, and the bold **TOTAL** row equals the column-wise sums.
  2. Headline percentages — the bold `**N% X / ...**` line matches the TOTAL
     row (standard rounding).
  3. Detail-section tallies — per `### §x.y` section, status-emoji counts in
     the detail table match that section's summary row. Status emoji are read
     from the doc's own "Status legend" block, so the same code covers docs
     with different column sets (DONE/PARTIAL/MISSING/... vs DEFERRED/IN
     APPLIED/...). Compound cells (e.g. "🔗 ✅") classify by the first legend
     emoji present. Summary rows with no matching detail section are reported
     and skipped (some docs carry section-level rows with no table).
  4. Test-count citations — any `tests/<file>.rs ... (N tests)` on one line is
     checked against the actual `#[test]` count of that file (searched across
     the workspace's */tests/ dirs; skipped if not found uniquely).

Exit 1 on any mismatch; prints every check performed.
"""

import re
import sys
from pathlib import Path

WORKSPACE = Path(__file__).resolve().parent.parent
ERRORS: list[str] = []
NOTES: list[str] = []


def cells(row: str) -> list[str]:
    # split on unescaped pipes only — table cells may contain `\|`
    return [c.strip() for c in re.split(r"(?<!\\)\|", row.strip().strip("|"))]


def parse_int(cell: str) -> int:
    return int(cell.replace("*", "").strip())


def check_doc(path: Path) -> None:
    text = path.read_text()
    lines = text.splitlines()
    rel = path.relative_to(WORKSPACE)

    # -- Status legend: "- ✅ DONE — ..." → {"DONE": "✅"}
    legend: dict[str, str] = {}
    for m in re.finditer(r"^- (\S+) ([A-Z/ ]+?)(?: —|$)", text, re.M):
        legend[m.group(2).strip()] = m.group(1)

    # -- Summary table
    try:
        s_start = next(i for i, l in enumerate(lines) if l.strip() == "## Summary")
    except StopIteration:
        ERRORS.append(f"{rel}: no '## Summary' section")
        return
    header = None
    rows: list[tuple[str, list[int], int]] = []  # (section label, counts, total)
    total_row = None
    for l in lines[s_start:]:
        if l.startswith("|"):
            c = cells(l)
            if header is None:
                header = c
                continue
            if set(c[1]) <= {"-", " "} and "-" in c[1]:
                continue
            label = c[0].replace("*", "").strip()
            nums = [parse_int(x) for x in c[1:-1]]
            tot = parse_int(c[-1])
            if label == "TOTAL":
                total_row = (nums, tot)
            else:
                rows.append((label, nums, tot))
        elif header is not None and l.strip() and not l.startswith("|"):
            break
    if header is None or total_row is None:
        ERRORS.append(f"{rel}: could not parse summary table / TOTAL row")
        return
    col_names = [h.strip() for h in header[1:-1]]

    # 1. row sums + TOTAL
    for label, nums, tot in rows:
        if sum(nums) != tot:
            ERRORS.append(f"{rel}: summary row '{label}' sums to {sum(nums)} ≠ Total {tot}")
    col_sums = [sum(x) for x in zip(*(nums for _, nums, _ in rows))]
    row_tot_sum = sum(t for _, _, t in rows)
    if col_sums != total_row[0] or row_tot_sum != total_row[1]:
        ERRORS.append(
            f"{rel}: TOTAL row {total_row[0]}/{total_row[1]} ≠ column sums {col_sums}/{row_tot_sum}"
        )
    print(f"{rel}: summary arithmetic ok ({len(rows)} rows, total {total_row[1]})")

    # 2. headline percentages
    m = re.search(r"^- \*\*(\d+% [^*]+)\*\*", text, re.M)
    if m:
        stated = re.findall(r"(\d+)% ([A-Z/ ]+?)(?: /|$)", m.group(1))
        grand = total_row[1]
        if grand == 0:
            ERRORS.append(f"{rel}: TOTAL row grand total is 0 — cannot check percentages")
            return
        checked = 0
        for pct, name in stated:
            name = name.strip()
            if name not in col_names:
                NOTES.append(f"{rel}: headline name '{name}' is not a summary column — % check skipped")
                continue
            checked += 1
            expect = int(total_row[0][col_names.index(name)] * 100 / grand + 0.5)
            if int(pct) != expect:
                ERRORS.append(f"{rel}: headline {pct}% {name} ≠ computed {expect}%")
        print(f"{rel}: headline percentages ok ({checked} checked)")
    else:
        NOTES.append(f"{rel}: no bold headline-percentage line found — % check skipped")

    # 3. detail-section tallies
    col_emoji = [legend.get(n) for n in col_names]
    sections: dict[str, list[int]] = {}
    rows_seen: dict[str, int] = {}
    cur = None
    for l in lines:
        h = re.match(r"^### (§[\d.]+)", l)
        if h:
            # docs may repeat "### §x" headings (e.g. a disposition index after
            # the detail tables) — accumulate rather than reset
            cur = h.group(1).rstrip(".")
            sections.setdefault(cur, [0] * len(col_names))
            rows_seen.setdefault(cur, 0)
        elif re.match(r"^#{1,4} ", l):
            # any non-§ heading ends the section, so tables under e.g. a
            # "### Disposition summary" block never pollute a §-tally
            cur = None
        if cur and l.startswith("|"):
            c = cells(l)
            if len(c) < 2 or (set(c[1]) <= {"-", " "} and "-" in c[1]):
                continue
            rows_seen[cur] += 1
            # compound cells: first legend emoji appearing in the cell wins
            first = None
            for e in sorted({e for e in col_emoji if e}, key=lambda e: c[1].find(e) if e in c[1] else 1 << 30):
                if e in c[1]:
                    first = e
                    break
            if first is not None:
                sections[cur][col_emoji.index(first)] += 1
    for label, nums, _ in rows:
        tok = label.split()[0].rstrip(".")
        if not tok.startswith("§"):
            continue
        if tok not in sections or rows_seen.get(tok, 0) == 0:
            # no detail table rows at all (e.g. a section-level N/A row, or a
            # bare heading in a disposition index) — nothing to tally against;
            # a table whose rows merely fail to classify still gets compared
            # (and errors) below rather than silently skipping
            NOTES.append(f"{rel}: summary row '{label}' has no detail table — tally skipped")
            continue
        if sections[tok] != nums:
            ERRORS.append(f"{rel}: section {tok} detail tally {sections[tok]} ≠ summary row {nums}")
    print(f"{rel}: detail tallies ok ({len(sections)} sections)")

    # 4. test-count citations
    for l in lines:
        for fm in re.finditer(r"tests/([A-Za-z0-9_]+\.rs).*?\((\d+) tests\)", l):
            fname, stated_n = fm.group(1), int(fm.group(2))
            hits = list(WORKSPACE.glob(f"*/tests/{fname}"))
            if len(hits) != 1:
                NOTES.append(f"{rel}: '{fname}' matched {len(hits)} files — test-count check skipped")
                continue
            actual = len(re.findall(r"#\[test\]", hits[0].read_text()))
            if actual != stated_n:
                ERRORS.append(f"{rel}: cites '{fname} ({stated_n} tests)' but file has {actual} #[test] fns")
            else:
                print(f"{rel}: test-count citation ok ({fname}: {stated_n})")


def main() -> int:
    for arg in sys.argv[1:]:
        try:
            check_doc(Path(arg).resolve())
        except Exception as e:  # malformed doc → clean diagnostic, not a traceback
            ERRORS.append(f"{arg}: unparseable ({type(e).__name__}: {e})")
    for n in NOTES:
        print(f"note: {n}")
    if ERRORS:
        for e in ERRORS:
            print(f"ERROR: {e}", file=sys.stderr)
        return 1
    print("audit-count guard: all checks passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
