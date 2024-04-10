#[derive(Debug)]
pub struct HashTable {
    inner: Vec<i32>,
}

impl std::ops::Index<u64> for HashTable {
    type Output = i32;

    fn index(&self, index: u64) -> &Self::Output {
        let len = self.inner.len();
        &self.inner[index as usize % len]
    }
}

impl std::ops::IndexMut<u64> for HashTable {
    fn index_mut(&mut self, index: u64) -> &mut Self::Output {
        let len = self.inner.len();
        &mut self.inner[index as usize % len]
    }
}

impl HashTable {
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            inner: vec![0; cap],
        }
    }

    pub fn clear(&mut self) {
        for entry in &mut self.inner {
            *entry = 0;
        }
    }
}
