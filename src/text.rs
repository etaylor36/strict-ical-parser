//! Decoding and encoding of the RFC 5545 §3.3.11 TEXT escaping: `\\`, `\;`,
//! `\,`, and `\n`/`\N` for a literal newline. Nothing else after a backslash
//! is valid.

/// Properties whose value type is TEXT per RFC 5545 §3.8, i.e. the ones where
/// `\\`, `\;`, `\,` and `\n`/`\N` escaping applies. This isn't every TEXT
/// property in the spec, just the ones likely to show up in real files.
pub const TEXT_PROPERTIES: &[&str] = &[
    "ACTION",
    "CATEGORIES",
    "CLASS",
    "COMMENT",
    "CONTACT",
    "DESCRIPTION",
    "LOCATION",
    "PRODID",
    "RELATED-TO",
    "REQUEST-STATUS",
    "RESOURCES",
    "STATUS",
    "SUMMARY",
    "TRANSP",
    "TZID",
    "TZNAME",
    "UID",
];

/// Decode an escaped TEXT property value into the literal characters it
/// represents. Returns the byte offset of the backslash on the first escape
/// sequence that isn't `\\`, `\;`, `\,`, `\n`, or `\N`.
pub fn unescape(raw: &str) -> Result<String, usize> {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.char_indices();
    while let Some((i, c)) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some((_, '\\')) => out.push('\\'),
            Some((_, ';')) => out.push(';'),
            Some((_, ',')) => out.push(','),
            Some((_, 'n')) | Some((_, 'N')) => out.push('\n'),
            _ => return Err(i),
        }
    }
    Ok(out)
}

/// Encode a plain-text value for use as a TEXT property value, escaping
/// backslashes, semicolons, commas, and newlines.
pub fn escape(plain: &str) -> String {
    let mut out = String::with_capacity(plain.len());
    for c in plain.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            ';' => out.push_str("\\;"),
            ',' => out.push_str("\\,"),
            '\n' => out.push_str("\\n"),
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unescape_decodes_all_five_sequences() {
        let raw = r"a\,b\;c\\d\ne";
        assert_eq!(unescape(raw).unwrap(), "a,b;c\\d\ne");
    }

    #[test]
    fn unescape_accepts_uppercase_n_for_newline() {
        assert_eq!(unescape(r"line1\Nline2").unwrap(), "line1\nline2");
    }

    #[test]
    fn unescape_passes_through_plain_text() {
        assert_eq!(unescape("no escapes here").unwrap(), "no escapes here");
    }

    #[test]
    fn unescape_rejects_unknown_escape() {
        let err = unescape(r"bad\xvalue").unwrap_err();
        assert_eq!(err, 3);
    }

    #[test]
    fn unescape_rejects_trailing_backslash() {
        let err = unescape(r"trailing\").unwrap_err();
        assert_eq!(err, 8);
    }

    #[test]
    fn escape_is_the_inverse_of_unescape() {
        let plain = "a,b;c\\d\ne";
        let escaped = escape(plain);
        assert_eq!(escaped, r"a\,b\;c\\d\ne");
        assert_eq!(unescape(&escaped).unwrap(), plain);
    }

    #[test]
    fn escape_leaves_safe_characters_alone() {
        assert_eq!(escape("Team Standup: room 4"), "Team Standup: room 4");
    }
}
