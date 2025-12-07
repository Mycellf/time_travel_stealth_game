use std::{mem, num::NonZero, ops::Range};

pub type FrameIndex = usize;

#[derive(Clone, Debug)]
pub struct EntityHistory<T> {
    data: Vec<Record<T>>,
}

impl<T> Default for EntityHistory<T> {
    fn default() -> Self {
        Self { data: Vec::new() }
    }
}

impl<T> EntityHistory<T> {
    pub fn get(&self, index: FrameIndex) -> Option<&T> {
        let record_index = match self
            .data
            .binary_search_by_key(&index, |record| record.start())
        {
            Ok(index) => index,
            Err(index) => index.wrapping_sub(1),
        };

        self.data.get(record_index)?.get(index)
    }

    pub fn try_insert(&mut self, index: FrameIndex, entry: T) -> Option<()>
    where
        T: PartialEq,
    {
        for (i, record) in self.data.iter().enumerate() {
            if record.finish() < index {
                continue;
            }

            if record.start() > index {
                self.data.insert(i, Record::new(index, entry));
                return Some(());
            }

            if record.finish() == index {
                if let Some(next) = self.data.get(i + 1)
                    && next.start() <= index
                {
                    return None;
                }

                if let Some(new_record) = self.data[i].extend(entry) {
                    self.data.insert(i + 1, new_record);
                }

                return Some(());
            } else if record.range().contains(&index) {
                return None;
            }
        }

        self.data.push(Record::new(index, entry));
        Some(())
    }
}

#[derive(Clone, Debug)]
enum Record<T> {
    Constant {
        start: FrameIndex,
        finish: NonZero<FrameIndex>,
        value: T,
    },
    Variable {
        start: FrameIndex,
        values: Vec<T>,
    },
}

impl<T> Record<T> {
    /// The number of repititions inside a variable record that will cause the creation of a
    /// constant record.
    const CONVERSION_THRESHOLD: usize = 5;

    fn new(start: FrameIndex, value: T) -> Self {
        Self::Constant {
            start,
            finish: NonZero::new(start + 1).unwrap(),
            value,
        }
    }

    fn start(&self) -> FrameIndex {
        match self {
            &Record::Constant { start, .. } | &Record::Variable { start, .. } => start,
        }
    }

    fn finish(&self) -> FrameIndex {
        match self {
            &Record::Constant { finish, .. } => finish.get(),
            &Record::Variable { start, ref values } => start + values.len(),
        }
    }

    fn range(&self) -> Range<FrameIndex> {
        self.start()..self.finish()
    }

    fn is_empty(&self) -> bool {
        self.range().is_empty()
    }

    fn get(&self, index: FrameIndex) -> Option<&T> {
        self.range().contains(&index).then(|| match self {
            Record::Constant { value, .. } => value,
            &Record::Variable { start, ref values } => &values[index - start],
        })
    }

    fn extend(&mut self, entry: T) -> Option<Record<T>>
    where
        T: PartialEq,
    {
        match self {
            Record::Constant {
                start,
                finish,
                value,
            } => {
                if *value == entry {
                    *finish = NonZero::new(finish.get() + 1).unwrap();
                    return None;
                }

                match finish.get() - *start {
                    0 => unreachable!(),
                    1 => {
                        let Record::Constant { start, value, .. } = mem::replace(
                            self,
                            // Dummy value
                            Record::Variable {
                                start: 0,
                                values: Vec::new(),
                            },
                        ) else {
                            unreachable!();
                        };

                        *self = Record::Variable {
                            start,
                            values: vec![value, entry],
                        };
                        None
                    }
                    _ => Some(Record::new(finish.get(), entry)),
                }
            }
            Record::Variable { start, values } => {
                if values.len() >= Self::CONVERSION_THRESHOLD {
                    let count = dbg!(values.iter().rev().take_while(|x| **x == entry).count());

                    if count + 1 >= Self::CONVERSION_THRESHOLD {
                        if values.len() == count {
                            *self = Record::Constant {
                                start: *start,
                                // Add 1 because we're adding another entry
                                finish: NonZero::new(*start + values.len() + 1).unwrap(),
                                value: entry,
                            };
                            return None;
                        } else {
                            values.drain(values.len() - count..);

                            let new_start = *start + values.len();

                            return Some(Record::Constant {
                                start: new_start,
                                // Add 1 because we're adding another entry
                                finish: NonZero::new(new_start + count + 1).unwrap(),
                                value: entry,
                            });
                        }
                    }
                }

                values.push(entry);
                None
            }
        }
    }
}
