//! Extension ABI skeleton. The runtime is v1.x; this crate keeps types independent.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub api_version: String,
    pub entry: String,
}
