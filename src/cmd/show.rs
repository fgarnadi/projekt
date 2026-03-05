use std::path::PathBuf;

use anyhow::{Result, anyhow};

use crate::db::load_registry;

// Return the path for an alias. Prefer exact name, id, then basename match.
pub fn cmd_show(alias: &str) -> Result<PathBuf> {
    let reg = load_registry(None)?;
    match reg.resolve(alias) {
        Some(p) => Ok(p),
        None => Err(anyhow!("no project found for alias '{}'", alias)),
    }
}
