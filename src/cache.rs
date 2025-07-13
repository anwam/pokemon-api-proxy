use crate::config::CacheConfig;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

// Custom error types for cache operations
#[derive(Debug)]
pub enum CacheError {
    LockError(String),
    MaxSizeExceeded,
    InvalidKey(String),
}

impl std::fmt::Display for CacheError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CacheError::LockError(msg) => write!(f, "Cache lock error: {}", msg),
            CacheError::MaxSizeExceeded => write!(f, "Cache maximum size exceeded"),
            CacheError::InvalidKey(key) => write!(f, "Invalid cache key: {}", key),
        }
    }
}

impl std::error::Error for CacheError {}

// Cache entry with expiration support
#[derive(Debug, Clone)]
struct CacheEntry<T> {
    value: T,
    created_at: Instant,
    access_count: u64,
}

impl<T: Clone> CacheEntry<T> {
    fn new(value: T) -> Self {
        Self {
            value,
            created_at: Instant::now(),
            access_count: 1,
        }
    }

    fn is_expired(&self, expiration_duration: Duration) -> bool {
        self.created_at.elapsed() > expiration_duration
    }

    fn access(&mut self) -> T {
        self.access_count += 1;
        self.value.clone()
    }
}

// Cache trait for different implementations
pub trait CacheTrait<T>: Send + Sync
where
    T: Clone + Send + Sync,
{
    fn get(&self, key: &str) -> Option<T>;
    fn insert(&self, key: String, value: T) -> Result<(), CacheError>;
    fn remove(&self, key: &str) -> Option<T>;
    fn clear(&self);
    fn size(&self) -> usize;
    fn hit_rate(&self) -> f64;
    fn cleanup_expired(&self);
}

// Statistics for cache monitoring
#[derive(Debug, Default, Clone)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub inserts: u64,
    pub removes: u64,
    pub cleanups: u64,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        if self.hits + self.misses == 0 {
            0.0
        } else {
            self.hits as f64 / (self.hits + self.misses) as f64
        }
    }
}

// In-memory cache implementation
pub struct InmemoryCache<T>
where
    T: Clone + Send + Sync,
{
    store: Arc<Mutex<HashMap<String, CacheEntry<T>>>>,
    config: CacheConfig,
    stats: Arc<Mutex<CacheStats>>,
}

impl<T> InmemoryCache<T>
where
    T: Clone + Send + Sync,
{
    pub fn new(config: CacheConfig) -> Self {
        tracing::info!(
            "Initializing in-memory cache with max_size: {}, expiration: {}s",
            config.max_size,
            config.expiration
        );

        Self {
            store: Arc::new(Mutex::new(HashMap::new())),
            config,
            stats: Arc::new(Mutex::new(CacheStats::default())),
        }
    }

    // Create with default configuration
    pub fn with_defaults() -> Self {
        let default_config = CacheConfig {
            r#type: "memory".to_string(),
            max_size: 1000,
            expiration: 3600, // 1 hour
        };
        Self::new(default_config)
    }

    // Check if cache is enabled based on config
    pub fn is_enabled(&self) -> bool {
        self.config.r#type == "memory"
    }

    // Get cache configuration
    pub fn config(&self) -> &CacheConfig {
        &self.config
    }

    // Evict least recently used entries when cache is full
    fn evict_lru(&self, store: &mut HashMap<String, CacheEntry<T>>) -> Result<(), CacheError> {
        if store.len() < self.config.max_size as usize {
            return Ok(());
        }

        // Find the entry with the oldest access time and lowest access count
        let lru_key = store
            .iter()
            .min_by(|a, b| {
                a.1.created_at
                    .cmp(&b.1.created_at)
                    .then_with(|| a.1.access_count.cmp(&b.1.access_count))
            })
            .map(|(key, _)| key.clone());

        if let Some(key) = lru_key {
            store.remove(&key);
            tracing::debug!("Evicted LRU cache entry: {}", key);
            
            // Update stats
            if let Ok(mut stats) = self.stats.lock() {
                stats.removes += 1;
            }
        }

        Ok(())
    }

    // Clean up expired entries
    fn cleanup_expired_entries(&self) {
        let expiration_duration = Duration::from_secs(self.config.expiration as u64);
        
        if let Ok(mut store) = self.store.lock() {
            let expired_keys: Vec<String> = store
                .iter()
                .filter(|(_, entry)| entry.is_expired(expiration_duration))
                .map(|(key, _)| key.clone())
                .collect();

            let expired_count = expired_keys.len();
            for key in expired_keys {
                store.remove(&key);
                tracing::debug!("Removed expired cache entry: {}", key);
            }

            if expired_count > 0 {
                tracing::debug!("Cleaned up {} expired cache entries", expired_count);
                
                // Update stats
                if let Ok(mut stats) = self.stats.lock() {
                    stats.cleanups += 1;
                    stats.removes += expired_count as u64;
                }
            }
        } else {
            tracing::error!("Failed to acquire lock for cache cleanup");
        }
    }

    // Get detailed cache statistics
    pub fn stats(&self) -> Option<CacheStats> {
        self.stats.lock().ok().map(|stats| CacheStats {
            hits: stats.hits,
            misses: stats.misses,
            inserts: stats.inserts,
            removes: stats.removes,
            cleanups: stats.cleanups,
        })
    }

    // Check if a key exists without retrieving the value
    pub fn contains_key(&self, key: &str) -> bool {
        if let Ok(store) = self.store.lock() {
            store.contains_key(key)
        } else {
            false
        }
    }

    // Get all cached Pokemon IDs
    pub fn keys(&self) -> Vec<String> {
        if let Ok(store) = self.store.lock() {
            store.keys().cloned().collect()
        } else {
            Vec::new()
        }
    }
}

