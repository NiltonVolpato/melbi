use crate::traits::ValueBuilder;

pub trait ValueView<VB: ValueBuilder>: Sized {
    fn ty(&self) -> VB::Ty;

    // Primitives: Return standard Rust types
    fn as_int(&self) -> Option<i64>;
    fn as_bool(&self) -> Option<bool>;

    // Complex Types: Return the associated types from the System
    fn as_array(&self) -> Option<VB::Array>;

    // fn as_map(&self) -> Option<S::Map>;
    // fn as_string(&self) -> Option<S::String>;
}
