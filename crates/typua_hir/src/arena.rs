use std::marker::PhantomData;

#[derive(Clone, Copy, PartialEq)]
pub struct Idx<T> {
    raw: u32,
    _phantom: PhantomData<fn() -> T>,
}

impl<T> Idx<T> {
    pub fn new(raw: u32) -> Self {
        Self {
            raw,
            _phantom: PhantomData,
        }
    }
    pub fn raw(&self) -> u32 {
        self.raw
    }
}

#[derive(Debug)]
pub struct Arena<T> {
    data: Vec<T>,
}

impl<T> Arena<T> {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }
    pub fn alloc(&mut self, value: T) -> Idx<T> {
        let idx = Idx {
            raw: self.data.len() as u32,
            _phantom: PhantomData,
        };
        self.data.push(value);
        idx
    }
    pub fn len(&self) -> usize {
        self.data.len()
    }
}

impl<T> std::ops::Index<Idx<T>> for Arena<T> {
    type Output = T;
    fn index(&self, idx: Idx<T>) -> &T {
        &self.data[idx.raw as usize]
    }
}

impl<T> std::ops::IndexMut<Idx<T>> for Arena<T> {
    fn index_mut(&mut self, idx: Idx<T>) -> &mut T {
        &mut self.data[idx.raw as usize]
    }
}
