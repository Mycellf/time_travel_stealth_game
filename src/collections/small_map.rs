use std::{
    marker::PhantomData,
    ops::{Index, IndexMut},
};

#[derive(Clone, Debug)]
pub struct SmallMap<K, V> {
    data: Vec<Option<V>>,
    first_free: usize,
    len: usize,
    _phantom: PhantomData<K>,
}

impl<K, V> Default for SmallMap<K, V>
where
    K: TryFrom<usize> + Into<usize>,
{
    fn default() -> Self {
        Self {
            data: Vec::new(),
            first_free: 0,
            len: 0,
            _phantom: PhantomData,
        }
    }
}

impl<K, V> SmallMap<K, V>
where
    K: TryFrom<usize> + Into<usize>,
{
    pub fn insert(&mut self, value: V) -> K {
        while self.first_free >= self.data.len() {
            self.data.push(None);
        }

        let key = self.first_free.try_into().ok().expect("Too many elements");
        self.data[self.first_free] = Some(value);

        while let Some(Some(_)) = self.data.get(self.first_free) {
            self.first_free += 1;
        }

        self.len += 1;

        key
    }

    pub fn get(&self, key: K) -> Option<&V> {
        self.data.get(key.into())?.as_ref()
    }

    pub fn get_mut(&mut self, key: K) -> Option<&mut V> {
        self.data.get_mut(key.into())?.as_mut()
    }

    /// Indexing with this key after this operation may result in unexpected behavior
    pub fn remove(&mut self, key: K) -> Option<V> {
        let index = key.into();

        let value = self.data.get_mut(index)?.take();

        if value.is_some() {
            self.first_free = self.first_free.min(index);
            self.len -= 1;
        }

        value
    }
}

impl<K, V> Index<K> for SmallMap<K, V>
where
    K: TryFrom<usize> + Into<usize>,
{
    type Output = V;

    fn index(&self, index: K) -> &Self::Output {
        self.get(index).unwrap()
    }
}

impl<K, V> IndexMut<K> for SmallMap<K, V>
where
    K: TryFrom<usize> + Into<usize>,
{
    fn index_mut(&mut self, index: K) -> &mut Self::Output {
        self.get_mut(index).unwrap()
    }
}

/// CREDIT: Adapted from slotmap's `new_key_type`.
#[macro_export]
macro_rules! new_small_key_type {
    ( $(#[$outer:meta])* $vis:vis struct $name:ident($inner:ty); $($rest:tt)* ) => {
        $(#[$outer])*
        #[derive(Copy, Clone, Default,
                 Eq, PartialEq, Ord, PartialOrd,
                 Hash, Debug)]
        #[repr(transparent)]
        $vis struct $name($inner);

        // Make it a bit harder to accidentally misuse the macro
        const _: u32 = <$inner>::BITS;

        impl From<$name> for usize {
            fn from(value: $name) -> Self {
                value.0.into()
            }
        }

        impl TryFrom<usize> for $name {
            type Error = std::num::TryFromIntError;

            fn try_from(value: usize) -> Result<Self, Self::Error> {
                Ok($name(value.try_into()?))
            }
        }

        $crate::new_small_key_type!($($rest)*);
    };

    () => {}
}

new_small_key_type! {
    pub struct DefaultU8Key(u8);
    pub struct DefaultU16Key(u16);
}
