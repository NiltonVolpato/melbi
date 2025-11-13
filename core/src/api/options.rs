//! Configuration options for the Melbi engine.

/// Configuration options for compilation.
///
/// These options control compile-time behavior and optimizations.
///
/// # Example
///
/// ```
/// use melbi_core::api::CompilationOptions;
///
/// let options = CompilationOptions::default();
/// ```
#[derive(Debug, Clone)]
pub struct CompilationOptions {
    // Future: optimization level, type checking strictness, etc.
}

impl CompilationOptions {
    /// Merge this options with another, preferring non-default values from `other`.
    ///
    /// Currently this is a no-op since CompilationOptions has no fields,
    /// but the pattern is established for future fields.
    pub fn merge(&self, _other: &CompilationOptions) -> Self {
        Self {}
    }
}

impl Default for CompilationOptions {
    fn default() -> Self {
        Self {}
    }
}

/// Configuration options for expression execution.
///
/// All fields are `Option` to support partial specification and merging.
/// `None` means "not specified, use default". When merging, values from
/// the override take precedence over the base.
///
/// # Example
///
/// ```
/// use melbi_core::api::ExecutionOptions;
///
/// // Specify only max_depth, use default for max_iterations
/// let options = ExecutionOptions {
///     max_depth: Some(500),
///     max_iterations: None,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct ExecutionOptions {
    /// Maximum evaluation stack depth (for recursion protection).
    ///
    /// `None` means not specified (use default: 1000).
    pub max_depth: Option<usize>,

    /// Maximum number of iterations in loops.
    ///
    /// - `None` = not specified, use default (unlimited)
    /// - `Some(None)` = explicitly unlimited
    /// - `Some(Some(n))` = explicitly limited to n iterations
    ///
    /// TODO: Consider using a custom enum like `IterationLimit { Unlimited, Limited(usize) }`
    /// instead of nested Option for better ergonomics.
    pub max_iterations: Option<Option<usize>>,
}

impl ExecutionOptions {
    /// Merge this options with another, preferring values from `other` when specified.
    ///
    /// For each field, if `other` specifies a value (is `Some`), use it.
    /// Otherwise, keep the value from `self`.
    pub fn merge(&self, other: &ExecutionOptions) -> Self {
        Self {
            max_depth: other.max_depth.or(self.max_depth),
            max_iterations: other.max_iterations.or(self.max_iterations),
        }
    }

    /// Get the effective max_depth value, using the default if not specified.
    pub(crate) fn max_depth_or_default(&self) -> usize {
        self.max_depth.unwrap_or(1000)
    }

    /// Get the effective max_iterations value, using the default if not specified.
    pub(crate) fn max_iterations_or_default(&self) -> Option<usize> {
        self.max_iterations.unwrap_or(None)
    }
}

impl Default for ExecutionOptions {
    fn default() -> Self {
        Self {
            max_depth: Some(1000),
            max_iterations: Some(None), // Unlimited by default
        }
    }
}

/// Configuration options for the Melbi engine.
///
/// These options set the defaults for compilation and execution,
/// which can be overridden on a per-call basis.
///
/// # Example
///
/// ```
/// use melbi_core::api::{EngineOptions, CompilationOptions, ExecutionOptions};
///
/// let options = EngineOptions {
///     default_compilation_options: CompilationOptions::default(),
///     default_execution_options: ExecutionOptions {
///         max_depth: Some(500),
///         max_iterations: Some(Some(10_000)),
///     },
/// };
/// ```
#[derive(Debug, Clone)]
pub struct EngineOptions {
    /// Default options for compilation.
    ///
    /// These can be overridden when calling `Engine::compile()`.
    pub default_compilation_options: CompilationOptions,

    /// Default options for execution.
    ///
    /// These can be overridden when calling `CompiledExpression::run()`.
    pub default_execution_options: ExecutionOptions,
}

impl Default for EngineOptions {
    fn default() -> Self {
        Self {
            default_compilation_options: CompilationOptions::default(),
            default_execution_options: ExecutionOptions::default(),
        }
    }
}
