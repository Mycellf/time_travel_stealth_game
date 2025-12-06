use std::{
    marker::PhantomData,
    ops::{Index, IndexMut},
};

use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct SmallMap<K, V> {
    data: Vec<Option<V>>,
    first_free: usize,
    len: usize,
    _phantom: PhantomData<K>,
}

pub trait Key: Sized {
    fn try_from_usize(value: usize) -> Option<Self>;

    fn into_usize(self) -> usize;
}

impl<K, V> Default for SmallMap<K, V>
where
    K: Key,
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
    K: Key,
{
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn insert(&mut self, value: V) -> K {
        while self.first_free >= self.data.len() {
            self.data.push(None);
        }

        let key = K::try_from_usize(self.first_free).expect("Too many elements");
        self.data[self.first_free] = Some(value);

        while let Some(Some(_)) = self.data.get(self.first_free) {
            self.first_free += 1;
        }

        self.len += 1;

        key
    }

    pub fn get(&self, key: K) -> Option<&V> {
        self.data.get(key.into_usize())?.as_ref()
    }

    pub fn get_mut(&mut self, key: K) -> Option<&mut V> {
        self.data.get_mut(key.into_usize())?.as_mut()
    }

    /// Indexing with this key after this operation may result in unexpected behavior
    pub fn remove(&mut self, key: K) -> Option<V> {
        let index = key.into_usize();

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
    K: Key,
{
    type Output = V;

    fn index(&self, index: K) -> &Self::Output {
        self.get(index).unwrap()
    }
}

impl<K, V> IndexMut<K> for SmallMap<K, V>
where
    K: Key,
{
    fn index_mut(&mut self, index: K) -> &mut Self::Output {
        self.get_mut(index).unwrap()
    }
}

pub struct IntoIter<K: Key, V> {
    inner: Vec<Option<V>>,
    _phantom: PhantomData<K>,
}

impl<K: Key, V> IntoIterator for SmallMap<K, V> {
    type Item = V;

    type IntoIter = IntoIter<K, V>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            inner: self.data,
            _phantom: PhantomData,
        }
    }
}

impl<K: Key, V> Iterator for IntoIter<K, V> {
    type Item = V;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(element) = self.inner.pop()? {
                return Some(element);
            }
        }
    }
}

pub struct Iter<'a, K: Key, V> {
    inner: &'a [Option<V>],
    _phantom: PhantomData<K>,
}

impl<K: Key, V> SmallMap<K, V> {
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter {
            inner: &self.data,
            _phantom: PhantomData,
        }
    }
}

impl<'a, K: Key, V> IntoIterator for &'a SmallMap<K, V> {
    type Item = (K, &'a V);

    type IntoIter = Iter<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, K: Key, V> Iterator for Iter<'a, K, V> {
    type Item = (K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(element) = self.inner.split_off_last()? {
                let key = K::try_from_usize(self.inner.len()).unwrap();
                return Some((key, element));
            }
        }
    }
}

pub struct IterMut<'a, K: Key, V> {
    inner: &'a mut [Option<V>],
    _phantom: PhantomData<K>,
}

impl<K: Key, V> SmallMap<K, V> {
    pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
        IterMut {
            inner: &mut self.data,
            _phantom: PhantomData,
        }
    }
}

impl<'a, K: Key, V> IntoIterator for &'a mut SmallMap<K, V> {
    type Item = (K, &'a mut V);

    type IntoIter = IterMut<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<'a, K: Key, V> Iterator for IterMut<'a, K, V> {
    type Item = (K, &'a mut V);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(element) = self.inner.split_off_last_mut()? {
                let key = K::try_from_usize(self.inner.len()).unwrap();
                return Some((key, element));
            }
        }
    }
}

pub struct Keys<'a, K: Key, V> {
    inner: &'a [Option<V>],
    _phantom: PhantomData<K>,
}

impl<K: Key, V> SmallMap<K, V> {
    pub fn keys(&self) -> Keys<'_, K, V> {
        Keys {
            inner: &self.data,
            _phantom: PhantomData,
        }
    }
}

impl<'a, K: Key, V> Iterator for Keys<'a, K, V> {
    type Item = K;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(_) = self.inner.split_off_last()? {
                let key = K::try_from_usize(self.inner.len()).unwrap();
                return Some(key);
            }
        }
    }
}

/// CREDIT: Adapted from slotmap's `new_key_type`.
#[macro_export]
macro_rules! new_small_key_type {
    ( $(#[$outer:meta])* $vis:vis struct $name:ident($inner:ty); $($rest:tt)* ) => {
        $(#[$outer])*
        #[derive(Copy, Clone, serde::Serialize, serde::Deserialize,
                 Eq, PartialEq, Ord, PartialOrd,
                 Hash, Debug)]
        #[repr(transparent)]
        $vis struct $name(std::num::NonZero<$inner>);

        // Make it a bit harder to accidentally misuse the macro
        const _: () = assert!(<$inner>::BITS <= u32::BITS);

        impl $crate::collections::small_map::Key for $name {
            fn try_from_usize(value: usize) -> Option<Self> {
                // If adding 1 overflows, it will result in 0, which will return an error
                Some(Self(std::num::NonZero::new(<$inner>::try_from(value).ok()?.wrapping_add(1))?))
            }

            fn into_usize(self) -> usize {
                (self.0.get() - 1) as usize
            }
        }

        $crate::new_small_key_type!($($rest)*);
    };

    () => {}
}

new_small_key_type! {
    pub struct DefaultU8Key(u8);
    pub struct DefaultU16Key(u16);
    pub struct DefaultU32Key(u32);
}
