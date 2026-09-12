pub mod parser;
pub mod writer;

/// A single parameter attached to a property, e.g. `TZID=America/Chicago` in
/// `DTSTART;TZID=America/Chicago:20260101T090000`.
#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub values: Vec<String>,
}

/// One unfolded, parsed line of an iCalendar file: `NAME;params:value`.
#[derive(Debug, Clone)]
pub struct ContentLine {
    pub name: String,
    pub params: Vec<Param>,
    pub value: String,
    pub source_line: usize,
}

/// A BEGIN/END block such as VCALENDAR, VEVENT, or VALARM, holding its own
/// properties and any nested components.
#[derive(Debug, Clone)]
pub struct Component {
    pub name: String,
    pub properties: Vec<ContentLine>,
    pub children: Vec<Component>,
}

impl Component {
    pub fn new(name: impl Into<String>) -> Self {
        Component {
            name: name.into(),
            properties: Vec::new(),
            children: Vec::new(),
        }
    }

    pub fn property(&self, name: &str) -> Option<&ContentLine> {
        self.properties.iter().find(|p| p.name.eq_ignore_ascii_case(name))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kind = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        if self.line == 0 {
            write!(f, "{kind}: {}", self.message)
        } else {
            write!(f, "{kind} at line {}: {}", self.line, self.message)
        }
    }
}

pub struct ParseOutcome {
    pub calendar: Component,
    pub diagnostics: Vec<Diagnostic>,
}
