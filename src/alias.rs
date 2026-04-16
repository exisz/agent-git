/// Shell alias management for `alias git=agent-git`.
/// Supports install/uninstall/status for ~/.zshrc and ~/.bashrc.
use std::fs;
use std::path::PathBuf;

const ALIAS_LINE: &str = "alias git='agent-git'";
const FULL_ALIAS_LINE: &str = "alias git='agent-git' # agent-git alias";

fn shell_rc_files() -> Vec<PathBuf> {
    let home = dirs::home_dir().expect("Could not determine home directory");
    vec![home.join(".zshrc"), home.join(".bashrc")]
}

/// Install the git alias in shell rc files.
pub fn install_alias() {
    let files = shell_rc_files();
    let mut installed = false;

    for file in &files {
        if !file.exists() {
            continue;
        }

        let content = fs::read_to_string(file).unwrap_or_default();

        if content.contains(ALIAS_LINE) {
            println!("✓ Alias already present in {}", file.display());
            installed = true;
            continue;
        }

        // Append alias
        let new_content = if content.ends_with('\n') || content.is_empty() {
            format!("{}{}\n", content, FULL_ALIAS_LINE)
        } else {
            format!("{}\n{}\n", content, FULL_ALIAS_LINE)
        };

        match fs::write(file, new_content) {
            Ok(_) => {
                println!("✓ Alias installed in {}", file.display());
                installed = true;
            }
            Err(e) => {
                eprintln!("✗ Failed to write {}: {}", file.display(), e);
            }
        }
    }

    if installed {
        println!("\nRestart your shell or run: source ~/.zshrc");
    } else {
        eprintln!("No shell rc files found to install alias into.");
    }
}

/// Uninstall the git alias from shell rc files.
pub fn uninstall_alias() {
    let files = shell_rc_files();

    for file in &files {
        if !file.exists() {
            continue;
        }

        let content = fs::read_to_string(file).unwrap_or_default();

        if !content.contains(ALIAS_LINE) {
            println!("✓ No alias found in {}", file.display());
            continue;
        }

        // Remove lines containing the alias
        let new_content: String = content
            .lines()
            .filter(|line| !line.contains(ALIAS_LINE))
            .collect::<Vec<_>>()
            .join("\n");

        let new_content = if new_content.is_empty() {
            new_content
        } else {
            format!("{}\n", new_content)
        };

        match fs::write(file, new_content) {
            Ok(_) => println!("✓ Alias removed from {}", file.display()),
            Err(e) => eprintln!("✗ Failed to write {}: {}", file.display(), e),
        }
    }

    println!("\nRestart your shell or run: source ~/.zshrc");
}

/// Check alias status in shell rc files.
pub fn alias_status() {
    let files = shell_rc_files();
    let mut found_any = false;

    for file in &files {
        if !file.exists() {
            println!("  {} — file not found", file.display());
            continue;
        }

        let content = fs::read_to_string(file).unwrap_or_default();
        if content.contains(ALIAS_LINE) {
            println!("  ✓ {} — alias installed", file.display());
            found_any = true;
        } else {
            println!("  ✗ {} — alias not installed", file.display());
        }
    }

    if !found_any {
        println!("\nAlias is not installed. Run: agent-git alias install");
    }
}
