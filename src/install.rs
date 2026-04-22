/// PATH-based install for true subprocess interception.
///
/// Symlinks `agent-git` as `git` into a PATH directory that comes before /usr/bin
/// AND is included in default subprocess PATH. This is the ONLY way to intercept
/// `bash -c "git ..."` style invocations used by build scripts and AI agents.
/// Shell aliases do not survive non-interactive shells.
use std::env;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::Command;

const TARGET_BIN: &str = "git";
const SELF_BIN: &str = "agent-git";

fn candidate_dirs() -> Vec<PathBuf> {
    let mut v = vec![
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/usr/local/bin"),
    ];
    if let Some(h) = dirs::home_dir() {
        v.push(h.join(".local").join("bin"));
    }
    v
}

fn self_path() -> Option<PathBuf> {
    if let Ok(out) = Command::new("which").arg(SELF_BIN).output() {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !s.is_empty() {
                return Some(PathBuf::from(s));
            }
        }
    }
    env::current_exe().ok()
}

fn pick_dir(force: Option<&str>) -> Option<PathBuf> {
    if let Some(p) = force {
        let p = PathBuf::from(p);
        if p.is_dir() { return Some(p); }
        eprintln!("⚠️  --dir {} is not a directory", p.display());
        return None;
    }
    for d in candidate_dirs() {
        if d.is_dir() && is_writable(&d) {
            return Some(d);
        }
    }
    None
}

fn is_writable(p: &Path) -> bool {
    let probe = p.join(format!(".agent-git-probe.{}", std::process::id()));
    match fs::OpenOptions::new().create(true).write(true).truncate(true).open(&probe) {
        Ok(_) => { let _ = fs::remove_file(&probe); true }
        Err(_) => false,
    }
}

fn precedes_usr_bin(dir: &Path) -> bool {
    let path = env::var("PATH").unwrap_or_default();
    let mut found_dir = None;
    let mut found_usr = None;
    for (i, entry) in path.split(':').enumerate() {
        if Path::new(entry) == dir && found_dir.is_none() { found_dir = Some(i); }
        if entry == "/usr/bin" && found_usr.is_none() { found_usr = Some(i); }
    }
    match (found_dir, found_usr) {
        (Some(d), Some(u)) => d < u,
        (Some(_), None)    => true,
        _                  => false,
    }
}

pub fn install(force_dir: Option<String>, force: bool) {
    let Some(self_p) = self_path() else {
        eprintln!("❌ Could not locate the agent-git binary on PATH. Did you `cargo install agent-git`?");
        return;
    };

    let Some(dir) = pick_dir(force_dir.as_deref()) else {
        eprintln!("❌ No writable PATH directory found. Tried:");
        for d in candidate_dirs() {
            eprintln!("   - {} ({})", d.display(), if d.is_dir() { "exists" } else { "missing" });
        }
        eprintln!();
        eprintln!("Fixes:");
        eprintln!("  - Install Homebrew (provides /opt/homebrew/bin or /usr/local/bin)");
        eprintln!("  - Or pass --dir <writable-dir-on-PATH>");
        return;
    };

    let target = dir.join(TARGET_BIN);

    if target.symlink_metadata().is_ok() {
        let existing = fs::read_link(&target).ok();
        let same = existing.as_deref() == Some(self_p.as_path());
        if same {
            println!("✓ Already installed: {} -> {}", target.display(), self_p.display());
            print_doctor(&dir, &target);
            return;
        }
        if !force {
            eprintln!("❌ {} already exists and points to:", target.display());
            eprintln!("   {}", existing.map(|p| p.display().to_string()).unwrap_or_else(|| "(real file, not a symlink)".to_string()));
            eprintln!();
            eprintln!("⚠️  WARNING: replacing /git interception is a SIGNIFICANT operation. It changes");
            eprintln!("   what every script that calls `git` sees. Re-run with --force if intentional.");
            return;
        }
        if let Err(e) = fs::remove_file(&target) {
            eprintln!("❌ Could not remove existing {}: {}", target.display(), e);
            return;
        }
    }

    if let Err(e) = symlink(&self_p, &target) {
        eprintln!("❌ Failed to create symlink {} -> {}: {}", target.display(), self_p.display(), e);
        return;
    }

    println!("✅ Installed: {} -> {}", target.display(), self_p.display());
    print_doctor(&dir, &target);
}

pub fn uninstall(force_dir: Option<String>) {
    let dirs: Vec<PathBuf> = match force_dir {
        Some(d) => vec![PathBuf::from(d)],
        None => candidate_dirs(),
    };
    let mut removed = 0;
    for dir in dirs {
        let target = dir.join(TARGET_BIN);
        if target.symlink_metadata().is_ok() {
            let link = fs::read_link(&target).ok();
            let is_ours = link.as_deref().map(|p| p.file_name().and_then(|n| n.to_str()) == Some(SELF_BIN)).unwrap_or(false);
            if is_ours {
                match fs::remove_file(&target) {
                    Ok(_) => { println!("🗑  Removed {}", target.display()); removed += 1; }
                    Err(e) => eprintln!("⚠️  Failed to remove {}: {}", target.display(), e),
                }
            } else {
                println!("⚪ {} exists but is not an agent-git symlink — left alone", target.display());
            }
        }
    }
    if removed == 0 { println!("Nothing to uninstall."); }
}

pub fn doctor() {
    let self_p = self_path();
    println!("agent-git doctor");
    println!("  binary on PATH : {}", self_p.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "❌ NOT FOUND".to_string()));

    for dir in candidate_dirs() {
        let target = dir.join(TARGET_BIN);
        let exists = target.symlink_metadata().is_ok();
        let link = fs::read_link(&target).ok();
        let is_ours = link.as_deref().map(|p| p.file_name().and_then(|n| n.to_str()) == Some(SELF_BIN)).unwrap_or(false);
        let writable = is_writable(&dir);
        let precedes = precedes_usr_bin(&dir);
        println!("  {} ", dir.display());
        println!("    exists?    {}", if dir.is_dir() { "yes" } else { "no" });
        println!("    writable?  {}", writable);
        println!("    on PATH before /usr/bin? {}", precedes);
        println!("    {} present? {}{}",
            TARGET_BIN,
            if exists { "yes" } else { "no" },
            if is_ours { " (✅ agent-git symlink)" } else if exists { " (⚠️ NOT ours / real binary)" } else { "" });
    }

    if let Ok(out) = Command::new("bash").args(["-c", "command -v git"]).output() {
        let resolved = String::from_utf8_lossy(&out.stdout).trim().to_string();
        println!("  bash -c 'command -v git' -> {}", resolved);
        let ok = resolved.ends_with(&format!("/{}", TARGET_BIN))
            && fs::read_link(&resolved).map(|p| p.file_name().and_then(|n| n.to_str()) == Some(SELF_BIN)).unwrap_or(false);
        println!("  → subprocess interception: {}", if ok { "✅ ACTIVE" } else { "❌ NOT ACTIVE — run `agent-git install`" });
    }
}

fn print_doctor(dir: &Path, target: &Path) {
    let precedes = precedes_usr_bin(dir);
    println!();
    println!("  {} on PATH before /usr/bin? {}", dir.display(), if precedes { "yes ✅" } else { "no ❌ — interception will NOT work" });
    if !precedes {
        println!();
        println!("  Fix your shell rc to put {} earlier in PATH, e.g.:", dir.display());
        println!("    export PATH=\"{}:$PATH\"", dir.display());
    }
    println!();
    println!("  Verify: bash -c 'command -v git'");
    println!("          (should print {})", target.display());
}
