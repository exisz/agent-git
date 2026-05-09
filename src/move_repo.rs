use crate::ephemeral::{is_banned, is_ephemeral, refuse_banned, refuse_ephemeral};
use crate::normalize::normalize_url;
use crate::registry::Registry;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

pub fn handle_move(path: &str, dest: &str, allow_tmp: bool) -> ExitCode {
    let old_path = match Path::new(path).canonicalize() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: Cannot resolve path '{}': {}", path, e);
            return ExitCode::from(1);
        }
    };
    let old_path_s = old_path.to_string_lossy().to_string();

    if !old_path.join(".git").exists() {
        eprintln!("error: '{}' is not a git repository", old_path_s);
        return ExitCode::from(1);
    }

    let new_path = resolve_destination(dest);
    let new_path_s = new_path.to_string_lossy().to_string();

    if !allow_tmp && is_ephemeral(&new_path_s) {
        return refuse_ephemeral(&new_path_s, "move");
    }
    if is_banned(&new_path_s) {
        return refuse_banned(&new_path_s, "move");
    }
    if new_path.exists() {
        eprintln!("error: destination already exists: {}", new_path.display());
        return ExitCode::from(1);
    }
    if let Some(parent) = new_path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            eprintln!(
                "error: failed to create destination parent '{}': {}",
                parent.display(),
                e
            );
            return ExitCode::from(1);
        }
    }

    let url = infer_repo_url(&old_path).unwrap_or_else(|| format!("local:{}", new_path_s));

    if let Err(e) = move_dir(&old_path, &new_path) {
        eprintln!(
            "error: failed to move '{}' to '{}': {}",
            old_path.display(),
            new_path.display(),
            e
        );
        return ExitCode::from(1);
    }

    let mut registry = Registry::load();
    if !registry.move_path(&old_path_s, new_path_s.clone()) {
        if let Err(e) = registry.upsert_by_path(url.clone(), new_path_s.clone()) {
            eprintln!("warning: moved repo but failed to register it: {}", e);
            return ExitCode::from(1);
        }
    }

    if let Err(e) = registry.save() {
        eprintln!("warning: moved repo but failed to save registry: {}", e);
        return ExitCode::from(1);
    }

    println!("Moved: {} → {}", old_path_s, new_path_s);
    println!("Registered: {} → {}", url, new_path_s);
    ExitCode::SUCCESS
}

fn move_dir(old_path: &Path, new_path: &Path) -> std::io::Result<()> {
    match fs::rename(old_path, new_path) {
        Ok(()) => Ok(()),
        Err(e) if e.raw_os_error() == Some(18) => {
            copy_across_devices(old_path, new_path)?;
            fs::remove_dir_all(old_path)
        }
        Err(e) => Err(e),
    }
}

fn copy_across_devices(src: &Path, dst: &Path) -> std::io::Result<()> {
    let status = Command::new("cp").arg("-a").arg(src).arg(dst).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("cp -a exited with {}", status),
        ))
    }
}

fn resolve_destination(dest: &str) -> PathBuf {
    let p = Path::new(dest);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(p))
            .unwrap_or_else(|_| p.to_path_buf())
    }
}

fn infer_repo_url(path: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(path)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if url.is_empty() {
        None
    } else {
        Some(normalize_url(&url))
    }
}
