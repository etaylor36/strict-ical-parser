# icalfmt

A validating parser and pretty printer for iCalendar (`.ics`, RFC 5545) files.

## The problem

iCalendar exports from real calendar software are inconsistent in ways the
spec doesn't allow: missing `VERSION` or `PRODID`, mismatched `BEGIN`/`END`
pairs, bare `LF` line endings instead of `CRLF`, inconsistent casing on
component and property names. Most tools that read `.ics` files just accept
whatever they're given, which means bugs in a producer quietly propagate
into every downstream consumer.

icalfmt takes the opposite default: it parses strictly against RFC 5545 and
rejects malformed input. If you're working with a feed you know is sloppy,
you opt into tolerating that explicitly with `--lenient` rather than the
tool silently guessing what you meant.

## What it does

- Unfolds RFC 5545 line folding and re-parses each logical line into a
  component tree (`VCALENDAR` / `VEVENT` / `VALARM` / ...), with properties
  and their parameters.
- In strict mode: requires `CRLF` line endings, a single `VCALENDAR` root,
  matching `BEGIN`/`END` pairs, and required `VERSION`/`PRODID` properties.
  Any violation aborts with a list of diagnostics on stderr and a non-zero
  exit code.
- In `--lenient` mode: bare `LF` is accepted, a mismatched `END` name is
  downgraded to a warning instead of failing, and missing `VERSION`/`PRODID`
  become warnings instead of errors. Structural corruption (an `END` with no
  matching `BEGIN`, a property outside any component, an unclosed component)
  still fails in both modes — there's no sane way to guess past that.
- Validates the `\\`, `\;`, `\,`, `\n`/`\N` escaping used inside `TEXT`-typed
  properties (`SUMMARY`, `DESCRIPTION`, `UID`, `CATEGORIES`, ...). A stray
  backslash followed by anything else is a strict-mode error and a
  lenient-mode warning. `src/text.rs` also exposes `unescape`/`escape` for
  decoding a value to its literal form and back.
- Pretty-prints the parsed result: uppercases component and parameter names,
  normalizes line endings to `CRLF`, and re-folds long lines at 75 octets.

## Usage

```
$ cargo run -- calendar.ics
$ cargo run -- --lenient calendar.ics
```

Given this input (`calendar.ics`, saved with CRLF line endings):

```
BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//example//icalfmt//EN
BEGIN:vevent
uid:1234@example.com
dtstamp:20260101T000000Z
dtstart:20260101T090000Z
summary:Standup
END:VEVENT
END:VCALENDAR
```

`cargo run -- calendar.ics` prints the normalized form to stdout:

```
BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//example//icalfmt//EN
BEGIN:VEVENT
UID:1234@example.com
DTSTAMP:20260101T000000Z
DTSTART:20260101T090000Z
SUMMARY:Standup
END:VEVENT
END:VCALENDAR
```

If a required property is missing or a line uses a bare `LF`, the strict
run stops and reports where, e.g.:

```
error: VCALENDAR is missing the required VERSION property
error at line 4: line is terminated with a bare LF, RFC 5545 requires CRLF
calendar.ics: failed to parse (2 diagnostic(s))
```

Re-running with `--lenient` turns those into warnings and still produces
output.

## Current scope

This is an early skeleton. It handles structural parsing (folding,
`BEGIN`/`END` nesting, parameters, `VCALENDAR`-level requirements) and
`TEXT` escape validation correctly, but it does not yet validate property
value types (`DATE-TIME`, `DURATION`, `RRULE`, ...), does not resolve `TZID`
references against `VTIMEZONE` blocks, and the pretty printer passes
property values through unchanged rather than round-tripping them through
`text::unescape`/`text::escape`.

## Layout

- `src/parser.rs` — unfolding, content-line tokenizing, tree building,
  validation.
- `src/text.rs` — RFC 5545 TEXT escaping (`unescape`/`escape`).
- `src/writer.rs` — pretty printing and re-folding.
- `src/lib.rs` — the shared `Component`/`ContentLine`/`Diagnostic` types.
- `src/main.rs` — CLI glue.

## License

MIT, see `LICENSE`.
