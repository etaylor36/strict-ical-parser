use crate::{Component, ContentLine, Diagnostic, Param, Severity};

/// Parse a full iCalendar document.
///
/// In strict mode (`lenient = false`) any error aborts parsing and the
/// diagnostics collected so far are returned as `Err`. In lenient mode,
/// spec violations that don't corrupt the BEGIN/END structure are downgraded
/// to warnings and parsing continues.
pub fn parse(input: &str, lenient: bool) -> Result<crate::ParseOutcome, Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();

    check_line_endings(input, lenient, &mut diagnostics);
    let logical_lines = unfold(input, &mut diagnostics);

    let mut content_lines = Vec::new();
    for (line_no, raw) in &logical_lines {
        if raw.is_empty() {
            continue;
        }
        match parse_content_line(raw, *line_no) {
            Ok(cl) => content_lines.push(cl),
            Err(message) => diagnostics.push(Diagnostic {
                severity: Severity::Error,
                line: *line_no,
                message,
            }),
        }
    }

    if !lenient && diagnostics.iter().any(|d| d.severity == Severity::Error) {
        return Err(diagnostics);
    }

    let mut tree = match build_tree(content_lines, lenient, &mut diagnostics) {
        Ok(tree) => tree,
        Err(message) => {
            diagnostics.push(Diagnostic {
                severity: Severity::Error,
                line: 0,
                message,
            });
            return Err(diagnostics);
        }
    };

    validate(&mut tree, lenient, &mut diagnostics);

    if !lenient && diagnostics.iter().any(|d| d.severity == Severity::Error) {
        return Err(diagnostics);
    }

    Ok(crate::ParseOutcome {
        calendar: tree,
        diagnostics,
    })
}

fn check_line_endings(input: &str, lenient: bool, diagnostics: &mut Vec<Diagnostic>) {
    if lenient {
        return;
    }
    let bytes = input.as_bytes();
    let mut line_no = 1;
    for i in 0..bytes.len() {
        if bytes[i] == b'\n' {
            let has_cr = i > 0 && bytes[i - 1] == b'\r';
            if !has_cr {
                diagnostics.push(Diagnostic {
                    severity: Severity::Error,
                    line: line_no,
                    message: "line is terminated with a bare LF, RFC 5545 requires CRLF".to_string(),
                });
            }
            line_no += 1;
        }
    }
}

// RFC 5545 line folding: a line that starts with a single space or tab is a
// continuation of the previous line, with that one leading character removed.
fn unfold(input: &str, diagnostics: &mut Vec<Diagnostic>) -> Vec<(usize, String)> {
    let mut logical: Vec<(usize, String)> = Vec::new();
    for (idx, raw) in input.lines().enumerate() {
        let line_no = idx + 1;
        if raw.starts_with(' ') || raw.starts_with('\t') {
            match logical.last_mut() {
                Some((_, buf)) => buf.push_str(&raw[1..]),
                None => {
                    diagnostics.push(Diagnostic {
                        severity: Severity::Error,
                        line: line_no,
                        message: "continuation line has no preceding line to fold onto".to_string(),
                    });
                    logical.push((line_no, raw[1..].to_string()));
                }
            }
        } else if !raw.is_empty() {
            logical.push((line_no, raw.to_string()));
        }
    }
    logical
}

fn parse_content_line(raw: &str, line_no: usize) -> Result<ContentLine, String> {
    let bytes = raw.as_bytes();
    let sep = bytes
        .iter()
        .position(|&b| b == b':' || b == b';')
        .ok_or_else(|| format!("line has no ':' separator: {raw:?}"))?;

    let name = raw[..sep].to_string();
    if name.is_empty() {
        return Err("property name is empty".to_string());
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err(format!(
            "property name {name:?} contains characters other than A-Z, 0-9 and '-'"
        ));
    }

    let (params, value) = if bytes[sep] == b':' {
        (Vec::new(), raw[sep + 1..].to_string())
    } else {
        let colon = find_unquoted(bytes, b':', sep + 1)
            .ok_or_else(|| "line has parameters but no ':' before the value".to_string())?;
        let params = parse_params(&raw[sep + 1..colon])?;
        (params, raw[colon + 1..].to_string())
    };

    Ok(ContentLine {
        name,
        params,
        value,
        source_line: line_no,
    })
}

