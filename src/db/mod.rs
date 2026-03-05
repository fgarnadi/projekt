use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use dirs_next::config_dir;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Project {
    pub id: String,
    pub name: Option<String>,
    pub path: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Registry {
    pub projects: Vec<Project>,
}

impl Registry {
    /// Resolve an alias or id or basename to a project path.
    pub fn resolve(&self, alias: &str) -> Option<PathBuf> {
        // try exact name or id
        for p in &self.projects {
            if let Some(n) = &p.name {
                if n == alias {
                    return Some(PathBuf::from(&p.path));
                }
            }
            if p.id == alias {
                return Some(PathBuf::from(&p.path));
            }
        }

        // try basename match
        for p in &self.projects {
            if let Some(b) = std::path::Path::new(&p.path)
                .file_name()
                .and_then(|s| s.to_str())
            {
                if b == alias {
                    return Some(PathBuf::from(&p.path));
                }
            }
        }

        None
    }
}

pub fn default_config_file() -> Result<PathBuf> {
    // On macOS prefer ~/.config/projekt
    if cfg!(target_os = "macos") {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                let home = std::env::var_os("HOME").unwrap_or_default();
                PathBuf::from(home).join(".config")
            });
        return Ok(base.join("projekt").join("projects.toml"));
    }

    let base = config_dir().unwrap_or_else(|| {
        // fallback to HOME/.config
        let home = std::env::var_os("HOME").unwrap_or_default();
        PathBuf::from(home).join(".config")
    });
    Ok(base.join("projekt").join("projects.toml"))
}

pub fn load_registry(path: Option<&Path>) -> Result<Registry> {
    let cfg = match path {
        Some(p) => p.to_path_buf(),
        None => default_config_file()?,
    };

    if !cfg.exists() {
        return Ok(Registry::default());
    }

    let s = fs::read_to_string(&cfg)
        .with_context(|| format!("reading registry file {}", cfg.display()))?;
    let reg: Registry = toml::from_str(&s).with_context(|| "parsing TOML registry")?;
    Ok(reg)
}

pub fn save_registry(reg: &Registry, path: Option<&Path>) -> Result<()> {
    let cfg = match path {
        Some(p) => p.to_path_buf(),
        None => default_config_file()?,
    };

    if let Some(parent) = cfg.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating config directory {}", parent.display()))?;
    }

    // pretty TOML
    let s = toml::to_string_pretty(&reg).context("serializing registry to TOML")?;
    // Atomic-ish write: write to temp file then rename
    let tmp = cfg.with_extension("toml.tmp");
    fs::write(&tmp, s).with_context(|| format!("writing temp registry {}", tmp.display()))?;
    fs::rename(&tmp, &cfg).with_context(|| {
        format!(
            "renaming temp registry {} -> {}",
            tmp.display(),
            cfg.display()
        )
    })?;
    Ok(())
}
