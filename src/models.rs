use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
