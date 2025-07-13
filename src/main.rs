mod config;
mod pokemon;

use axum::{
    Json, Router, debug_handler,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};
use config::Config;
use pokemon::Pokemon;
use rand;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// Custom error types for better error handling
#[derive(Debug)]
pub enum AppError {
    ConfigError(String),
    NetworkError(String),
    CacheError(String),
    ParseError(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            AppError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            AppError::CacheError(msg) => write!(f, "Cache error: {}", msg),
            AppError::ParseError(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

impl std::error::Error for AppError {}

impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self {
        AppError::NetworkError(err.to_string())
    }
}

impl From<toml::de::Error> for AppError {
    fn from(err: toml::de::Error) -> Self {
        AppError::ConfigError(err.to_string())
    }
}

struct AppState {
    cache: Arc<dyn CacheTrait>,
    config: Config,
}

#[derive(Default)]
struct InmemoryCache {
    store: Arc<Mutex<HashMap<String, Pokemon>>>,
}

trait CacheTrait: Send + Sync {
    fn get(&self, key: String) -> Option<Pokemon>;
    fn insert(&self, key: String, value: Pokemon) -> Result<(), AppError>;
}

impl CacheTrait for InmemoryCache {
    fn get(&self, key: String) -> Option<Pokemon> {
        match self.store.lock() {
            Ok(store) => {
                let result = store.get(key.as_str()).cloned();
                if result.is_some() {
                    tracing::debug!("Cache hit for key: {}", key);
                } else {
                    tracing::debug!("Cache miss for key: {}", key);
                }
                result
            }
            Err(e) => {
                tracing::error!("Failed to acquire cache read lock for key {}: {}", key, e);
                None
            }
        }
    }

    fn insert(&self, key: String, value: Pokemon) -> Result<(), AppError> {
        match self.store.lock() {
            Ok(mut store) => {
                let was_present = store.insert(key.clone(), value).is_some();
                if was_present {
                    tracing::debug!("Updated existing Pokémon in cache: {}", key);
                } else {
                    tracing::debug!("Inserted new Pokémon into cache: {}", key);
                }
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to acquire cache write lock: {}", e);
                tracing::error!("{}", error_msg);
                Err(AppError::CacheError(error_msg))
            }
        }
    }
}

fn load_config() -> Result<Config, AppError> {
    let config_str = include_str!("../config/config.toml");
    toml::from_str(config_str)
        .map_err(|e| {
            tracing::error!("Failed to parse config.toml: {}", e);
            AppError::from(e)
        })
}

async fn get_pokemon(api_url: String, id: u32) -> Result<Pokemon, AppError> {
    let url = format!("{}/pokemon/{}", api_url, id);
    tracing::debug!("Fetching Pokemon from URL: {}", url);
    
    let response = reqwest::get(&url).await
        .map_err(|e| {
            tracing::error!("Failed to make HTTP request to {}: {}", url, e);
            AppError::from(e)
        })?;
    
    if !response.status().is_success() {
        let status = response.status();
        let error_msg = format!("API request failed with status: {}", status);
        tracing::error!("{}", error_msg);
        return Err(AppError::NetworkError(error_msg));
    }
    
    let pokemon = response.json::<Pokemon>().await
        .map_err(|e| {
            tracing::error!("Failed to parse JSON response from {}: {}", url, e);
            AppError::ParseError(format!("JSON parsing failed: {}", e))
        })?;
    
    tracing::debug!("Successfully fetched Pokemon: {} (ID: {})", pokemon.name, pokemon.id);
    Ok(pokemon)
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                // axum logs rejections from built-in extractors with the `axum::rejection`
                // target, at `TRACE` level. `axum::rejection=trace` enables showing those events
                format!(
                    "{}=debug,tower_http=debug,axum::rejection=trace",
                    env!("CARGO_CRATE_NAME")
                )
                .into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = match load_config() {
        Ok(config) => config,
        Err(e) => {
            tracing::error!("Failed to load configuration: {}", e);
            std::process::exit(1);
        }
    };
    
    let inmemory_cache = InmemoryCache::default();
    let state = AppState {
        cache: Arc::new(inmemory_cache),
        config,
    };

    let app_state = Arc::new(state);

    let app = Router::new()
        .route("/random", get(get_random_pokemon_handler))
        .route("/pokemon/{id}", get(get_pokemon_handler))
        .with_state(app_state);

    let listener = match tokio::net::TcpListener::bind("0.0.0.0:3000").await {
        Ok(listener) => listener,
        Err(e) => {
            tracing::error!("Failed to bind to address 0.0.0.0:3000: {}", e);
            std::process::exit(1);
        }
    };

    tracing::info!("listening on {}", listener.local_addr().unwrap());
    
    if let Err(e) = axum::serve(listener, app).await {
        tracing::error!("Server error: {}", e);
        std::process::exit(1);
    }
}

#[debug_handler]
async fn get_random_pokemon_handler(
    State(app_state): State<Arc<AppState>>,
) -> (StatusCode, Json<Pokemon>) {
    let random_pokemon: u32 = rand::random_range(1..=1025);

    if let Some(pokemon) = app_state.cache.get(random_pokemon.to_string()) {
        tracing::debug!("Cache hit for Pokémon ID: {}", random_pokemon);
        return (StatusCode::OK, Json(pokemon));
    }
    
    let api_url = app_state.config.pokemon.api_url.to_string();
    tracing::debug!("Cache miss for Pokémon ID: {}, fetching from API", random_pokemon);

    match get_pokemon(api_url, random_pokemon).await {
        Ok(pokemon) => {
            tracing::debug!("Successfully fetched Pokémon ID: {}", random_pokemon);
            if let Err(e) = app_state
                .cache
                .insert(random_pokemon.to_string(), pokemon.clone())
            {
                tracing::warn!("Failed to cache Pokémon ID {}: {}", random_pokemon, e);
            }
            (StatusCode::OK, Json(pokemon))
        }
        Err(e) => {
            tracing::error!("Failed to fetch Pokémon ID {}: {}", random_pokemon, e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(Pokemon::default()))
        }
    }
}

#[debug_handler]
async fn get_pokemon_handler(
    State(app_state): State<Arc<AppState>>,
    Path(id): Path<u32>,
) -> (StatusCode, Json<Pokemon>) {
    if let Some(pokemon) = app_state.cache.get(id.to_string()) {
        tracing::debug!("Cache hit for Pokémon ID: {}", id);
        return (StatusCode::OK, Json(pokemon));
    }

    let api_url = app_state.config.pokemon.api_url.to_string();
    tracing::debug!("Cache miss for Pokémon ID: {}, fetching from API", id);

    match get_pokemon(api_url, id).await {
        Ok(pokemon) => {
            tracing::debug!("Successfully fetched Pokémon ID: {}", id);
            if let Err(e) = app_state.cache.insert(id.to_string(), pokemon.clone()) {
                tracing::warn!("Failed to cache Pokémon ID {}: {}", id, e);
            }
            (StatusCode::OK, Json(pokemon))
        }
        Err(e) => {
            tracing::error!("Failed to fetch Pokémon ID {}: {}", id, e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(Pokemon::default()))
        }
    }
}
