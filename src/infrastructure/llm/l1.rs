//! L1 内存 LRU 缓存 backend
//!
//! L1 由 Router 内联调用，不做持久化。Capacity 预分配，key 为 64-bit hash。

use std::num::NonZeroUsize;
use std::sync::Mutex;

use lru::LruCache;

use crate::infrastructure::llm::cache::CacheValue;

/// 内存 LRU backend
pub struct MemoryLruBackend {
    cache: Mutex<LruCache<u64, (CacheValue, std::time::SystemTime)>>,
}

impl MemoryLruBackend {
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: Mutex::new(LruCache::new(
                NonZeroUsize::new(capacity).expect("capacity > 0"),
            )),
        }
    }

    pub fn get(&self, hash: u64) -> Option<CacheValue> {
        let mut c = self.cache.lock().ok()?;
        c.pop(&hash).map(|(v, _)| v)
    }

    pub fn put(&self, hash: u64, value: CacheValue) {
        if let Ok(mut c) = self.cache.lock() {
            c.put(hash, (value, std::time::SystemTime::now()));
        }
    }

    pub fn len(&self) -> usize {
        self.cache.lock().map(|c| c.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::llm::cache::CacheValue;

    #[test]
    fn lru_put_then_get_returns_value() {
        let b = MemoryLruBackend::new(10);
        b.put(
            1,
            CacheValue::Execution {
                result_summary: "x".into(),
                success: true,
                duration_ms: 100,
            },
        );
        match b.get(1) {
            Some(CacheValue::Execution { result_summary, .. }) => assert_eq!(result_summary, "x"),
            _ => panic!("expected Execution"),
        }
    }

    #[test]
    fn lru_evicts_oldest_when_full() {
        let b = MemoryLruBackend::new(2);
        b.put(
            1,
            CacheValue::Execution {
                result_summary: "a".into(),
                success: true,
                duration_ms: 1,
            },
        );
        b.put(
            2,
            CacheValue::Execution {
                result_summary: "b".into(),
                success: true,
                duration_ms: 1,
            },
        );
        b.put(
            3,
            CacheValue::Execution {
                result_summary: "c".into(),
                success: true,
                duration_ms: 1,
            },
        );
        assert!(b.get(1).is_none()); // 被淘汰
        assert!(b.get(2).is_some());
        assert!(b.get(3).is_some());
    }
}
