use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use git2::Repository;
use owo_colors::OwoColorize;
use uuid::Uuid;
use walkdir::WalkDir;

use crate::config::{load_registry, save_registry};
use crate::models::Project;

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

use crate::opts::SortBy;

pub fn cmd_ls(sort: SortBy) -> Result<()> {
    let reg = load_registry(None)?;
    if reg.projects.is_empty() {
        println!("no projects tracked");
        return Ok(());
    }

    // column widths
    let w_name = 25usize;
    let w_branch = 12usize;
    let w_commit = 10usize;
    let w_last_sync = 8usize;
    let w_last_mod = 14usize;

    // helper to pad visible length taking ANSI escapes into account
    fn strip_ansi(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\x1b' {
                // consume up to 'm'
                if let Some('[') = chars.peek() {
                    // consume '['
                    let _ = chars.next();
                    // skip until 'm'
                    while let Some(nc) = chars.next() {
                        if nc == 'm' {
                            break;
                        }
                    }
                    continue;
                }
            }
            out.push(c);
        }
        out
    }

    fn pad_visible(s: String, width: usize) -> String {
        let vis = strip_ansi(&s);
        let vis_len = vis.chars().count();
        if vis_len >= width {
            s
        } else {
            let mut res = s;
            for _ in 0..(width - vis_len) {
                res.push(' ');
            }
            res
        }
    }

    // Build rows with computed values so we can sort before printing
    struct Row {
        name: String,
        branch: String,
        commit: String,
        last_sync: Option<chrono::DateTime<Utc>>,
        last_mod: Option<chrono::DateTime<Utc>>,
    }

    let mut rows: Vec<Row> = Vec::with_capacity(reg.projects.len());
    for p in reg.projects {
        let name = p.name.clone().unwrap_or_else(|| {
            // fall back to basename of path
            Path::new(&p.path)
                .file_name()
                .and_then(|os| os.to_str())
                .unwrap_or("-")
                .to_string()
        });

        // try open git repo
        let (branch, commit, last_sync, last_mod) = match Repository::open(&p.path) {
            Ok(repo) => match repo.head() {
                Ok(head) => {
                    let branch = head.shorthand().unwrap_or("(detached)").to_string();
                    let commit = head
                        .peel_to_commit()
                        .map(|c| c.id().to_string())
                        .unwrap_or_default();
                    let short = if commit.len() >= 7 {
                        &commit[..7]
                    } else {
                        &commit
                    };
                    let last_sync = get_fetch_head_time(&p.path);
                    let last_mod = compute_last_modification(&repo, Path::new(&p.path));
                    (branch, short.to_string(), last_sync, last_mod)
                }
                Err(_) => ("-".to_string(), "-".to_string(), None, None),
            },
            Err(_) => ("-".to_string(), "-".to_string(), None, None),
        };

        rows.push(Row {
            name,
            branch,
            commit,
            last_sync,
            last_mod,
        });
    }

    // sort rows according to SortBy
    match sort {
        SortBy::Name => rows.sort_by_key(|r| r.name.to_lowercase()),
        SortBy::Branch => rows.sort_by_key(|r| r.branch.to_lowercase()),
        SortBy::Commit => rows.sort_by_key(|r| r.commit.to_lowercase()),
        SortBy::LastSync => rows.sort_by(|a, b| b.last_sync.cmp(&a.last_sync)),
        SortBy::LastModified => rows.sort_by(|a, b| b.last_mod.cmp(&a.last_mod)),
    }

    // header
    let h_name = pad_visible("NAME".to_string().bold().underline().to_string(), w_name);
    let h_branch = pad_visible(
        "BRANCH".to_string().bold().underline().to_string(),
        w_branch,
    );
    let h_commit = pad_visible(
        "COMMIT".to_string().bold().underline().to_string(),
        w_commit,
    );
    let h_last_sync = pad_visible(
        "last sync".to_string().bold().underline().to_string(),
        w_last_sync,
    );
    let h_last_mod = pad_visible(
        "last modified".to_string().bold().underline().to_string(),
        w_last_mod,
    );
    println!(
        "{}  {}  {}  {}  {}",
        h_name, h_branch, h_commit, h_last_sync, h_last_mod
    );

    for r in rows {
        // colorize branch and commit
        let branch_col = if r.branch == "(detached)" {
            r.branch.to_string().yellow().to_string()
        } else {
            r.branch.to_string().green().to_string()
        };
        let commit_col = if r.commit == "-" {
            r.commit.to_string()
        } else {
            r.commit.to_string().cyan().to_string()
        };

        let c_name = pad_visible(r.name.to_string().bold().to_string(), w_name);
        let c_branch = pad_visible(branch_col, w_branch);
        let c_commit = pad_visible(commit_col, w_commit);
        let c_last = pad_visible(fmt_age_opt(&r.last_sync).dimmed().to_string(), w_last_sync);
        let c_mod = pad_visible(fmt_age_opt(&r.last_mod).to_string(), w_last_mod);
        println!(
            "{}  {}  {}  {}  {}",
            c_name, c_branch, c_commit, c_last, c_mod
        );
    }
    Ok(())
}