fn parse_params(s: &str) -> Result<Vec<Param>, String> {
    let mut params = Vec::new();
    for chunk in split_unquoted(s, b';') {
        if chunk.is_empty() {
            return Err("empty parameter between ';' separators".to_string());
        }
        let chunk_bytes = chunk.as_bytes();
        let eq = find_unquoted(chunk_bytes, b'=', 0)
            .ok_or_else(|| format!("parameter {chunk:?} is missing '='"))?;
        let name = chunk[..eq].to_string();
        if name.is_empty() {
            return Err("parameter name is empty".to_string());
        }
        let values = split_unquoted(&chunk[eq + 1..], b',')
            .into_iter()
            .map(unquote)
            .collect();
        params.push(Param { name, values });
    }
    Ok(params)
}

fn unquote(v: &str) -> String {
    if v.len() >= 2 && v.starts_with('"') && v.ends_with('"') {
        v[1..v.len() - 1].to_string()
    } else {
        v.to_string()
    }
}

fn find_unquoted(bytes: &[u8], target: u8, start: usize) -> Option<usize> {
    let mut in_quotes = false;
    for (i, &b) in bytes.iter().enumerate().skip(start) {
        match b {
            b'"' => in_quotes = !in_quotes,
            b if b == target && !in_quotes => return Some(i),
            _ => {}
        }
    }
    None
}

