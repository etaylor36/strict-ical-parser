use crate::text;
use crate::{Component, ContentLine};

/// Render a component tree back into RFC 5545 text: uppercase component and
/// parameter names, CRLF line endings, and lines folded at 75 octets.
pub fn print(calendar: &Component) -> String {
    let mut out = String::new();
    write_component(calendar, &mut out);
    out
}

fn write_component(component: &Component, out: &mut String) {
    push_folded(&format!("BEGIN:{}", component.name.to_uppercase()), out);
    for prop in &component.properties {
        push_folded(&render_property(prop), out);
    }
    for child in &component.children {
        write_component(child, out);
    }
    push_folded(&format!("END:{}", component.name.to_uppercase()), out);
}

fn render_property(prop: &ContentLine) -> String {
    let mut line = prop.name.to_uppercase();
    for param in &prop.params {
        line.push(';');
        line.push_str(&param.name.to_uppercase());
        line.push('=');
        let rendered: Vec<String> = param.values.iter().map(|v| render_param_value(v)).collect();
        line.push_str(&rendered.join(","));
    }
    line.push(':');
    let is_text = text::TEXT_PROPERTIES.iter().any(|name| prop.name.eq_ignore_ascii_case(name));
    if is_text {
        line.push_str(&text::escape(&prop.value));
    } else {
        line.push_str(&prop.value);
    }
    line
}

fn render_param_value(v: &str) -> String {
    if v.contains(':') || v.contains(';') || v.contains(',') {
        format!("\"{v}\"")
    } else {
        v.to_string()
    }
}

const FOLD_WIDTH: usize = 75;

// Continuation lines carry a mandatory leading space that counts against
// the 75 octet budget, so they get one less byte of content per line.
fn push_folded(line: &str, out: &mut String) {
    let bytes = line.as_bytes();
    if bytes.len() <= FOLD_WIDTH {
        out.push_str(line);
        out.push_str("\r\n");
        return;
    }

    let mut start = 0;
    let mut first = true;
    while start < bytes.len() {
        let budget = if first { FOLD_WIDTH } else { FOLD_WIDTH - 1 };
        let mut end = (start + budget).min(bytes.len());
        while end > start && !is_utf8_boundary(bytes, end) {
            end -= 1;
        }
        if !first {
            out.push(' ');
        }
        out.push_str(&line[start..end]);
        out.push_str("\r\n");
        start = end;
        first = false;
    }
}

fn is_utf8_boundary(bytes: &[u8], idx: usize) -> bool {
    idx == bytes.len() || (bytes[idx] & 0b1100_0000) != 0b1000_0000
}
