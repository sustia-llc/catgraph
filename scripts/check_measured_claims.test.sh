#!/usr/bin/env bash
# Self-test for check_measured_claims.py.
#
# A guard that cannot fail is not a guard (mutants-knowledge §8.5), and this one
# is a required CI gate, so its own failure modes are pinned here rather than
# demonstrated once by hand. Every case below was a real review finding or a
# real false-green reproduced against an earlier draft.
#
# Run: bash scripts/check_measured_claims.test.sh
set -u

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
guard="$here/check_measured_claims.py"
data="$here/testdata/measured-claims"
log="$data/facts.log"

pass=0
fail=0

expect() {
    local want="$1" name="$2" root="$3"
    local out rc
    out="$(python3 "$guard" "$log" "$root" 2>&1)"
    rc=$?
    if [ "$rc" -eq "$want" ]; then
        pass=$((pass + 1))
        printf 'ok   %-42s (exit %d)\n' "$name" "$rc"
    else
        fail=$((fail + 1))
        printf 'FAIL %-42s (exit %d, wanted %d)\n%s\n' "$name" "$rc" "$want" "$out"
    fi
}

# Correct citations, plus every shape that must NOT trip the guard: grouped and
# unseparated digits, zero, negatives, earlier digits on the line, two markers
# on one line, a table cell, extra whitespace in the comment opener, and both
# fence styles used as the escape hatch.
expect 0 "correct citations pass"            "$data/pass"

# A cited figure that no longer matches what the test measured.
expect 1 "drifted figure fails"              "$data/drift"

# The renamed-or-deleted-test case: prose still claims a figure nothing emits.
expect 1 "unknown key fails"                 "$data/unknown"

# B2: `0.7828` must not be read as `7828`.
expect 1 "decimal not read as its tail"      "$data/detached"

# The symmetric counterpart of the empty-log check.
expect 1 "prose with no citations fails"     "$data/nocite"

# A log carrying no facts cannot verify anything (mutants-knowledge §1.3: exit 0
# must never be able to mean "nothing was tested").
out="$(python3 "$guard" "$data/nocite/nocite.md" "$data/pass" 2>&1)"
if [ $? -eq 1 ]; then
    pass=$((pass + 1))
    printf 'ok   %-42s (exit 1)\n' "log with no facts fails"
else
    fail=$((fail + 1))
    printf 'FAIL %-42s\n%s\n' "log with no facts fails" "$out"
fi

printf '\n%d passed, %d failed\n' "$pass" "$fail"
[ "$fail" -eq 0 ]
