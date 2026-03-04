mod commands;
mod config;
mod models;
mod opts;

use std::path::PathBuf;

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};

#[derive(Parser)]
#[command(name = "pj", about = "Project tracker (TOML-backed)")]
struct Cli {
    /// Print the config directory and exit
    #[arg(long = "config-dir", global = true)]
    config_dir: bool,

    // positional alias removed; use `pj show <alias>` instead
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a project path to tracker
    Add {
        /// Path to project directory
        path: PathBuf,

        /// Optional display alias (defaults to directory name)
        #[arg(short, long)]
        alias: Option<String>,

        /// Optional comma-separated tags
        #[arg(short, long)]
        tags: Option<String>,
    },

    /// List tracked projects
    Ls {
        /// Sort by which column
        #[arg(long, value_enum, default_value_t = crate::opts::SortBy::LastModified)]
        sort: crate::opts::SortBy,
    },

    /// Show the path for a project alias
    Show {
        /// Alias or directory name
        alias: String,
    },

    /// Install shell integration
    Init {
        /// Shell to initialize. Supported: fish.
        shell: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.config_dir {
        // Print the config directory (not the file) and exit
        let cfg_file = config::default_config_file()?;
        if let Some(parent) = cfg_file.parent() {
            println!("{}", parent.display());
        } else {
            println!("{}", cfg_file.display());
        }
        return Ok(());
    }

    match cli.command {
        Some(Commands::Add { path, alias, tags }) => {
            let tag_vec = tags
                .map(|s| {
                    s.split(',')
                        .map(|t| t.trim().to_string())
                        .filter(|t| !t.is_empty())
                        .collect()
                })
                .unwrap_or_default();

            // determine default name: alias or directory name
            let name = alias.or_else(|| {
                path.file_name()
                    .and_then(|os| os.to_str().map(|s| s.to_string()))
            });

            commands::cmd_add(path, name, tag_vec)?;
        }
        Some(Commands::Ls { sort }) => {
            commands::cmd_ls(sort)?;
        }
        Some(Commands::Init { shell }) => {
            commands::cmd_init(&shell)?;
        }
        Some(Commands::Show { alias }) => {
            let path = commands::cmd_show(&alias)?;
            println!("{}", path.display());
        }
        None => {
            // No subcommand provided — print help
            let _ = Cli::command().print_help();
            println!();
        }
    }

    Ok(())
}
