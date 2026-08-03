use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, Mutex, Weak};
use tokio::sync::Mutex as AsyncMutex;

/// 按业务键复用异步互斥锁，并通过弱引用自动释放闲置条目。
pub struct KeyedAsyncLock<K> {
    locks: Mutex<HashMap<K, Weak<AsyncMutex<()>>>>,
}

impl<K> Default for KeyedAsyncLock<K> {
    fn default() -> Self {
        Self {
            locks: Mutex::new(HashMap::new()),
        }
    }
}

impl<K> KeyedAsyncLock<K>
where
    K: Eq + Hash,
{
    /// 获取指定键的共享异步锁。
    ///
    /// # 参数
    /// - `key`：需要串行化的业务键。
    pub fn get(&self, key: K) -> Result<Arc<AsyncMutex<()>>, String> {
        let mut locks = self.locks.lock().map_err(|error| error.to_string())?;
        if let Some(lock) = locks.get(&key).and_then(Weak::upgrade) {
            return Ok(lock);
        }

        let lock = Arc::new(AsyncMutex::new(()));
        locks.insert(key, Arc::downgrade(&lock));
        Ok(lock)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reuses_live_lock_and_releases_idle_entry() {
        let locks = KeyedAsyncLock::default();
        let first = locks.get("book".to_string()).unwrap();
        let second = locks.get("book".to_string()).unwrap();
        assert!(Arc::ptr_eq(&first, &second));

        drop(first);
        drop(second);
        let replacement = locks.get("book".to_string()).unwrap();
        assert_eq!(Arc::strong_count(&replacement), 1);
    }
}
