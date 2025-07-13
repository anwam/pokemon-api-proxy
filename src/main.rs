mod config;
mod cache;

use axum::{
    body::Body,
    extract::{Path, State},
    http::StatusCode,
    response::Response,
    routing::get,
    Router,
};
use cache::{CacheTrait, InmemoryCache};
use config::Config;
use rand;
use std::sync::Arc;
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
    cache: Arc<dyn CacheTrait<String>>,
    config: Config,
    client: reqwest::Client,
}

fn load_config() -> Result<Config, AppError> {
    let config_str = include_str!("../config/config.toml");
    toml::from_str(config_str)
        .map_err(|e| {
            tracing::error!("Failed to parse config.toml: {}", e);
            AppError::from(e)
        })
}

async fn proxy_pokemon_api(client: &reqwest::Client, api_url: &str, path: &str) -> Result<String, AppError> {
    let url = format!("{}{}", api_url, path);
    tracing::debug!("Proxying request to URL: {}", url);
    
    let response = client.get(&url).send().await
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
    
    let response_body = response.text().await
        .map_err(|e| {
            tracing::error!("Failed to read response body from {}: {}", url, e);
            AppError::ParseError(format!("Failed to read response: {}", e))
        })?;
    
    tracing::debug!("Successfully fetched data from: {}", url);
    Ok(response_body)
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
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    let config = match load_config() {
        Ok(config) => config,
        Err(e) => {
            tracing::error!("Failed to load configuration: {}", e);
            std::process::exit(1);
        }
    };
    
    // Initialize cache with configuration
    let inmemory_cache: InmemoryCache<String> = InmemoryCache::new(config.cache.clone());
    
    // Create HTTP client
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(config.pokemon.timeout as u64))
        .build()
        .map_err(|e| {
            tracing::error!("Failed to create HTTP client: {}", e);
            std::process::exit(1);
        })
        .unwrap();
    
    let state = AppState {
        cache: Arc::new(inmemory_cache),
        config,
        client,
    };

    let app_state = Arc::new(state);

    let app = Router::new()
        .route("/random", get(get_random_pokemon_handler))
        .route("/{*path}", get(proxy_handler))
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

async fn get_random_pokemon_handler(
    State(app_state): State<Arc<AppState>>,
) -> Response {
    let random_pokemon: u32 = rand::random_range(1..=1025);
    let path = format!("/pokemon/{}", random_pokemon);

    if let Some(cached_response) = app_state.cache.get(&path) {
        tracing::debug!("Cache hit for path: {}", path);
        return Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "application/json")
            .body(Body::from(cached_response))
            .unwrap();
    }
    
    let api_url = &app_state.config.pokemon.api_url;
    tracing::debug!("Cache miss for path: {}, fetching from API", path);

    match proxy_pokemon_api(&app_state.client, api_url, &path).await {
        Ok(response_body) => {
            tracing::debug!("Successfully fetched data for path: {}", path);
            if let Err(e) = app_state
                .cache
                .insert(path.clone(), response_body.clone())
            {
                tracing::warn!("Failed to cache response for path {}: {}", path, e);
            }
            Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "application/json")
                .body(Body::from(response_body))
                .unwrap()
        }
        Err(e) => {
            tracing::error!("Failed to fetch data for path {}: {}", path, e);
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("content-type", "application/json")
                .body(Body::from(r#"{"error": "Internal server error"}"#))
                .unwrap()
        }
    }
}

async fn proxy_handler(
    State(app_state): State<Arc<AppState>>,
    Path(path): Path<String>,
) -> Response {
    let full_path = format!("/{}", path);
    
    if let Some(cached_response) = app_state.cache.get(&full_path) {
        tracing::debug!("Cache hit for path: {}", full_path);
        return Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "application/json")
            .body(Body::from(cached_response))
            .unwrap();
    }

    let api_url = &app_state.config.pokemon.api_url;
    tracing::debug!("Cache miss for path: {}, fetching from API", full_path);

    match proxy_pokemon_api(&app_state.client, api_url, &full_path).await {
        Ok(response_body) => {
            tracing::debug!("Successfully fetched data for path: {}", full_path);
            if let Err(e) = app_state.cache.insert(full_path.clone(), response_body.clone()) {
                tracing::warn!("Failed to cache response for path {}: {}", full_path, e);
            }
            Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "application/json")
                .body(Body::from(response_body))
                .unwrap()
        }
        Err(e) => {
            tracing::error!("Failed to fetch data for path {}: {}", full_path, e);
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("content-type", "application/json")
                .body(Body::from(r#"{"error": "Internal server error"}"#))
                .unwrap()
        }
    }
}
