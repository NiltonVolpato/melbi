mod impls;
mod traits;

use impls::{HeapCtx, RefCtx, RefHandle, ViewCtx, ViewHandle};
use traits::{Context, ExprKind, Folder, Span};

// --- Bridge: Comparing View vs Ref ---

// 1. Compare Handles: ViewHandle vs RefHandle
impl PartialEq<RefHandle> for ViewHandle {
    fn eq(&self, other: &RefHandle) -> bool {
        // Dereference both wrappers to get to the Kinds
        let view_kind: &ExprKind<ViewCtx> = &self.0;
        let ref_kind: &ExprKind<RefCtx> = other.0;

        // Compare the Kinds
        view_kind == ref_kind
    }
}

// 2. Compare Kinds: ExprKind<ViewCtx> vs ExprKind<RefCtx>
// This allows the recursive comparison to work
impl PartialEq<ExprKind<RefCtx>> for ExprKind<ViewCtx> {
    fn eq(&self, other: &ExprKind<RefCtx>) -> bool {
        match (self, other) {
            (ExprKind::Lit(a), ExprKind::Lit(b)) => a == b,
            (ExprKind::Add(l1, r1), ExprKind::Add(l2, r2)) => {
                // l1 is ViewHandle, l2 is RefHandle
                // This invokes the implementation above
                l1 == l2 && r1 == r2
            }
            _ => false,
        }
    }
}

fn main() {
    // 1. Create Real Tree
    let mut ctx = HeapCtx;
    let l = ctx.alloc_expr(Span(0, 0), ExprKind::Lit(1));
    let r = ctx.alloc_expr(Span(0, 0), ExprKind::Lit(2));
    let root = ctx.alloc_expr(Span(0, 0), ExprKind::Add(l, r));

    // 2. Fold to View
    let mut view_ctx = ViewCtx;
    let mut folder = Folder::new(&ctx, &mut view_ctx);
    let view_root: ViewHandle = folder.fold_expr(&root);

    // 3. Construct Reference Tree for Test
    // We need to construct the static references.
    // In a real test macro, this would be hidden.
    // let lit_1 = &ExprKind::Lit(1);
    // let lit_2 = &ExprKind::Lit(2);
    // let root_ref = &ExprKind::<ViewCtx>::Add(RefHandle(lit_1), RefHandle(lit_2));

    // 4. Assert
    // view_root is ViewHandle
    // root_ref is &ExprKind<RefCtx>
    // We need to compare ViewHandle with RefHandle(root_ref) OR unwrap view_root.

    // Let's unwrap view_root to match your desired syntax: assert_eq!(view, &Expr...)
    let view_kind: &ExprKind<ViewCtx> = &view_root.0;
    println!("{:?}", view_kind);
}