// Return the path for an alias. Prefer exact name, id, then basename match.
pub fn cmd_show(alias: &str) -> Result<PathBuf> {
    let reg = load_registry(None)?;
    match reg.resolve(alias) {
        Some(p) => Ok(p),
        None => Err(anyhow::anyhow!("no project found for alias '{}'", alias)),
    }
}

// Initialize shell integration: print a fish wrapper script to stdout.
// The wrapper treats `pj cd <alias>` as a request to `cd` into the path printed
// by the `pj` binary; subcommands are forwarded to the binary.
pub fn cmd_init(shell: &str) -> Result<()> {
    if shell != "fish" {
        return Err(anyhow::anyhow!("only 'fish' shell is supported for init"));
    }

    let script = r#"function pj
    # no args -> show help from binary
    if test (count $argv) -eq 0
        command pj
        return $status
    end

    # handle explicit `cd` subcommand: `pj cd <alias>` -> cd (pj show <alias>)
    if test "$argv[1]" = "cd"
        if test (count $argv) -ge 2
            set target (command pj show $argv[2])
            if test -d "$target"
                cd "$target"
                return $status
            end
            # treat this as error
            command pj show $argv[2]
            return 1
        else
            command pj
            return $status
        end
    end

    # forward explicit subcommands to the binary
    switch $argv[1]
        case add ls init show help --help --version
            command pj $argv
            return $status
    end

    # otherwise treat first arg as an alias: call `pj show <alias>` and cd
    set target (command pj show $argv[1])
    if test -d "$target"
        cd "$target"
        return $status
    else
        command pj show $argv[1]
        return 1
    end
end"#;

    println!("{}", script);
    Ok(())
}

fn fmt_age_opt(dt: &Option<chrono::DateTime<Utc>>) -> String {
    match dt {
        None => "-".to_string(),
        Some(t) => {
            let now = Utc::now();
            let delta = now.signed_duration_since(*t);
            if delta.num_seconds() < 0 {
                return "0s".to_string();
            }
            if delta.num_days() >= 1 {
                return format!("{}d", delta.num_days());
            }
            if delta.num_hours() >= 1 {
                return format!("{}h", delta.num_hours());
            }
            if delta.num_minutes() >= 1 {
                return format!("{}m", delta.num_minutes());
            }
            format!("{}s", delta.num_seconds())
        }
    }
}

// Attempt to locate the git dir for a working directory and return the
// modification time of FETCH_HEAD if present.
fn get_fetch_head_time(workdir: &str) -> Option<chrono::DateTime<Utc>> {
    let p = Path::new(workdir);
    // .git inside workdir
    let git_path = p.join(".git");

    let git_dir = if git_path.is_dir() {
        Some(git_path)
    } else if git_path.is_file() {
        // .git may be a file containing: "gitdir: /actual/path"
        if let Ok(s) = fs::read_to_string(&git_path) {
            for line in s.lines() {
                if let Some(rest) = line.strip_prefix("gitdir: ") {
                    let td = PathBuf::from(rest.trim());
                    return fetch_head_mtime(&td);
                }
            }
        }
        None
    } else {
        None
    };

    if let Some(d) = git_dir {
        return fetch_head_mtime(&d);
    }

    None
}

fn fetch_head_mtime(gitdir: &Path) -> Option<chrono::DateTime<Utc>> {
    let fh = gitdir.join("FETCH_HEAD");
    match fs::metadata(&fh) {
        Ok(meta) => match meta.modified() {
            Ok(st) => {
                let dt: chrono::DateTime<Utc> = st.into();
                Some(dt)
            }
            Err(_) => None,
        },
        Err(_) => None,
    }
}

// Walk the working directory to find the most recently modified file that is
// not ignored by git. Returns the modification time of that file.
fn compute_last_modification(repo: &Repository, workdir: &Path) -> Option<chrono::DateTime<Utc>> {
    use std::time::SystemTime;
    let mut most_recent: Option<SystemTime> = None;

    for entry in WalkDir::new(workdir).into_iter().filter_map(|e| e.ok()) {
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        // skip files under .git
        if p.components().any(|c| c.as_os_str() == ".git") {
            continue;
        }

        // compute path relative to workdir for git ignore checks
        if let Ok(rel) = p.strip_prefix(workdir) {
            if let Ok(ignored) = repo.is_path_ignored(rel) {
                if ignored {
                    continue;
                }
            }
        }

        if let Ok(md) = fs::metadata(p) {
            if let Ok(mtime) = md.modified() {
                match most_recent {
                    Some(prev) if prev >= mtime => {}
                    _ => most_recent = Some(mtime),
                }
            }
        }
    }

    most_recent.map(|st| st.into())
}
