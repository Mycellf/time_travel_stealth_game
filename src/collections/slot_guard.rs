use std::{
    fmt::Debug,
    marker::PhantomData,
    ops::{Index, IndexMut},
};

use slotmap::SlotMap;

pub type GuardedSlotMap<'a, K, V> = SlotGuard<'a, SlotMap<K, V>, K, V>;

/// HACK: This is probably unsound if `K` doesn't implement `Eq` correctly
#[derive(Debug)]
pub struct SlotGuard<'a, T, K, V> {
    collection: &'a mut T,
    protected_slot: K,
    _phantom: PhantomData<&'a mut V>,
}

impl<'a, K, V, T> SlotGuard<'a, T, K, V> {
    pub fn new(collection: &'a mut T, protected_slot: K) -> (&'a mut V, Self)
    where
        K: Clone,
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

    pub fn iter(&'a self) -> impl Iterator<Item = (K, &'a V)>
    where
        K: Eq,
        &'a T: IntoIterator<Item = (K, &'a V)>,
    {
        self.collection
            .into_iter()
            .filter(|(slot, _)| *slot != self.protected_slot)
    }

    pub fn iter_mut(&'a mut self) -> impl Iterator<Item = (K, &'a mut V)>
    where
        K: Eq,
        &'a mut T: IntoIterator<Item = (K, &'a mut V)>,
    {
        self.collection
            .into_iter()
            .filter(|(slot, _)| *slot != self.protected_slot)
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