impl<T> Default for InmemoryCache<T>
where
    T: Clone + Send + Sync,
{
    fn default() -> Self {
        Self::with_defaults()
    }
}

impl<T> CacheTrait<T> for InmemoryCache<T>
where
    T: Clone + Send + Sync,
{
    fn get(&self, key: &str) -> Option<T> {
        if key.is_empty() {
            tracing::warn!("Attempted to get cache entry with empty key");
            return None;
        }

        let expiration_duration = Duration::from_secs(self.config.expiration as u64);

        match self.store.lock() {
            Ok(mut store) => {
                if let Some(entry) = store.get_mut(key) {
                    if entry.is_expired(expiration_duration) {
                        tracing::debug!("Cache entry expired for key: {}", key);
                        store.remove(key);
                        
                        // Update stats
                        if let Ok(mut stats) = self.stats.lock() {
                            stats.misses += 1;
                        }
                        
                        None
                    } else {
                        tracing::debug!("Cache hit for key: {}", key);
                        
                        // Update stats
                        if let Ok(mut stats) = self.stats.lock() {
                            stats.hits += 1;
                        }
                        
                        Some(entry.access())
                    }
                } else {
                    tracing::debug!("Cache miss for key: {}", key);
                    
                    // Update stats
                    if let Ok(mut stats) = self.stats.lock() {
                        stats.misses += 1;
                    }
                    
                    None
                }
            }
            Err(e) => {
                tracing::error!("Failed to acquire cache read lock for key {}: {}", key, e);
                None
            }
        }
    }

    fn insert(&self, key: String, value: T) -> Result<(), CacheError> {
        if key.is_empty() {
            return Err(CacheError::InvalidKey("Key cannot be empty".to_string()));
        }

        match self.store.lock() {
            Ok(mut store) => {
                // Check if we need to evict entries before inserting
                if store.len() >= self.config.max_size as usize && !store.contains_key(&key) {
                    self.evict_lru(&mut store)?;
                }

                let was_present = store.insert(key.clone(), CacheEntry::new(value)).is_some();
                
                if was_present {
                    tracing::debug!("Updated existing Pokémon in cache: {}", key);
                } else {
                    tracing::debug!("Inserted new Pokémon into cache: {}", key);
                }

                // Update stats
                if let Ok(mut stats) = self.stats.lock() {
                    stats.inserts += 1;
                }

                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to acquire cache write lock: {}", e);
                tracing::error!("{}", error_msg);
                Err(CacheError::LockError(error_msg))
            }
        }
    }

    fn remove(&self, key: &str) -> Option<T> {
        if key.is_empty() {
            tracing::warn!("Attempted to remove cache entry with empty key");
            return None;
        }

        match self.store.lock() {
            Ok(mut store) => {
                let removed = store.remove(key).map(|entry| entry.value);
                if removed.is_some() {
                    tracing::debug!("Removed cache entry: {}", key);
                    
                    // Update stats
                    if let Ok(mut stats) = self.stats.lock() {
                        stats.removes += 1;
                    }
                }
                removed
            }
            Err(e) => {
                tracing::error!("Failed to acquire cache write lock for removal of key {}: {}", key, e);
                None
            }
        }
    }

    fn clear(&self) {
        match self.store.lock() {
            Ok(mut store) => {
                let size = store.len();
                store.clear();
                tracing::info!("Cleared cache ({} entries)", size);
                
                // Reset stats
                if let Ok(mut stats) = self.stats.lock() {
                    *stats = CacheStats::default();
                }
            }
            Err(e) => {
                tracing::error!("Failed to acquire cache write lock for clearing: {}", e);
            }
        }
    }

    fn size(&self) -> usize {
        match self.store.lock() {
            Ok(store) => store.len(),
            Err(_) => 0,
        }
    }

    fn hit_rate(&self) -> f64 {
        match self.stats.lock() {
            Ok(stats) => stats.hit_rate(),
            Err(_) => 0.0,
        }
    }

    fn cleanup_expired(&self) {
        self.cleanup_expired_entries();
    }
}

