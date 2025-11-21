use std::fmt::Debug;

/// The "Driver" trait.
/// Defines the specific storage strategy (Box, Arena, etc)
/// and the specific types (Kind, Data) for a tree.
pub trait TreeBuilder: Sized {
    type TreeData;
    type TreeKind;

    /// The pointer to a node.
    /// Examples: Box<TreeNode<Self>>, u32, &'a TreeNode<Self>
    type Handle: Clone + Debug + PartialEq + Eq;

    /// Construct a handle from data and kind.
    fn build(&self, node: TreeNode<Self>) -> Self::Handle;

    /// Dereference a handle to get the node's contents.
    fn node(handle: &Self::Handle) -> &TreeNode<Self>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tree<B: TreeBuilder>(B::Handle);

impl<B: TreeBuilder> Tree<B> {
    pub fn new(builder: &B, node: TreeNode<B>) -> Self {
        Self(builder.build(node))
    }

    pub fn handle(&self) -> &B::Handle {
        &self.0
    }

    pub fn node(&self) -> &TreeNode<B> {
        B::node(self.handle())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeNode<B: TreeBuilder>(B::TreeData, B::TreeKind);

impl<B: TreeBuilder> TreeNode<B> {
    pub fn new(data: B::TreeData, kind: B::TreeKind) -> Self {
        Self(data, kind)
    }

    pub fn alloc(self, builder: &B) -> Tree<B> {
        Tree::new(builder, self)
    }

    pub fn data(&self) -> &B::TreeData {
        &self.0
    }

    pub fn kind(&self) -> &B::TreeKind {
        &self.1
    }
}

/// The Visitation Trait.
/// Implemented by the User's Kind (e.g. Expr).
/// Allows transforming the Kind from an Input Builder to an Output Builder.
pub trait Fold<In: TreeBuilder, Out: TreeBuilder> {
    fn fold(&self, input: &In, output: &Out) -> Out::TreeKind;
}

#[cfg(test)]
mod arena_builder_test;

#[cfg(test)]
mod heap_builder_test;
