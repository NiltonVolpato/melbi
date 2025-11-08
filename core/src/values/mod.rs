pub mod dynamic;
pub mod from_raw;
pub mod function;
pub mod raw;
pub mod typed;
pub use from_raw::TypeError;
pub use function::{FunctionData, NativeFn};
pub use raw::{ArrayData, RawValue};
pub use typed::{Array, Bridge, RawConvertible, Str};

#[cfg(test)]
mod display_test;
#[cfg(test)]
mod dynamic_test;
#[cfg(test)]
mod function_test;
#[cfg(test)]
mod raw_test;
#[cfg(test)]
mod value_test;
