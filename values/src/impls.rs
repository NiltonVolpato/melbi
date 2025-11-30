// impls.rs

use crate::{dynamic::ValueView, raw::RawValue, ty::TyKind, typed::ArrayView};
use alloc::rc::Rc;

use crate::traits::{Value, ValueBuilder};

#[derive(Debug, Clone, PartialEq, Eq)]
struct HeapBuilder;

impl ValueBuilder for HeapBuilder {
    type Ty = Rc<TyKind>;

    type Raw = RawValue;

    type ValueHandle = Rc<Value<Self>>;

    type Value = Value<HeapBuilder>;
    type Array = Array;

    fn alloc(&self, value: Value<Self>) -> Self::ValueHandle {
        Rc::new(value)
    }
}

impl ValueView<HeapBuilder> for Value<HeapBuilder> {
    fn ty(&self) -> Rc<TyKind> {
        self.ty().clone()
    }

    fn as_int(&self) -> Option<i64> {
        let TyKind::Int = self.ty().as_ref() else {
            return None;
        };
        Some(self.raw().as_int_unchecked())
    }

    fn as_bool(&self) -> Option<bool> {
        let TyKind::Bool = self.ty().as_ref() else {
            return None;
        };
        Some(self.raw().as_bool_unchecked())
    }

    // Complex Types: Return the associated types from the System
    fn as_array(&self) -> Option<Array> {
        let TyKind::Array(element_type) = self.ty().as_ref() else {
            return None;
        };
        Some(Array(self.raw().clone(), element_type.clone()))
    }
}

struct Array(
    <HeapBuilder as ValueBuilder>::Raw,
    <HeapBuilder as ValueBuilder>::Ty,
);

impl ArrayView<Value<HeapBuilder>> for Array {
    fn len(&self) -> usize {
        self.0.as_array_unchecked().len()
    }

    fn get(&self, index: usize) -> Option<Value<HeapBuilder>> {
        let element = self.0.as_array_unchecked().get(index)?;
        Some(Value::new(self.1.clone(), element.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ty::TyKind;

    #[test]
    fn test_heap_builder() {
        let builder = HeapBuilder;

        let v = Value::new(TyKind::Int.handle(), RawValue::new_int(42)).alloc(&builder);
        let value = v.value();

        assert_eq!(value.as_bool(), None);
        assert_eq!(value.as_int(), Some(42));
    }
}
