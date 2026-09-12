use std::env;
use std::fs;
use std::process::ExitCode;

use icalfmt::{parser, writer};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();

    let mut lenient = false;
    let mut path: Option<String> = None;

    for arg in args {
        match arg.as_str() {
            "--lenient" => lenient = true,
            "-h" | "--help" => {
                print_usage();
                return ExitCode::SUCCESS;
            }
            other => {
                if path.is_some() {
                    eprintln!("unexpected extra argument: {other}");
                    return ExitCode::FAILURE;
                }
                path = Some(other.to_string());
            }
        }
    }

    let path = match path {
        Some(p) => p,
        None => {
            print_usage();
            return ExitCode::FAILURE;
        }
    };

    let input = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("cannot read {path}: {e}");
            return ExitCode::FAILURE;
        }
    };

    match parser::parse(&input, lenient) {
        Ok(outcome) => {
            for diag in &outcome.diagnostics {
                eprintln!("{diag}");
            }
            print!("{}", writer::print(&outcome.calendar));
            ExitCode::SUCCESS
        }
        Err(diagnostics) => {
            for diag in &diagnostics {
                eprintln!("{diag}");
            }
            eprintln!("{path}: failed to parse ({} diagnostic(s))", diagnostics.len());
            ExitCode::FAILURE
        }
    }
}

fn print_usage() {
    eprintln!("usage: icalfmt [--lenient] <file.ics>");
    eprintln!();
    eprintln!("Parses an iCalendar file, validates it, and prints a normalized form to stdout.");
    eprintln!("By default parsing is strict; pass --lenient to accept common producer mistakes.");
}
