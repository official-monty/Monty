use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone, Copy, Debug, Default)]
pub struct HashEntry {
    hash: u32,
    q: u16,
    visits: u32,
}

impl HashEntry {
    pub fn q(&self) -> f32 {
        f32::from(self.q) / f32::from(u16::MAX)
    }
}

#[derive(Default)]
struct HashEntryInternal(AtomicU64);

impl Clone for HashEntryInternal {
    fn clone(&self) -> Self {
        Self(AtomicU64::new(self.0.load(Ordering::Relaxed)))
    }
}

pub struct HashTable {
    table: Vec<HashEntryInternal>,
}

impl HashTable {
    pub fn new(size: usize, threads: usize) -> Self {
        let chunk_size = size.div_ceil(threads);

        let mut table = HashTable { table: Vec::new() };
        table.table.reserve_exact(size);

        unsafe {
            use std::mem::{size_of, MaybeUninit};
            let ptr = table.table.as_mut_ptr().cast();
            let uninit: &mut [MaybeUninit<u8>] =
                std::slice::from_raw_parts_mut(ptr, size * size_of::<HashEntryInternal>());

            std::thread::scope(|s| {
                for chunk in uninit.chunks_mut(chunk_size) {
                    s.spawn(|| {
                        chunk.as_mut_ptr().write_bytes(0, chunk.len());
                    });
                }
            });

            table.table.set_len(size);
        }

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
        let raw = self.table[idx as usize].0.load(Ordering::Relaxed);
        Self::unpack(raw)
    }

    fn key(hash: u64) -> u32 {
        (hash >> 40) as u32
    }

    fn pack(key: u32, q: u16, visits: u32) -> u64 {
        (u64::from(key) & 0xFFFFFF)
            | ((u64::from(q) & 0xFFFF) << 24)
            | ((u64::from(visits) & 0xFFFFFF) << 40)
    }

    fn unpack(raw: u64) -> HashEntry {
        HashEntry {
            hash: (raw & 0xFFFFFF) as u32,
            q: ((raw >> 24) & 0xFFFF) as u16,
            visits: ((raw >> 40) & 0xFFFFFF) as u32,
        }
    }

    pub fn get(&self, hash: u64) -> Option<HashEntry> {
        let entry = self.fetch(hash);

        if entry.hash == Self::key(hash) {
            Some(entry)
        } else {
            None
        }
    }

    pub fn push(&self, hash: u64, q: f32, visits: u64) {
        let idx = hash % (self.table.len() as u64);
        let key = Self::key(hash);
        let q_u16 = (q * f32::from(u16::MAX)) as u16;
        let visits_u32 = visits.min(0xFFFFFF) as u32;

        let new_raw = Self::pack(key, q_u16, visits_u32);
        let entry_atomic = &self.table[idx as usize].0;

        let mut old_raw = entry_atomic.load(Ordering::Relaxed);

        loop {
            let old_entry = Self::unpack(old_raw);

            let replace = if old_entry.hash != key {
                true
            } else {
                visits_u32 >= old_entry.visits
            };

            if !replace {
                break;
            }

            match entry_atomic.compare_exchange(
                old_raw,
                new_raw,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => old_raw = actual,
            }
        }
    }
}
