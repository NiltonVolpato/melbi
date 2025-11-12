//! Configuration options for the Melbi engine.

/// Configuration options for the Melbi engine runtime.
///
/// These options control resource limits and runtime behavior.
///
/// # Example
///
/// ```
/// use melbi_core::api::EngineOptions;
///
/// let options = EngineOptions {
///     max_depth: 500,
///     max_iterations: Some(10_000),
/// };
/// ```
#[derive(Debug, Clone)]
pub struct EngineOptions {
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

impl Default for EngineOptions {
    fn default() -> Self {
        Self {
            max_depth: 1000,
            max_iterations: None,
        }
    }
}
