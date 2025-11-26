use crate::*;
use bumpalo::Bump;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span(pub usize, pub usize);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr<B: TreeBuilder> {
    Lit(i32),
    // We now use Tree<B> instead of B::Handle directly.
    // This is much cleaner for the user.
    Add(Tree<B>, Tree<B>),
}

// Define a builder that owns the memory
#[derive(Debug, Clone)]
pub struct ArenaBuilder<'arena> {
    arena: &'arena Bump,
}

impl PartialEq for ArenaBuilder<'_> {
    fn eq(&self, other: &Self) -> bool {
        // Two builders are "equal" if they point to the same memory arena.
        std::ptr::eq(self.arena, other.arena)
    }
}

impl Eq for ArenaBuilder<'_> {}

impl<'arena> ArenaBuilder<'arena> {
    pub fn new(arena: &'arena Bump) -> Self {
        Self { arena }
    }
}

impl<'arena> TreeBuilder for ArenaBuilder<'arena> {
    type TreeData = Span;
    type TreeKind = Expr<Self>;
    type Handle = &'arena TreeNode<Self>;

    fn build(&self, node: TreeNode<Self>) -> Self::Handle {
        self.arena.alloc(node)
    }

    fn node(handle: &Self::Handle) -> &TreeNode<Self> {
        handle
    }
}

fn fold_tree<In, Out>(input: &In, output: &Out, tree: &Tree<In>) -> Tree<Out>
where
    In: TreeBuilder<TreeData = Span>,
    Out: TreeBuilder<TreeData = Span, TreeKind = Expr<Out>>,
    In::TreeKind: Fold<In, Out>,
{
    let node = tree.node();
    TreeNode(*node.data(), node.kind().fold(input, output)).alloc(output)
}

impl<'input, 'output> Fold<ArenaBuilder<'input>, ArenaBuilder<'output>>
    for Expr<ArenaBuilder<'input>>
{
    fn fold(
        &self,
        input: &ArenaBuilder<'input>,
        output: &ArenaBuilder<'output>,
    ) -> Expr<ArenaBuilder<'output>> {
        match self {
            Expr::Lit(x) => Expr::Lit(*x),
            Expr::Add(l, r) => {
                // l and r are of type Tree<HeapBuilder>
                let new_l = fold_tree(input, output, l);
                let new_r = fold_tree(input, output, r);
                Expr::Add(new_l, new_r)
            }
        }
    }
}

#[test]
fn test_arena_builder() {
    // 1. Setup: We need TWO arenas.
    // One for the source (read-only), one for the destination (write-only).
    let old_bump = Bump::new();
    let old_arena = ArenaBuilder::new(&old_bump);
    let new_bump = Bump::new();
    let new_arena = ArenaBuilder::new(&new_bump);

    // 2. Populate the Old Arena (The "Parser" phase)
    let l1 = TreeNode(Span(0, 1), Expr::Lit(10)).alloc(&old_arena);
    let l2 = TreeNode(Span(2, 3), Expr::Lit(20)).alloc(&old_arena);
    let root = TreeNode(Span(0, 3), Expr::Add(l1, l2)).alloc(&old_arena);

    // 3. Fold: Transform from Old -> New (The "Lowering" phase)
    // We resolve from 'old_arena' and build into 'new_arena'.

    // We have to manually start the fold logic here
    let node = root.node();

    // Notice: input is &old_arena, output is &mut new_arena
    let new_kind = node.kind().fold(&old_arena, &new_arena);

    // The result is a handle valid ONLY in new_arena
    let new_root = TreeNode(*node.data(), new_kind).alloc(&new_arena);

    // 4. Verify
    // We cannot compare 'root' and 'new_root' directly because they are
    // indices (usize) referring to DIFFERENT arrays.
    // root might be index 2 in old_arena. new_root might be index 0 in new_arena.

    let old_node = root.node();
    let new_node = new_root.node();

    assert_eq!(*old_node.data(), *new_node.data());
}
