use crate::api::{Diagnostic, Severity};
use crate::diagnostics::context::Context;
use crate::parser::{Rule, Span};
use crate::{String, Vec, format};
use alloc::string::ToString;

/// Parser error with context
#[derive(Debug)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub context: Vec<Context>,
}

/// Specific kinds of parse errors
#[derive(Debug)]
pub enum ParseErrorKind {
    /// Unexpected token
    UnexpectedToken {
        expected: String,
        found: String,
        span: Span,
    },
    /// Unclosed delimiter
    UnclosedDelimiter {
        delimiter: char,
        span: Span,
    },
    /// Invalid number literal
    InvalidNumber {
        text: String,
        span: Span,
    },
    /// Maximum nesting depth exceeded
    MaxDepthExceeded {
        depth: usize,
        max_depth: usize,
        span: Span,
    },
    /// Other parse errors (catch-all for Pest errors we don't specifically handle)
    Other {
        message: String,
        span: Span,
    },
}

impl ParseErrorKind {
    /// Get the span of the error
    pub fn span(&self) -> Span {
        match self {
            ParseErrorKind::UnexpectedToken { span, .. } => span.clone(),
            ParseErrorKind::UnclosedDelimiter { span, .. } => span.clone(),
            ParseErrorKind::InvalidNumber { span, .. } => span.clone(),
            ParseErrorKind::MaxDepthExceeded { span, .. } => span.clone(),
            ParseErrorKind::Other { span, .. } => span.clone(),
        }
    }
}

impl ParseError {
    /// Create a new ParseError with no context
    pub fn new(kind: ParseErrorKind) -> Self {
        Self {
            kind,
            context: Vec::new(),
        }
    }

    /// Convert to a Diagnostic for API boundary
    pub fn to_diagnostic(&self) -> Diagnostic {
        let (message, code, help) = match &self.kind {
            ParseErrorKind::UnexpectedToken { expected, found, .. } => (
                format!("Expected {}, found {}", expected, found),
                Some("P001"),
                None,
            ),
            ParseErrorKind::UnclosedDelimiter { delimiter, .. } => (
                format!("Unclosed delimiter '{}'", delimiter),
                Some("P002"),
                Some("Add the missing closing delimiter"),
            ),
            ParseErrorKind::InvalidNumber { text, .. } => (
                format!("Invalid number literal '{}'", text),
                Some("P003"),
                Some("Check the number format"),
            ),
            ParseErrorKind::MaxDepthExceeded { depth, max_depth, .. } => (
                format!(
                    "Expression nesting depth {} exceeds maximum {}",
                    depth, max_depth
                ),
                Some("P004"),
                Some("Reduce nesting or simplify the expression"),
            ),
            ParseErrorKind::Other { message, .. } => (
                message.clone(),
                Some("P999"),
                None,
            ),
        };

        Diagnostic {
            severity: Severity::Error,
            message,
            span: self.kind.span(),
            related: self
                .context
                .iter()
                .map(|ctx| ctx.to_related_info())
                .collect(),
            help: help.map(|s| s.to_string()),
            code: code.map(|s| s.to_string()),
        }
    }
}

/// Convert Pest error to human-readable ParseError
pub fn convert_pest_error(err: pest::error::Error<Rule>) -> ParseError {
    use pest::error::ErrorVariant;

    let span = match err.location {
        pest::error::InputLocation::Pos(pos) => Span(pos..pos),
        pest::error::InputLocation::Span((start, end)) => Span(start..end),
    };

    let kind = match err.variant {
        ErrorVariant::ParsingError {
            positives,
            negatives,
        } => {
            // Convert technical Pest messages to human-readable ones
            let expected = format_expected_rules(&positives);
            let found = format_found_rules(&negatives);

            ParseErrorKind::UnexpectedToken {
                expected,
                found,
                span,
            }
        }
        ErrorVariant::CustomError { message } => {
            // Check if it's a depth error
            if message.contains("nesting depth") {
                if let Some(depth_str) = extract_number_from_message(&message, "depth") {
                    if let Ok(depth) = depth_str.parse::<usize>() {
                        // Default max depth is in the message
                        let max_depth = extract_number_from_message(&message, "maximum")
                            .and_then(|s| s.parse::<usize>().ok())
                            .unwrap_or(100);
                        return ParseError::new(ParseErrorKind::MaxDepthExceeded {
                            depth,
                            max_depth,
                            span,
                        });
                    }
                }
            }

            ParseErrorKind::Other { message, span }
        }
    };

    ParseError::new(kind)
}

