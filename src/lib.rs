pub mod agent;
pub mod bindings;
pub mod cli;
pub mod config;
pub mod llm;
pub mod mcp;
pub mod multi_agent;
pub mod sandbox;
pub mod session;
pub mod skills;
pub mod tools;

use napi::bindgen_prelude::*;
use napi_derive::napi;

#[napi]
pub fn orion_core_version() -> String {
    "2.0.0".to_string()
}
