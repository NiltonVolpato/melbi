//! Beautiful error rendering using ariadne
//!
//! This module provides utilities for rendering Melbi errors with
//! rich formatting, source code snippets, and helpful annotations.

use crate::{Diagnostic, Error, Severity};
use ariadne::{ColorGenerator, Label, Report, ReportKind, Source};
use std::io::Write;

/// Render an error with beautiful formatting to stderr
///
/// # Example
/// ```no_run
/// use melbi::{Engine, EngineOptions, render_error};
/// use bumpalo::Bump;
///
/// let arena = Bump::new();
/// let engine = Engine::new(EngineOptions::default(), &arena, |_,_,_| {});
///
/// let source = "1 + true";
/// match engine.compile(Default::default(), source, &[]) {
///     Err(e) => render_error(&e),
///     Ok(_) => {}
/// }
/// ```
pub fn render_error(error: &Error) {
    render_error_to_writer(error, &mut std::io::stderr(), true).ok();
}

/// Render an error to a specific writer
///
/// This is useful when you want to control where the error is written,
/// such as to a file, a buffer, or a custom output stream.
pub fn render_error_to(error: &Error, writer: &mut dyn Write) -> std::io::Result<()> {
    render_error_to_writer(error, writer, true)
}

/// Render an error to a String (useful for tests, web UIs, etc.)
///
/// # Example
/// ```no_run
/// use melbi::{Engine, EngineOptions, render_error_to_string};
/// use bumpalo::Bump;
///
/// let arena = Bump::new();
/// let engine = Engine::new(EngineOptions::default(), &arena, |_,_,_| {});
///
/// let source = "1 + true";
/// match engine.compile(Default::default(), source, &[]) {
///     Err(e) => {
///         let formatted = render_error_to_string(&e);
///         // Use formatted error in UI, logs, etc.
///     }
///     Ok(_) => {}
/// }
/// ```
pub fn render_error_to_string(error: &Error) -> String {
    let mut buf = Vec::new();
    render_error_to_writer(error, &mut buf, true).ok();
    String::from_utf8_lossy(&buf).to_string()
}

/// Render an error to a String without color codes (useful for tests)
///
/// This is the same as `render_error_to_string` but without ANSI color codes,
/// making the output easier to compare in tests.
pub fn render_error_to_string_no_color(error: &Error) -> String {
    let mut buf = Vec::new();
    render_error_to_writer(error, &mut buf, false).ok();
    String::from_utf8_lossy(&buf).to_string()
}

fn render_error_to_writer(
    error: &Error,
    writer: &mut dyn Write,
    use_color: bool,
) -> std::io::Result<()> {
    match error {
        Error::Compilation {
            diagnostics,
            source,
        } => render_diagnostics(source, diagnostics, writer, use_color),
        Error::Runtime {
            diagnostic,
            source,
        } => render_diagnostics(source, &[diagnostic.clone()], writer, use_color),
        Error::ResourceExceeded(msg) => {
            writeln!(writer, "Resource limit exceeded: {}", msg)
        }
        Error::Api(msg) => {
            writeln!(writer, "API error: {}", msg)
        }
    }
}

fn render_diagnostics(
    source: &str,
    diagnostics: &[Diagnostic],
    writer: &mut dyn Write,
    use_color: bool,
) -> std::io::Result<()> {
    for diag in diagnostics {
        let mut colors = ColorGenerator::new();
        colors.next(); // Skip the first color.

        let kind = match diag.severity {
            Severity::Error => ReportKind::Error,
            Severity::Warning => ReportKind::Warning,
            Severity::Info => ReportKind::Advice,
        };

        let mut report = Report::build(kind, ("<unknown>", diag.span.0.clone()))
            .with_message(&diag.message)
            .with_config(ariadne::Config::default().with_color(use_color));

        // Add error code if present
        if let Some(code) = &diag.code {
            report = report.with_code(code);
        }

        // Primary label with the main error span
        let color = colors.next();
        report = report.with_label(
            Label::new(("<unknown>", diag.span.0.clone()))
                .with_message(&diag.message)
                .with_color(color),
        );

        // Related info as secondary labels (shows context breadcrumbs!)
        for related in &diag.related {
            let color = colors.next();
            report = report.with_label(
                Label::new(("<unknown>", related.span.0.clone()))
                    .with_message(&related.message)
                    .with_color(color),
            );
        }

        // Help text as notes
        for help_msg in &diag.help {
            report = report.with_help(help_msg);
        }

        // Render to the writer (need to reborrow to avoid moving)
        report.finish().write(("<unknown>", Source::from(source)), &mut *writer)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Engine, EngineOptions};
    use bumpalo::Bump;

    #[test]
    fn test_render_parse_error() {
        let arena = Bump::new();
        let engine = Engine::new(EngineOptions::default(), &arena, |_, _, _| {});

        let source = "1 + + 2"; // Invalid syntax
        let result = engine.compile(Default::default(), source, &[]);

        assert!(result.is_err());
        if let Err(e) = result {
            let output = render_error_to_string_no_color(&e);

            // Should contain error indicator
            assert!(output.contains("Error") || output.contains("error"));
            // Should show the source
            assert!(output.contains("1 + + 2"));
        }
    }

    #[test]
    fn test_render_type_error() {
        let arena = Bump::new();
        let engine = Engine::new(EngineOptions::default(), &arena, |_, _, _| {});

        let source = "1 + \"hello\""; // Type mismatch
        let result = engine.compile(Default::default(), source, &[]);

        assert!(result.is_err());
        if let Err(e) = result {
            let output = render_error_to_string_no_color(&e);

            // Should indicate type error
            assert!(output.contains("Type") || output.contains("type"));
        }
    }

    #[test]
    fn test_render_to_string_captures_output() {
        let arena = Bump::new();
        let engine = Engine::new(EngineOptions::default(), &arena, |_, _, _| {});

        let source = "bad syntax {";
        let result = engine.compile(Default::default(), source, &[]);

        assert!(result.is_err());
        if let Err(e) = result {
            let output = render_error_to_string_no_color(&e);

            // Output should not be empty
            assert!(!output.is_empty());
            // Should be multi-line (ariadne adds formatting)
            assert!(output.lines().count() > 1);
        }
    }
}
