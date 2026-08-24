#!/usr/bin/env bash
# Self-test for check_measured_claims.py.
#
# A guard that cannot fail is not a guard, and this one
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

# Asserts the exit code AND a substring of the message. Exit code alone lets a
# case go green for the wrong reason — `nocite` failing because the walker found
# no files would look identical to it failing on the no-citations rule.
expect() {
    local want="$1" name="$2" root="$3" needle="${4:-}"
    local out rc
    out="$(python3 "$guard" "$log" "$root" 2>&1)"
    rc=$?
    if [ "$rc" -ne "$want" ]; then
        fail=$((fail + 1))
        printf 'FAIL %-42s (exit %d, wanted %d)\n%s\n' "$name" "$rc" "$want" "$out"
        return
    fi
    if [ -n "$needle" ] && [[ "$out" != *"$needle"* ]]; then
        fail=$((fail + 1))
        printf 'FAIL %-42s (exit %d ok, but message lacked %q)\n%s\n' \
            "$name" "$rc" "$needle" "$out"
        return
    fi
    pass=$((pass + 1))
    printf 'ok   %-42s (exit %d)\n' "$name" "$rc"
}

# Correct citations, plus every shape that must NOT trip the guard: grouped and
# unseparated digits, zero, negatives, earlier digits on the line, two markers
# on one line, a table cell, extra whitespace in the comment opener, both fence
# styles as the escape hatch, and fences behind `///` / `//!` in a .rs file.
expect 0 "correct citations pass"            "$data/pass"

# A cited figure that no longer matches what the test measured.
expect 1 "drifted figure fails"              "$data/drift" "as 7829 — measured 7828"

# The renamed-or-deleted-test case: prose still claims a figure nothing emits.
expect 1 "unknown key fails"                 "$data/unknown" "which no test emitted"

# B2: `0.7828` must not be read as `7828`.
expect 1 "decimal not read as its tail"      "$data/detached" "no number touching it"

# The symmetric counterpart of the empty-log check.
expect 1 "prose with no citations fails"     "$data/nocite" "no prose cited any measured fact"

# B1, round 1's blocking false green: a file whose ONLY marker carries extra
# whitespace in the comment opener. Needs the sibling file, or it would fail on
# the no-citations rule instead and pass while B1 stayed live.
expect 1 "spaced sole marker is checked"     "$data/spacedmarker" "as 9999 — measured 7828"

# An unterminated fence must be an error, not a silent skip of the rest.
expect 1 "unterminated fence fails"          "$data/unbalanced" "unterminated code fence"

# A log carrying no facts cannot verify anything: exit 0 must never be able to
# mean "nothing was tested".
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