/// Format expected rules in a human-readable way
fn format_expected_rules(rules: &[Rule]) -> String {
    if rules.is_empty() {
        return "something else".to_string();
    }

    // Group related rules into higher-level concepts
    let mut concepts = Vec::new();

    for rule in rules {
        match rule {
            Rule::grouped | Rule::neg | Rule::not | Rule::if_op | Rule::lambda_op => {
                if !concepts.contains(&"expression") {
                    concepts.push("expression");
                }
            }
            Rule::integer | Rule::float | Rule::boolean | Rule::string | Rule::bytes => {
                if !concepts.contains(&"literal") {
                    concepts.push("literal");
                }
            }
            Rule::ident => {
                if !concepts.contains(&"identifier") {
                    concepts.push("identifier");
                }
            }
            Rule::EOI => {
                if !concepts.contains(&"end of input") {
                    concepts.push("end of input");
                }
            }
            _ => {
                // For other rules, just note "expression"
                if !concepts.contains(&"expression") {
                    concepts.push("expression");
                }
            }
        }
    }

    if concepts.is_empty() {
        return "something else".to_string();
    }

    if concepts.len() == 1 {
        concepts[0].to_string()
    } else {
        let last = concepts.pop().unwrap();
        format!("{} or {}", concepts.join(", "), last)
    }
}

/// Format found rules in a human-readable way
fn format_found_rules(rules: &[Rule]) -> String {
    if rules.is_empty() {
        return "unexpected token".to_string();
    }

    // For simplicity, just format the first negative rule
    format!("{:?}", rules[0])
}

/// Extract a number from a message string
fn extract_number_from_message(message: &str, keyword: &str) -> Option<String> {
    // Look for "keyword N" pattern
    let keyword_pos = message.find(keyword)?;
    let after_keyword = &message[keyword_pos + keyword.len()..];

    // Skip whitespace and "of"
    let trimmed = after_keyword.trim_start();
    let trimmed = if trimmed.starts_with("of") {
        trimmed[2..].trim_start()
    } else {
        trimmed
    };

    // Extract digits
    let digits: String = trimmed.chars().take_while(|c| c.is_ascii_digit()).collect();

    if digits.is_empty() {
        None
    } else {
        Some(digits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_error_kind_span() {
        let span = Span(10..20);
        let kind = ParseErrorKind::UnexpectedToken {
            expected: "expression".to_string(),
            found: "comma".to_string(),
            span,
        };
        assert_eq!(kind.span(), span);
    }

    #[test]
    fn test_parse_error_to_diagnostic() {
        let error = ParseError::new(ParseErrorKind::UnexpectedToken {
            expected: "expression".to_string(),
            found: "comma".to_string(),
            span: Span(10..20),
        });

        let diagnostic = error.to_diagnostic();
        assert_eq!(diagnostic.severity, Severity::Error);
        assert!(diagnostic.message.contains("Expected expression"));
        assert!(diagnostic.message.contains("found comma"));
        assert_eq!(diagnostic.code, Some("P001".to_string()));
    }

    #[test]
    fn test_format_expected_rules() {
        let rules = vec![Rule::integer, Rule::float];
        let formatted = format_expected_rules(&rules);
        assert_eq!(formatted, "literal");
    }

    #[test]
    fn test_extract_number_from_message() {
        let message = "nesting depth 150 exceeds maximum of 100 levels";
        assert_eq!(
            extract_number_from_message(message, "depth"),
            Some("150".to_string())
        );
        assert_eq!(
            extract_number_from_message(message, "maximum"),
            Some("100".to_string())
        );
    }
}
