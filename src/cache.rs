use std::collections::HashMap;
use std::error::Error;
use std::hash::Hash;
use std::time::{Duration, SystemTime};

use futures::future::BoxFuture;
use std::future::Future;
use tokio::sync::RwLock;

pub trait AsyncUpdateFn<K, V> {
    fn call(&self, args: K) -> BoxFuture<'static, Result<V, Box<dyn Error>>>;
}

impl<T, F, K, V> AsyncUpdateFn<K, V> for T
where
    T: Fn(K) -> F,
    F: Future<Output = Result<V, Box<dyn Error>>> + Send + 'static,
    K: Hash + Clone + Eq,
    V: Clone,
{
    fn call(&self, key: K) -> BoxFuture<'static, Result<V, Box<dyn Error>>> {
        Box::pin(self(key))
    }
}

pub struct TimedMemCached<K, V>
where
    K: Hash + Clone + Eq,
    V: Clone,
{
    expires: Duration,
    cache: RwLock<HashMap<K, (SystemTime, V)>>,
    update_func: Box<dyn AsyncUpdateFn<K, V> + Send + Sync>,
}

impl<K, V> TimedMemCached<K, V>
where
    K: Hash + Clone + Eq,
    V: Clone,
{
    pub fn new(
        expires: Duration,
        update_func: Box<dyn AsyncUpdateFn<K, V> + Send + Sync>,
    ) -> TimedMemCached<K, V> {
        TimedMemCached {
            expires,
            cache: RwLock::new(HashMap::new()),
            update_func,
        }
    }

    pub async fn update(&self, key: K) -> Result<V, Box<dyn Error>> {
        let mut cache = self.cache.write().await;
        // A second check to prevent another writer trying to update right after previous update
        // due to the write lock.
        if let Some((last_updated, value)) = cache.get(&key) {
            if last_updated.elapsed()? < self.expires {
                return Ok(value.clone());
            }
        }

        let value = self.update_func.call(key.clone()).await?;
        cache.insert(key, (SystemTime::now(), value.clone()));
        Ok(value)
    }

    pub async fn get(&self, key: K) -> Result<V, Box<dyn Error>> {
        let cache = self.cache.read().await;
        if let Some((last_updated, value)) = cache.get(&key) {
            if last_updated.elapsed()? < self.expires {
                return Ok(value.clone());
            }
        }
        drop(cache);
        self.update(key).await
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::time::UNIX_EPOCH;
    use tokio::time;

    async fn async_update(_unused: u64) -> Result<u64, Box<dyn std::error::Error>> {
        Ok(SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs())
    }

    #[tokio::test]
    async fn test_cache_init() {
        // With async closure
        let cache = TimedMemCached::new(
            Duration::from_secs(1),
            Box::new(async move |_unused| {
                Ok(SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs())
            }),
        );
        cache.get(1).await.unwrap();

        // With async function
        let cache = TimedMemCached::new(Duration::from_secs(1), Box::new(async_update));
        cache.get(1).await.unwrap();
    }

    #[tokio::test]
    async fn test_cache() {
        let cache = TimedMemCached::new(
            Duration::from_secs(1),
            Box::new(async move |_unused| {
                Ok(SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs())
            }),
        );

        let first_call = cache.get(1).await.unwrap();
        // Within 1 seconds the cache will return the old value.
        assert_eq!(first_call, cache.get(1).await.unwrap());
        assert_eq!(first_call, cache.get(1).await.unwrap());

        time::sleep(Duration::from_secs(1)).await;
        // After cache expired, the value should be updated.
        assert_ne!(first_call, cache.get(1).await.unwrap());
    }
}
