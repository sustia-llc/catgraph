# B1 — the whole file's only marker uses extra whitespace, and is WRONG

This pins round 1's blocking false green: a file-level fast path tested for one
literal spelling of the comment opener while the matcher accepted any whitespace
run, so this marker was skipped — but only when it was the sole marker in its
file, which is why the fixture needs a sibling that cites normally.

DO NOT write the plain spelling of the opener anywhere in this file. Doing so
re-arms the fast path, the file is scanned for an unrelated reason, and the
case passes while the bug is live. That is exactly what happened on the first
draft of this fixture.

The count was **9 999**<!--  m:fixture.plain  --> against a measured 7828.
