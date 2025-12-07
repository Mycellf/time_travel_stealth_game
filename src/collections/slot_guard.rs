use std::{
    fmt::Debug,
    marker::PhantomData,
    ops::{Index, IndexMut},
};

/// HACK: This is probably unsafe if `K` doesn't implement `Eq` correctly
#[derive(Debug)]
pub struct SlotGuard<'a, T, K, V> {
    collection: &'a mut T,
    protected_slot: K,
    _phantom: PhantomData<&'a mut V>,
}

impl<'a, K, V, T> SlotGuard<'a, T, K, V> {
    pub fn new(collection: &'a mut T, protected_slot: K) -> (&'a mut V, Self)
    where
        K: Clone + Eq + Debug,
        T: IndexMut<K, Output = V>,
    {
        let value = &mut collection[protected_slot.clone()];

        // SAFETY: The returned reference should only live as long as Self
        let value = unsafe { &mut *(value as *mut V) };

        (
            value,
            Self {
                collection,
                protected_slot,
                _phantom: PhantomData,
            },
        )
    }
}

impl<'a, K, V, T> Index<K> for SlotGuard<'a, T, K, V>
where
    K: Eq + Debug,
    T: Index<K, Output = V>,
{
    type Output = V;

    fn index(&self, index: K) -> &Self::Output {
        if index == self.protected_slot {
            panic!("Slot {index:?} is protected!");
        }

        &self.collection[index]
    }
}

impl<'a, K, V, T> IndexMut<K> for SlotGuard<'a, T, K, V>
where
    K: Eq + Debug,
    T: IndexMut<K, Output = V>,
{
    fn index_mut(&mut self, index: K) -> &mut Self::Output {
        if index == self.protected_slot {
            panic!("Slot {index:?} is protected!");
        }

        &mut self.collection[index]
    }
}