fn split_unquoted(s: &str, target: u8) -> Vec<&str> {
    let bytes = s.as_bytes();
    let mut parts = Vec::new();
    let mut start = 0;
    let mut in_quotes = false;
    for (i, &b) in bytes.iter().enumerate() {
        match b {
            b'"' => in_quotes = !in_quotes,
            b if b == target && !in_quotes => {
                parts.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(&s[start..]);
    parts
}

fn build_tree(
    lines: Vec<ContentLine>,
    lenient: bool,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<Component, String> {
    let mut stack: Vec<Component> = Vec::new();
    let mut root: Option<Component> = None;

    for line in lines {
        if line.name.eq_ignore_ascii_case("BEGIN") {
            stack.push(Component::new(line.value));
        } else if line.name.eq_ignore_ascii_case("END") {
            let ended = stack.pop().ok_or_else(|| {
                format!("line {}: END:{} has no matching BEGIN", line.source_line, line.value)
            })?;
            if !ended.name.eq_ignore_ascii_case(&line.value) {
                let message = format!(
                    "line {}: END:{} does not match the innermost open BEGIN:{}",
                    line.source_line, line.value, ended.name
                );
                if lenient {
                    diagnostics.push(Diagnostic {
                        severity: Severity::Warning,
                        line: line.source_line,
                        message,
                    });
                } else {
                    return Err(message);
                }
            }
            match stack.last_mut() {
                Some(parent) => parent.children.push(ended),
                None => {
                    if root.is_some() {
                        return Err(format!(
                            "line {}: input has more than one top level component",
                            line.source_line
                        ));
                    }
                    root = Some(ended);
                }
            }
        } else {
            match stack.last_mut() {
                Some(current) => current.properties.push(line),
                None => {
                    return Err(format!(
                        "line {}: property {} appears outside of any BEGIN/END component",
                        line.source_line, line.name
                    ));
                }
            }
        }
    }

    if let Some(unclosed) = stack.pop() {
        return Err(format!("component {} was opened but never closed with END", unclosed.name));
    }

    root.ok_or_else(|| "input contained no top level component".to_string())
}

fn validate(tree: &mut Component, lenient: bool, diagnostics: &mut Vec<Diagnostic>) {
    let severity = if lenient { Severity::Warning } else { Severity::Error };

    if !tree.name.eq_ignore_ascii_case("VCALENDAR") {
        diagnostics.push(Diagnostic {
            severity,
            line: 0,
            message: format!("top level component is {}, RFC 5545 requires VCALENDAR", tree.name),
        });
    }

    if tree.property("VERSION").is_none() {
        diagnostics.push(Diagnostic {
            severity,
            line: 0,
            message: "VCALENDAR is missing the required VERSION property".to_string(),
        });
    }

    if tree.property("PRODID").is_none() {
        diagnostics.push(Diagnostic {
            severity,
            line: 0,
            message: "VCALENDAR is missing the required PRODID property".to_string(),
        });
    }

    if tree.children.is_empty() {
        diagnostics.push(Diagnostic {
            severity: Severity::Warning,
            line: 0,
            message: "VCALENDAR has no sub components such as VEVENT or VTODO".to_string(),
        });
    }

    decode_text_properties(tree, lenient, diagnostics);
}

// Decode each TEXT property's value in place, from its RFC 5545 §3.3.11
// escaped form on the wire to the literal text it represents. The writer
// re-escapes from this literal form rather than ever touching the raw bytes
// that were read from the source, so the escaping logic has exactly one
// place it can drift from the spec.
fn decode_text_properties(component: &mut Component, lenient: bool, diagnostics: &mut Vec<Diagnostic>) {
    let severity = if lenient { Severity::Warning } else { Severity::Error };

    for prop in &mut component.properties {
        let is_text = crate::text::TEXT_PROPERTIES
            .iter()
            .any(|name| prop.name.eq_ignore_ascii_case(name));
        if !is_text {
            continue;
        }
        match crate::text::unescape(&prop.value) {
            Ok(decoded) => prop.value = decoded,
            Err(offset) => diagnostics.push(Diagnostic {
                severity,
                line: prop.source_line,
                message: format!(
                    "property {} has an invalid escape sequence at offset {offset}, only \\\\, \\;, \\, \\n and \\N are valid",
                    prop.name
                ),
            }),
        }
    }

    for child in &mut component.children {
        decode_text_properties(child, lenient, diagnostics);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ok(input: &str) -> crate::ParseOutcome {
        parse(input, false).expect("expected input to parse")
    }

    #[test]
    fn text_property_values_are_stored_decoded() {
        let outcome = parse_ok(
            "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//test//EN\r\nBEGIN:VEVENT\r\nUID:1\r\nSUMMARY:Team\\, standup\\; room 4\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n",
        );
        let event = &outcome.calendar.children[0];
        assert_eq!(event.property("SUMMARY").unwrap().value, "Team, standup; room 4");
    }

    #[test]
    fn writer_round_trips_decoded_text_through_escaping() {
        let outcome = parse_ok(
            "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//test//EN\r\nBEGIN:VEVENT\r\nUID:1\r\nSUMMARY:Team\\, standup\\; room 4\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n",
        );
        let printed = crate::writer::print(&outcome.calendar);
        assert!(printed.contains("SUMMARY:Team\\, standup\\; room 4"));
    }

    #[test]
    fn writer_normalizes_uppercase_n_newline_escape_to_lowercase() {
        let outcome = parse_ok(
            "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//test//EN\r\nBEGIN:VEVENT\r\nUID:1\r\nDESCRIPTION:line1\\Nline2\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n",
        );
        let printed = crate::writer::print(&outcome.calendar);
        assert!(printed.contains("DESCRIPTION:line1\\nline2"));
    }

    #[test]
    fn invalid_text_escape_is_reported_and_left_undecoded() {
        let diagnostics = parse(
            "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//test//EN\r\nBEGIN:VEVENT\r\nUID:1\r\nSUMMARY:bad\\xvalue\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n",
            false,
        )
        .unwrap_err();
        assert!(diagnostics.iter().any(|d| d.message.contains("invalid escape sequence")));
    }
}