// Periodic cleanup task
impl<T> InmemoryCache<T>
where
    T: Clone + Send + Sync,
{
    pub async fn start_cleanup_task<U>(cache: Arc<dyn CacheTrait<U>>)
    where
        U: Clone + Send + Sync,
    {
        let mut interval = tokio::time::interval(Duration::from_secs(300)); // Clean every 5 minutes
        
        loop {
            interval.tick().await;
            tracing::debug!("Starting periodic cache cleanup");
            cache.cleanup_expired();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_basic_operations() {
        let config = CacheConfig {
            r#type: "memory".to_string(),
            max_size: 3,
            expiration: 3600,
        };
        
        let cache: InmemoryCache<String> = InmemoryCache::new(config);
        let pokemon_json = r#"{"id": 25, "name": "pikachu"}"#.to_string();

        // Test insert and get
        assert!(cache.insert("25".to_string(), pokemon_json.clone()).is_ok());
        
        let retrieved = cache.get("25");
        assert!(retrieved.is_some());
        assert!(retrieved.unwrap().contains("pikachu"));

        // Test cache miss
        assert!(cache.get("1").is_none());
    }

    #[test]
    fn test_cache_eviction() {
        let config = CacheConfig {
            r#type: "memory".to_string(),
            max_size: 2,
            expiration: 3600,
        };
        
        let cache: InmemoryCache<String> = InmemoryCache::new(config);
        
        // Fill cache to capacity
        assert!(cache.insert("1".to_string(), r#"{"id": 1, "name": "bulbasaur"}"#.to_string()).is_ok());
        assert!(cache.insert("2".to_string(), r#"{"id": 2, "name": "ivysaur"}"#.to_string()).is_ok());

        // Insert one more (should trigger eviction)
        assert!(cache.insert("3".to_string(), r#"{"id": 3, "name": "venusaur"}"#.to_string()).is_ok());

        // The first entry should have been evicted
        assert!(cache.get("1").is_none());
        assert!(cache.get("2").is_some());
        assert!(cache.get("3").is_some());
    }

    #[test]
    fn test_invalid_operations() {
        let cache: InmemoryCache<String> = InmemoryCache::with_defaults();

        // Test empty key
        assert!(cache.insert("".to_string(), "test".to_string()).is_err());
        assert!(cache.get("").is_none());
    }

    #[test]
    fn test_generic_string_cache() {
        let config = CacheConfig {
            r#type: "memory".to_string(),
            max_size: 10,
            expiration: 3600,
        };
        
        let cache: InmemoryCache<String> = InmemoryCache::new(config);
        
        // Test with String values
        assert!(cache.insert("key1".to_string(), "value1".to_string()).is_ok());
        
        let retrieved = cache.get("key1");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), "value1");
        
        // Test cache miss
        assert!(cache.get("nonexistent").is_none());
    }

    #[test]
    fn test_generic_number_cache() {
        let config = CacheConfig {
            r#type: "memory".to_string(),
            max_size: 5,
            expiration: 3600,
        };
        
        let cache: InmemoryCache<i32> = InmemoryCache::new(config);
        
        // Test with i32 values
        assert!(cache.insert("number1".to_string(), 42).is_ok());
        assert!(cache.insert("number2".to_string(), 100).is_ok());
        
        assert_eq!(cache.get("number1"), Some(42));
        assert_eq!(cache.get("number2"), Some(100));
        assert_eq!(cache.get("nonexistent"), None);
    }
}
