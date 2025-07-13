use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    pub pokemon: PokemonConfig,
    pub cache: CacheConfig,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PokemonConfig {
    pub api_url: String,
    pub timeout: u32,
    pub cache_enabled: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct CacheConfig {
    pub r#type: String,
    pub max_size: u32,
    pub expiration: u32,
}
