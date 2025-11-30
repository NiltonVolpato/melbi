// traits/builder.rs

use core::fmt::Debug;

use crate::{dynamic::ValueView, ty::TyKind, typed::ArrayView};

pub trait ValueBuilder: Sized + Clone {
    // The type of the value being built.
    // Example: `&'a TyKind<'a>`, `Rc<TyKind>`.
    type Ty: AsRef<TyKind> + Clone;
    // The raw representation of the value.
    // Example: `RawValue` (untagged union), or an enum.
    type Raw;
    // The handle to the value.
    // Example: `Value<Self>`, `Rc<Value<Self>>`
    type ValueHandle: AsRef<Value<Self>> + Clone + Debug;

    type Value: ValueView<Self>;
    //type Array: ArrayView<Self>;
    type Array: ArrayView<Value<Self>>;

    fn alloc(&self, value: Value<Self>) -> Self::ValueHandle;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Val<VB: ValueBuilder>(VB::ValueHandle);

impl<VB: ValueBuilder> Val<VB> {
    pub fn new(builder: &VB, value: Value<VB>) -> Self {
        Val(builder.alloc(value))
    }

    pub fn value(&self) -> &Value<VB> {
        self.0.as_ref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Value<VB: ValueBuilder> {
    raw: VB::Raw,
    ty: VB::Ty,
}

impl<VB: ValueBuilder> Value<VB> {
    pub fn new(ty: VB::Ty, raw: VB::Raw) -> Self {
        Self { raw, ty }
    }

    pub fn raw(&self) -> &VB::Raw {
        &self.raw
    }

    pub fn ty(&self) -> &VB::Ty {
        &self.ty
    }

    pub fn alloc(self, builder: &VB) -> Val<VB> {
        Val::new(builder, self)
    }
}
