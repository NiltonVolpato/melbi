pub mod from_raw;
pub mod raw;
pub mod typed;
pub mod value;
pub use from_raw::TypeError;
pub use raw::{ArrayData, RawValue};
pub use typed::{Array, Bridge, RawConvertible};
pub use value::{DynamicArray, Value};

#[cfg(test)]
mod display_test;
#[cfg(test)]
mod raw_test;
#[cfg(test)]
mod value_test;
