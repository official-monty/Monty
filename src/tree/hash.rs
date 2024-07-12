#[derive(Clone, Copy, Debug)]
pub struct HashEntry {
    pub hash: u64,
    pub visits: i32,
    pub q: f32,
}

impl Default for HashEntry {
    fn default() -> Self {
        Self {
            hash: 0,
            visits: 0,
            q: 0.0,
        }
    }
}

pub struct HashTable {
    table: Vec<HashEntry>,
}

impl HashTable {
    pub fn new(size: usize) -> Self {
        Self {
            table: vec![HashEntry::default(); size],
        }
    }

    pub fn clear(&mut self) {
        for entry in &mut self.table {
            *entry = HashEntry::default();
        }
    }

    pub fn fetch(&self, hash: u64) -> &HashEntry {
        let idx = hash % (self.table.len() as u64);
        &self.table[idx as usize]
    }

    pub fn get(&self, hash: u64) -> Option<HashEntry> {
        let entry = self.fetch(hash);

        if entry.hash == hash {
            Some(*entry)
        } else {
            None
        }
    }

    pub fn push(&mut self, hash: u64, visits: i32, q: f32) {
        let idx = hash % (self.table.len() as u64);
        self.table[idx as usize] = HashEntry { hash, visits, q };
    }
}
