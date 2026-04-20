mod alias;
mod clone;
mod normalize;
mod passthrough;
mod registry;
mod scan;

use clap::{Parser, Subcommand};
use registry::Registry;
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "agent-git",
    about = "A git wrapper that tracks cloned repos and prevents duplicate clones",
    version,
    disable_help_subcommand = false,
    allow_external_subcommands = true
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Clone a repository (with duplicate detection)
    Clone {
        /// Repository URL to clone
        url: String,
        /// Destination directory (optional)
        dest: Option<String>,
    },

    /// List all tracked repositories
    List,

    /// Find where a repository is cloned
    Whereis {
        /// Repository URL or name to find
        query: String,
    },

    /// Register the current directory (or a given path) as a tracked repo
    Register {
        /// Path to register (defaults to current directory)
        path: Option<String>,
    },

    /// Unregister a tracked repo by path
    Unregister {
        /// Path to unregister
        path: String,
    },

    /// Scan a directory for git repos and register them
    Scan {
        /// Directory to scan (defaults to current directory)
        dir: Option<String>,
    },

    /// Manage shell alias (git → agent-git)
    Alias {
        #[command(subcommand)]
        action: AliasAction,
    },
}

#[derive(Subcommand)]
enum AliasAction {
    /// Install alias git=agent-git in shell rc files
    Install,
    /// Remove alias from shell rc files
    Uninstall,
    /// Check alias status
    Status,
}

fn main() -> ExitCode {
    // Check if we're being called with raw git arguments (no recognized subcommand)
    let args: Vec<String> = std::env::args().collect();

    // If called with no args, show help
    if args.len() < 2 {
        let cli = Cli::parse();
        // parse() with no args will show help or empty
        match cli.command {
            None => {
                // Show help
                use clap::CommandFactory;
                Cli::command().print_help().ok();
                println!();
                return ExitCode::SUCCESS;
            }
            _ => unreachable!(),
        }
    }

    // Try to parse as our CLI first
    // If the first arg matches a known subcommand, handle it
    // Otherwise, passthrough to git
    let first_arg = &args[1];

    // Known subcommands we handle
    let known = [
        "clone",
        "list",
        "whereis",
        "register",
        "unregister",
        "scan",
        "alias",
        "help",
        "--help",
        "-h",
        "--version",
        "-V",
    ];

    if !known.contains(&first_arg.as_str()) {
        // Passthrough to real git
        return passthrough::passthrough(&args[1..]);
    }

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Clone { url, dest }) => clone::handle_clone(&url, dest.as_deref()),

        Some(Commands::List) => {
            let registry = Registry::load();
            if registry.repos.is_empty() {
                println!("No repositories tracked. Use 'agent-git clone' or 'agent-git scan' to add repos.");
                return ExitCode::SUCCESS;
            }
            println!("{:<50} {:<50} {}", "REPOSITORY", "PATH", "CLONED AT");
            println!("{}", "-".repeat(130));
            for repo in &registry.repos {
                println!(
                    "{:<50} {:<50} {}",
                    repo.url,
                    repo.path,
                    repo.cloned_at.format("%Y-%m-%d %H:%M")
                );
            }
            println!("\n{} repositories tracked", registry.repos.len());
            ExitCode::SUCCESS
        }

        Some(Commands::Whereis { query }) => {
            let registry = Registry::load();
            let normalized = normalize::normalize_url(&query);

            // Search by normalized URL
            if let Some(repo) = registry.find_by_url(&normalized) {
                println!("{}", repo.path);
                return ExitCode::SUCCESS;
            }

            // Search by partial match
            let matches: Vec<_> = registry
                .repos
                .iter()
                .filter(|r| r.url.contains(&query) || r.path.contains(&query))
                .collect();

            if matches.is_empty() {
                eprintln!("Repository '{}' not found in registry", query);
                ExitCode::from(1)
            } else {
                for repo in &matches {
                    println!("{} → {}", repo.url, repo.path);
                }
                ExitCode::SUCCESS
            }
        }

        Some(Commands::Register { path }) => {
            let path = path.unwrap_or_else(|| ".".to_string());
            let abs_path = match std::path::Path::new(&path).canonicalize() {
                Ok(p) => p.to_string_lossy().to_string(),
                Err(e) => {
                    eprintln!("error: Cannot resolve path '{}': {}", path, e);
                    return ExitCode::from(1);
                }
            };

            // Check if it's a git repo
            if !std::path::Path::new(&abs_path).join(".git").exists() {
                eprintln!("error: '{}' is not a git repository", abs_path);
                return ExitCode::from(1);
            }

            // Try to get remote URL
            let url = std::process::Command::new("git")
                .args(["remote", "get-url", "origin"])
                .current_dir(&abs_path)
                .output()
                .ok()
                .and_then(|o| {
                    if o.status.success() {
                        let u = String::from_utf8_lossy(&o.stdout).trim().to_string();
                        if u.is_empty() {
                            None
                        } else {
                            Some(normalize::normalize_url(&u))
                        }
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| format!("local:{}", abs_path));

            let mut registry = Registry::load();
            match registry.register(url.clone(), abs_path.clone()) {
                Ok(_) => {
                    registry.save().ok();
                    println!("Registered: {} → {}", url, abs_path);
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("{}", e);
                    ExitCode::from(1)
                }
            }
        }

        Some(Commands::Unregister { path }) => {
            let abs_path = match std::path::Path::new(&path).canonicalize() {
                Ok(p) => p.to_string_lossy().to_string(),
                Err(_) => path.clone(),
            };

            let mut registry = Registry::load();
            if registry.unregister_by_path(&abs_path) || registry.unregister_by_path(&path) {
                registry.save().ok();
                println!("Unregistered: {}", abs_path);
                ExitCode::SUCCESS
            } else {
                // Try by URL
                if registry.unregister_by_url(&path) {
                    registry.save().ok();
                    println!("Unregistered: {}", path);
                    ExitCode::SUCCESS
                } else {
                    eprintln!("Not found in registry: {}", path);
                    ExitCode::from(1)
                }
            }
        }

        Some(Commands::Scan { dir }) => {
            let dir = dir.unwrap_or_else(|| ".".to_string());
            scan::scan_directory(&dir);
            ExitCode::SUCCESS
        }

        Some(Commands::Alias { action }) => {
            match action {
                AliasAction::Install => alias::install_alias(),
                AliasAction::Uninstall => alias::uninstall_alias(),
                AliasAction::Status => alias::alias_status(),
            }
            ExitCode::SUCCESS
        }

        None => {
            use clap::CommandFactory;
            Cli::command().print_help().ok();
            println!();
            ExitCode::SUCCESS
        }
    }
}
