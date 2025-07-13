// Example demonstrating the flexible generic cache usage

use pokemon_api_proxy::{CacheTrait, InmemoryCache, CacheConfig};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    println!("🚀 Generic Cache Examples");
    
    // Example 1: String cache for API responses
    string_cache_example();
    
    // Example 2: Numeric cache for computed values  
    numeric_cache_example();
    
    // Example 3: Custom struct cache
    custom_struct_example();
    
    // Example 4: Vector cache for batch data
    vector_cache_example();
    
    // Example 5: Async concurrent operations
    async_cache_example().await;
}

fn string_cache_example() {
    println!("\n📝 String Cache Example:");
    
    let config = CacheConfig {
        r#type: "memory".to_string(),
        max_size: 500,
        expiration: 1800, // 30 minutes
    };
    
    let cache: InmemoryCache<String> = InmemoryCache::new(config);
    
    // Cache some API response data
    let _ = cache.insert(
        "user:123".to_string(), 
        "{\"id\": 123, \"name\": \"John Doe\"}".to_string()
    );
    
    let _ = cache.insert(
        "config:app".to_string(),
        "debug=true,timeout=30".to_string()
    );
    
    // Retrieve cached data
    if let Some(user_data) = cache.get("user:123") {
        println!("✅ Cached user data: {}", user_data);
    }
    
    if let Some(config_data) = cache.get("config:app") {
        println!("✅ Cached config: {}", config_data);
    }
    
    println!("📊 Cache size: {}", cache.size());
}

fn numeric_cache_example() {
    println!("\n🔢 Numeric Cache Example:");
    
    let config = CacheConfig {
        r#type: "memory".to_string(),
        max_size: 100,
        expiration: 300, // 5 minutes
    };
    
    let cache: InmemoryCache<f64> = InmemoryCache::new(config);
    
    // Cache computed mathematical results
    let _ = cache.insert("pi".to_string(), 3.14159);
    let _ = cache.insert("e".to_string(), 2.71828);
    let _ = cache.insert("golden_ratio".to_string(), 1.618033);
    
    // Retrieve and use cached calculations
    if let Some(pi) = cache.get("pi") {
        println!("✅ π = {}", pi);
        println!("✅ Area of circle (r=5): {}", pi * 5.0 * 5.0);
    }
    
    if let Some(e) = cache.get("e") {
        println!("✅ e = {}", e);
    }
    
    println!("📊 Math cache size: {}", cache.size());
}

fn custom_struct_example() {
    println!("\n🏗️ Custom Struct Cache Example:");
    
    #[derive(Clone, Debug)]
    struct UserSession {
        user_id: u64,
        token: String,
        expires_at: u64,
        permissions: Vec<String>,
    }
    
    let config = CacheConfig {
        r#type: "memory".to_string(),
        max_size: 10000,
        expiration: 7200, // 2 hours
    };
    
    let cache: InmemoryCache<UserSession> = InmemoryCache::new(config);
    
    let session = UserSession {
        user_id: 123,
        token: "abc123xyz".to_string(),
        expires_at: 1234567890,
        permissions: vec!["read".to_string(), "write".to_string()],
    };
    
    let _ = cache.insert("session:abc123xyz".to_string(), session);
    
    // Retrieve session and check permissions
    if let Some(session) = cache.get("session:abc123xyz") {
        println!("✅ User session: ID={}, Permissions={:?}", 
                session.user_id, session.permissions);
    }
    
    println!("📊 Session cache size: {}", cache.size());
}

fn vector_cache_example() {
    println!("\n📋 Vector Cache Example:");
    
    let config = CacheConfig {
        r#type: "memory".to_string(),
        max_size: 50,
        expiration: 600, // 10 minutes
    };
    
    let cache: InmemoryCache<Vec<String>> = InmemoryCache::new(config);
    
    let morning_tasks = vec![
        "Check emails".to_string(),
        "Review PRs".to_string(), 
        "Team standup".to_string(),
        "Code review".to_string(),
    ];
    
    let afternoon_tasks = vec![
        "Deploy to staging".to_string(),
        "Write documentation".to_string(),
        "Fix bugs".to_string(),
    ];
    
    let _ = cache.insert("tasks:morning".to_string(), morning_tasks);
    let _ = cache.insert("tasks:afternoon".to_string(), afternoon_tasks);
    
    // Retrieve and process task lists
    if let Some(tasks) = cache.get("tasks:morning") {
        println!("✅ Morning tasks ({}): {:?}", tasks.len(), tasks);
    }
    
    if let Some(tasks) = cache.get("tasks:afternoon") {
        println!("✅ Afternoon tasks ({}): {:?}", tasks.len(), tasks);
    }
    
    println!("📊 Task cache size: {}", cache.size());
}

async fn async_cache_example() {
    println!("\n⚡ Async Concurrent Cache Example:");
    
    let config = CacheConfig {
        r#type: "memory".to_string(),
        max_size: 100,
        expiration: 3600,
    };
    
    let cache: InmemoryCache<String> = InmemoryCache::new(config);
    let cache_arc = Arc::new(cache);
    
    // Simulate concurrent cache operations
    let tasks: Vec<_> = (1..=5).map(|i| {
        let cache_clone = Arc::clone(&cache_arc);
        tokio::spawn(async move {
            let key = format!("async_key_{}", i);
            let value = format!("async_value_{}", i);
            
            // Simulate some async work
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            
            if let Err(e) = cache_clone.insert(key.clone(), value) {
                println!("❌ Failed to insert {}: {}", key, e);
            } else {
                println!("✅ Successfully cached: {}", key);
            }
            
            // Try to retrieve what we just inserted
            if let Some(retrieved) = cache_clone.get(&key) {
                println!("✅ Retrieved: {} = {}", key, retrieved);
            }
        })
    }).collect();
    
    // Wait for all tasks to complete
    for task in tasks {
        let _ = task.await;
    }
    
    println!("📊 Final async cache size: {}", cache_arc.size());
    println!("✨ Async cache example completed!");
}

// Utility function showing polymorphic usage
#[allow(dead_code)]
fn demonstrate_trait_objects() {
    println!("\n🎭 Trait Object Example:");
    
    let config = CacheConfig {
        r#type: "memory".to_string(),
        max_size: 10,
        expiration: 300,
    };
    
    // Create different cache types as trait objects
    let string_cache: Box<dyn CacheTrait<String>> = 
        Box::new(InmemoryCache::<String>::new(config.clone()));
    
    let number_cache: Box<dyn CacheTrait<i32>> = 
        Box::new(InmemoryCache::<i32>::new(config));
    
    // Use them polymorphically
    let _ = string_cache.insert("key1".to_string(), "value1".to_string());
    let _ = number_cache.insert("key1".to_string(), 42);
    
    println!("✅ Trait objects created and used successfully!");
}
