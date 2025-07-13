// pokemon.rs
// This file contains the definitions for the Pokemon-related data structures.

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Pokemon {
    pub id: i32,
    pub name: String,
    pub base_experience: i32,
    pub height: i32,
    pub is_default: bool,
    pub order: i32,
    pub weight: i32,
    pub forms: Vec<Option<NamedAPIResource>>,
    pub held_items: Vec<Option<PokemonHeldItem>>,
    pub moves: Vec<Option<PokemonMove>>,
    pub species: Option<NamedAPIResource>,
    pub stats: Vec<Option<PokemonStat>>,
    pub types: Vec<Option<PokemonType>>,
}

impl Default for Pokemon {
    fn default() -> Self {
        Self {
            id: 0,
            name: String::new(),
            base_experience: 0,
            height: 0,
            is_default: false,
            order: 0,
            weight: 0,
            forms: Vec::new(),
            held_items: Vec::new(),
            moves: Vec::new(),
            species: Some(NamedAPIResource {
                name: String::new(),
            }),
            stats: Vec::new(),
            types: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PokemonAbility {
    pub is_hidden: bool,
    pub slot: i32,
    pub ability: Option<NamedAPIResource>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NamedAPIResource {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PokemonHeldItem {
    pub item: Option<NamedAPIResource>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PokemonMove {
    pub r#move: Option<NamedAPIResource>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PokemonStat {
    pub base_stat: i32,
    pub effort: i32,
    pub stat: Option<NamedAPIResource>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PokemonType {
    pub slot: i32,
    pub r#type: Option<NamedAPIResource>,
}