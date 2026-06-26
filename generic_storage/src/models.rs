use wincode::{SchemaWrite, SchemaRead};

#[derive(
    Debug, Clone, PartialEq,
    borsh::BorshSerialize, borsh::BorshDeserialize,
    SchemaWrite, SchemaRead,
    serde::Serialize, serde::Deserialize,
)]
pub struct Person {
    pub name: String,
    pub age:  u32,
}

// same for Config
#[derive(
    Debug, Clone, PartialEq,
    borsh::BorshSerialize, borsh::BorshDeserialize,
    SchemaWrite, SchemaRead,
    serde::Serialize, serde::Deserialize,
)]
pub struct Config {
    pub version:     u32,
    pub enabled:     bool,
    pub tags:        Vec<String>,
    pub max_retries: u8,
}

impl Person {
    pub fn new(name: impl Into<String>, age: u32) -> Self {
        Self { name: name.into(), age }
    }
}

impl Config {
    pub fn sample() -> Self {
        Self { version: 2, enabled: true, tags: vec!["prod".into(), "v2".into()], max_retries: 3 }
    }
}