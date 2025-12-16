use std::sync::atomic::{AtomicU32, Ordering};

#[derive(Clone, Copy, Debug, Default)]
pub struct HashEntry {
    hash: u32,
    q: u32,
    d: u32,
    #[allow(dead_code)]
    visits: u32,
}

impl HashEntry {
    pub fn q(&self) -> f32 {
        self.q as f32 / u32::MAX as f32
    }

    pub fn d(&self) -> f32 {
        self.d as f32 / u32::MAX as f32
    }
}

#[derive(Default)]
struct HashEntryInternal {
    hash: AtomicU32,
    q: AtomicU32,
    d: AtomicU32,
    visits: AtomicU32,
}

impl Clone for HashEntryInternal {
    fn clone(&self) -> Self {
        Self {
            hash: AtomicU32::new(self.hash.load(Ordering::Relaxed)),
            q: AtomicU32::new(self.q.load(Ordering::Relaxed)),
            d: AtomicU32::new(self.d.load(Ordering::Relaxed)),
            visits: AtomicU32::new(self.visits.load(Ordering::Relaxed)),
        }
    }
}

pub struct HashTable {
    table: Vec<HashEntryInternal>,
}

impl HashTable {
    pub fn new(size: usize, _threads: usize) -> Self {
        let mut table = HashTable { table: Vec::new() };
        table.table.resize_with(size, HashEntryInternal::default);

        table
    }

    pub fn clear(&mut self, threads: usize) {
        let chunk_size = self.table.len().div_ceil(threads);

        std::thread::scope(|s| {
            for chunk in self.table.chunks_mut(chunk_size) {
                s.spawn(|| {
                    for entry in chunk.iter_mut() {
                        *entry = HashEntryInternal::default();
                    }
                });
            }
        });
    }

    pub fn fetch(&self, hash: u64) -> HashEntry {
        let idx = hash % (self.table.len() as u64);
        let entry = &self.table[idx as usize];

        HashEntry {
            hash: entry.hash.load(Ordering::Relaxed),
            q: entry.q.load(Ordering::Relaxed),
            d: entry.d.load(Ordering::Relaxed),
            visits: entry.visits.load(Ordering::Relaxed),
        }
    }

    fn key(hash: u64) -> u32 {
        (hash >> 32) as u32
    }

    pub fn get(&self, hash: u64) -> Option<HashEntry> {
        let entry = self.fetch(hash);

        if entry.hash == Self::key(hash) {
            Some(entry)
        } else {
            None
        }
    }

    pub fn push(&self, hash: u64, q: f32, draw: f32, visits: u64) {
        let idx = hash % (self.table.len() as u64);
        let key = Self::key(hash);
        let q_u32 = (q.clamp(0.0, 1.0) * u32::MAX as f32) as u32;
        let d_u32 = (draw.clamp(0.0, 1.0) * u32::MAX as f32) as u32;
        let visits_u32 = visits.min(u32::MAX as u64) as u32;

        let entry = &self.table[idx as usize];
        let existing_hash = entry.hash.load(Ordering::Relaxed);
        let replace = if existing_hash != key {
            true
        } else {
            visits_u32 >= entry.visits.load(Ordering::Relaxed)
        };

        if replace {
            entry.q.store(q_u32, Ordering::Relaxed);
            entry.d.store(d_u32, Ordering::Relaxed);
            entry.visits.store(visits_u32, Ordering::Relaxed);
            entry.hash.store(key, Ordering::Relaxed);
        }
    }
}
