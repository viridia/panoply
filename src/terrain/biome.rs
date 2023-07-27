use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
pub struct FloraTableEntry {
    proto: Option<String>,
    probability: f32,
}

#[derive(Default, Serialize, Deserialize)]
pub struct BiomeData {
    pub name: String,
    pub surface: String,
    pub trees: Vec<FloraTableEntry>,
    pub shrubs: Vec<FloraTableEntry>,
    pub herbs: Vec<FloraTableEntry>,
}

pub struct _Biome {}
