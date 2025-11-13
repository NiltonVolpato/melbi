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
/// These options control resource limits and runtime behavior during evaluation.
///
/// # Example
///
/// ```
/// use melbi_core::api::ExecutionOptions;
///
/// let options = ExecutionOptions {
///     max_depth: 500,
///     max_iterations: Some(10_000),
/// };
/// ```
#[derive(Debug, Clone)]
pub struct ExecutionOptions {
    /// Maximum evaluation stack depth (for recursion protection).
    ///
    /// Default: 1000
    pub max_depth: usize,

    /// Maximum number of iterations in loops (if Some).
    ///
    /// Set to `None` for unlimited iterations (be careful with untrusted code!).
    ///
    /// Default: None
    pub max_iterations: Option<usize>,
}

impl ExecutionOptions {
    /// Merge this options with another, taking field-by-field values from `other`.
    ///
    /// All fields from `other` override the corresponding fields in `self`.
    pub fn merge(&self, other: &ExecutionOptions) -> Self {
        Self {
            max_depth: other.max_depth,
            max_iterations: other.max_iterations,
        }
    }
}

impl Default for ExecutionOptions {
    fn default() -> Self {
        Self {
            max_depth: 1000,
            max_iterations: None,
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
///         max_depth: 500,
///         max_iterations: Some(10_000),
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
