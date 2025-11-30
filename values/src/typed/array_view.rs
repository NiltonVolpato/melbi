pub trait ArrayView<E> {
    // fn new<VB: ValueBuilder>(builder: &VB, values: &[E]) -> Self
    // where
    //     E: Copy;

    // fn from_iter<VB: ValueBuilder>(builder: &VB, values: impl IntoIterator<Item = E>) -> Self;

    fn len(&self) -> usize;

    fn get(&self, index: usize) -> Option<E>;

    // fn get_unchecked(&self, index: usize) -> E;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
