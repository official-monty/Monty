use std::sync::atomic::{AtomicU32, Ordering};

#[derive(Clone, Copy, Debug, Default)]
pub struct HashEntry {
    hash: u16,
    q: u16,
}

impl HashEntry {
    pub fn q(&self) -> f32 {
        f32::from(self.q) / f32::from(u16::MAX)
    }
}

#[derive(Default)]
struct HashEntryInternal(AtomicU32);

impl Clone for HashEntryInternal {
    fn clone(&self) -> Self {
        Self(AtomicU32::new(self.0.load(Ordering::Relaxed)))
    }
}

impl From<&HashEntryInternal> for HashEntry {
    fn from(value: &HashEntryInternal) -> Self {
        unsafe { std::mem::transmute(value.0.load(Ordering::Relaxed)) }
    }
}

impl From<HashEntry> for u32 {
    fn from(value: HashEntry) -> Self {
        unsafe { std::mem::transmute(value) }
    }
}

pub struct HashTable {
    table: Vec<HashEntryInternal>,
}

impl HashTable {
    pub fn new(size: usize) -> Self {
        Self {
            table: vec![HashEntryInternal::default(); size],
        }
    }

    pub fn clear(&mut self) {
        for entry in &mut self.table {
            *entry = HashEntryInternal::default();
        }
    }

    pub fn fetch(&self, hash: u64) -> HashEntry {
        let idx = hash % (self.table.len() as u64);
        HashEntry::from(&self.table[idx as usize])
    }

    fn key(hash: u64) -> u16 {
        (hash >> 48) as u16
    }

    pub fn get(&self, hash: u64) -> Option<HashEntry> {
        let entry = self.fetch(hash);

        if entry.hash == Self::key(hash) {
            Some(entry)
        } else {
            None
        }
    }

    pub fn push(&self, hash: u64, q: f32) {
        let idx = hash % (self.table.len() as u64);

        let entry = HashEntry {
            hash: Self::key(hash),
            q: (q * f32::from(u16::MAX)) as u16,
        };

        self.table[idx as usize]
            .0
            .store(u32::from(entry), Ordering::Relaxed)
    }
}
