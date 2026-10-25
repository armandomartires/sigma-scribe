//! A small persistent key/value store for one plugin (its layout cache).
//!
//! One file per key under the plugin's `.cache` folder, written atomically.
//! Bounded by entry count and total size; the oldest files go first, so a
//! plugin that rewrites a key keeps it fresh.

use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::SystemTime;

pub struct DirCache {
    dir: PathBuf,
    max_entries: usize,
    max_bytes: u64,
}

impl DirCache {
    #[must_use]
    pub fn new(dir: PathBuf, max_entries: usize, max_bytes: u64) -> Self {
        Self {
            dir,
            max_entries,
            max_bytes,
        }
    }

    fn path(&self, key: u64) -> PathBuf {
        self.dir.join(format!("{key:016x}"))
    }

    #[must_use]
    pub fn get(&self, key: u64) -> Option<Vec<u8>> {
        fs::read(self.path(key)).ok()
    }

    /// Stores `value`, then evicts the oldest entries until the limits hold.
    ///
    /// # Errors
    /// `InvalidInput` when the value alone exceeds the size limit, or any
    /// filesystem error.
    pub fn put(&self, key: u64, value: &[u8]) -> io::Result<()> {
        if value.len() as u64 > self.max_bytes {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "cache value too large",
            ));
        }
        sigma_core::store::write_atomic_bytes(&self.path(key), value)?;
        self.evict(key);
        Ok(())
    }

    fn evict(&self, keep: u64) {
        let keep = self.path(keep);
        let Ok(rd) = fs::read_dir(&self.dir) else {
            return;
        };
        let mut files: Vec<(SystemTime, u64, PathBuf)> = rd
            .filter_map(Result::ok)
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .is_some_and(|n| n.len() == 16 && !n.starts_with('.'))
            })
            .filter_map(|e| {
                let m = e.metadata().ok()?;
                Some((m.modified().ok()?, m.len(), e.path()))
            })
            .collect();
        files.sort();
        let mut total: u64 = files.iter().map(|f| f.1).sum();
        let mut count = files.len();
        for (_, len, path) in files {
            if count <= self.max_entries && total <= self.max_bytes {
                break;
            }
            if path == keep {
                continue;
            }
            if fs::remove_file(&path).is_ok() {
                count -= 1;
                total -= len;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("sigma-cache-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn round_trip_and_miss() {
        let d = dir("rt");
        let c = DirCache::new(d.clone(), 4, 1000);
        assert_eq!(c.get(1), None);
        c.put(1, b"hello").unwrap();
        assert_eq!(c.get(1).as_deref(), Some(&b"hello"[..]));
        c.put(1, b"world").unwrap();
        assert_eq!(c.get(1).as_deref(), Some(&b"world"[..]));
        fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn oldest_entries_are_evicted_but_the_new_one_stays() {
        let d = dir("ev");
        let c = DirCache::new(d.clone(), 2, 1000);
        for k in 1..=4u64 {
            c.put(k, &[k as u8; 10]).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(15));
        }
        assert_eq!(c.get(4).as_deref(), Some(&[4u8; 10][..]));
        assert!(c.get(3).is_some());
        assert!(c.get(1).is_none() && c.get(2).is_none());
        fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn size_limit_refuses_and_evicts() {
        let d = dir("sz");
        let c = DirCache::new(d.clone(), 10, 25);
        assert!(c.put(9, &[0; 26]).is_err());
        c.put(1, &[1; 10]).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(15));
        c.put(2, &[2; 10]).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(15));
        c.put(3, &[3; 10]).unwrap(); // 30 bytes > 25: key 1 goes
        assert!(c.get(1).is_none() && c.get(2).is_some() && c.get(3).is_some());
        fs::remove_dir_all(d).unwrap();
    }
}
