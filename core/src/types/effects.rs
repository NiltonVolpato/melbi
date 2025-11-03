/// Represents the effects that an expression may have during evaluation.
///
/// Effects are tracked in Melbi's type system to enable safe evaluation,
/// better error messages, and compile-time optimizations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Effects {
    /// Whether the computation can fail at runtime.
    ///
    /// Operations that can error include:
    /// - Division by zero: `x / 0`
    /// - Array indexing out of bounds: `arr[100]`
    /// - Map key lookup on missing keys: `map[key]`
    /// - Some cast operations: `value as Type`
    /// - Certain function calls.
    ///
    /// Errors are denoted by `!` in type signatures (e.g., `Int!`).
    ///
    /// Errors must be handled before the top level using:
    /// - `otherwise`: `10 / a otherwise 0`
    ///
    /// The error effect propagates automatically through expressions:
    /// ```melbi
    /// 2 * ((a / b) + (c / d))  // Type: Int! (error propagates from both divisions)
    /// ```
    pub can_error: bool,

    /// Whether the computation depends on runtime context.
    ///
    /// An expression is impure if it:
    /// - Uses input data passed from the host environment
    /// - Performs I/O operations (file access, network, console)
    /// - Is non-deterministic (random numbers, current time)
    ///
    /// Impure expressions are denoted by `~` in type signatures (e.g., `String~`).
    ///
    /// # Why this matters
    ///
    /// Pure expressions (without `~`) can be **constant-folded** at compile time:
    /// ```melbi
    /// // This map is built once at compile time, not every evaluation:
    /// email.sender not in {"spam@example.com": true, "bad@actor.com": true}
    /// ```
    ///
    /// Impure expressions must be evaluated at runtime:
    /// ```melbi
    /// email.sender  // String~ (depends on input data)
    /// random()      // Float~ (non-deterministic)
    /// read_file(x)  // String~! (I/O + can fail)
    /// ```
    ///
    /// **The impure effect cannot be removed** - it's indelible. Once an expression
    /// depends on external context, that property propagates through all computations
    /// using it.
    ///
    /// TODO: This should be associated with data instead of computations directly.
    pub is_impure: bool,
}

impl Effects {
    /// No effects - a pure, total computation.
    ///
    /// Pure expressions can be:
    /// - Constant-folded at compile time
    /// - Cached indefinitely
    /// - Reordered or parallelized freely
    pub const TOTAL: Self = Effects {
        can_error: false,
        is_impure: false,
    };

    /// Can fail, but is otherwise pure.
    ///
    /// Example: `10 / x` where `x` is a compile-time constant.
    pub const ERROR: Self = Effects {
        can_error: true,
        is_impure: false,
    };

    /// Impure but cannot fail.
    ///
    /// Example: `print("hello")` - performs I/O but always succeeds.
    pub const IMPURE: Self = Effects {
        can_error: false,
        is_impure: true,
    };

    /// Both impure and can fail.
    ///
    /// Example: `read_file(path)` - performs I/O and might fail.
    pub const BOTH: Self = Effects {
        can_error: true,
        is_impure: true,
    };

    /// Combines two effect sets, taking the union of their effects.
    ///
    /// This is how effects propagate through expressions:
    /// ```
    /// # use melbi_core::types::Effects;
    /// let e1 = Effects::ERROR;  // !
    /// let e2 = Effects::IMPURE; // ~
    /// assert_eq!(e1.union(e2), Effects::BOTH); // ~!
    /// ```
    pub fn union(self, other: Self) -> Self {
        Effects {
            can_error: self.can_error || other.can_error,
            is_impure: self.is_impure || other.is_impure,
        }
    }

    /// Returns `true` if this is a pure, total computation.
    pub fn is_total(&self) -> bool {
        *self == Effects::TOTAL
    }

    /// Format the effect markers as a string.
    ///
    /// Returns:
    /// - `""` for total
    /// - `"!"` for error only
    /// - `"~"` for impure only
    /// - `"~!"` for both
    pub fn to_string(&self) -> &'static str {
        match (self.can_error, self.is_impure) {
            (false, false) => "",
            (true, false) => "!",
            (false, true) => "~",
            (true, true) => "!~",
        }
    }
}
