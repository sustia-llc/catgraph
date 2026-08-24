# Shapes that must PASS

Plain digits: **7 828**<!--m:fixture.plain--> of them.

Grouped with a space: **14 473**<!--m:fixture.grouped--> composable triples.

Grouped with a comma: 14,473<!--m:fixture.grouped--> the same figure.

Unseparated: 14473<!--m:fixture.grouped--> again.

Zero is a value like any other: **0**<!--m:fixture.zero--> mismatches.

Negative: **-12**<!--m:fixture.negative--> drift.

Digits earlier on the line must not be absorbed — in #350, the count was
**7 828**<!--m:fixture.plain--> and not 3507828.

Two markers on one line: **0**<!--m:fixture.zero--> and **7 828**<!--m:fixture.plain-->.

A table cell:

| metric | value |
|---|---|
| plain | 7828<!--m:fixture.plain--> |

Whitespace inside the comment opener is tolerated: **0**<!--  m:fixture.zero  -->.

A fenced block is the escape hatch — this WRONG citation must be skipped:

```markdown
**999**<!--m:fixture.plain--> composable triples
```

A tilde fence too:

~~~
**999**<!--m:fixture.grouped-->
~~~

The harness prefix on a MEASURED line (see facts.log, `fixture.zero`) is
tolerated, which this file's zero citation depends on.
