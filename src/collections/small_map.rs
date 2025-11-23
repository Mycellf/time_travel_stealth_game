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
