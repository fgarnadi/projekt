use std::path::PathBuf;

use anyhow::{Context, Result, anyhow};
use uuid::Uuid;

use crate::db::{Project, load_registry, save_registry};

// Add a project (path), optional name and tags vector
pub fn cmd_add(path: PathBuf, name: Option<String>, tags: Vec<String>) -> Result<()> {
    let canonical = path
        .canonicalize()
        .with_context(|| format!("canonicalizing path {}", path.display()))?;

    if !canonical.exists() || !canonical.is_dir() {
        return Err(anyhow!(
            "path does not exist or is not a directory: {}",
            canonical.display()
        ));
    }

    // load registry
    let mut reg = load_registry(None)?;

    // dedupe by canonical path
    let p_str = canonical.to_string_lossy().to_string();
    if reg.projects.iter().any(|p| p.path == p_str) {
        println!("project already tracked: {}", p_str);
        return Ok(());
    }

    let proj = Project {
        id: Uuid::now_v7().to_string(),
        name,
        path: p_str,
        tags,
    };

    reg.projects.push(proj.clone());
    save_registry(&reg, None)?;

    println!("added project:");
    println!("  id: {}", proj.id);
    println!("  path: {}", proj.path);
    if let Some(n) = proj.name.as_ref() {
        println!("  name: {}", n);
    }
    Ok(())
}
