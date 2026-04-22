```
                          _            _ _
   __ _  __ _  ___ _ __ | |_      __ _(_) |_
  / _` |/ _` |/ _ \ '_ \| __|    / _` | | __|
 | (_| | (_| |  __/ | | | |_    | (_| | | |_
  \__,_|\__, |\___|_| |_|\__|    \__, |_|\__|
        |___/                    |___/
```

# agent-git

[![Crates.io](https://img.shields.io/crates/v/agent-git)](https://crates.io/crates/agent-git)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![CI](https://github.com/exisz/agent-git/actions/workflows/ci.yml/badge.svg)](https://github.com/exisz/agent-git/actions/workflows/ci.yml)

**A git wrapper that tracks cloned repos and prevents duplicate clones.**

---

## Why?

You've cloned the same repo twice. Maybe three times. They're scattered across `~/projects`, `~/work`, `~/tmp`, and that random folder you made last Tuesday.

- 🔍 "Where did I clone that repo?"
- 🗑️ "Is this a duplicate?"
- 📁 "I have 200 repos and no idea where half of them are"

**agent-git** wraps your `git` command transparently. Every `git clone` is intercepted and tracked. Try to clone something you already have? It tells you where it is instead of wasting your disk space.

## Install

```bash
# One-liner (recommended)
curl -fsSL https://raw.githubusercontent.com/exisz/agent-git/master/install.sh | sh

# Homebrew
brew tap exisz/tap
brew install agent-git

# Cargo
cargo install agent-git
```

## Quick Start

```bash
# Install agent-git as `git` (intercepts ALL shells, recommended)
agent-git install
agent-git doctor       # verify subprocess interception is ✅ ACTIVE

# Now git clone is tracked automatically — even from build scripts and AI agents!
git clone https://github.com/user/repo

# Try cloning again — blocked!
git clone https://github.com/user/repo
# Error: Already cloned at /Users/you/projects/repo

# See all tracked repos
git list

# Find where a repo is
git whereis user/repo
```

> **Why `install` and not `alias`?** Shell aliases only fire in interactive shells. Build scripts, Makefiles, CI agents and AI tools call `bash -c "git ..."` which **skips aliases**. The PATH symlink installed by `agent-git install` is the only reliable interception point. The `alias` subcommand still exists but is **deprecated**.

## Commands

| Command | Description |
|---------|-------------|
| `agent-git install` | Symlink as `git` in PATH (RECOMMENDED — intercepts every shell) |
| `agent-git uninstall` | Remove the symlink |
| `agent-git doctor` | Diagnose install + interception |
| `agent-git alias install` | [DEPRECATED] Add `alias git=agent-git` to ~/.zshrc/.bashrc |
| `git clone <url>` | Clone with tracking |
| `git list` | List all tracked repos |
| `git whereis <query>` | Find where a repo is cloned |
| `git scan [dir]` | Scan directory for existing repos and track them |
| `git status` | Passthrough to real git (all non-agent commands work normally) |

## Comparison

| Feature | agent-git | [ghq](https://github.com/x-motemen/ghq) | [git-repo-manager](https://github.com/hakoerber/git-repo-manager) | [sirup](https://github.com/lensvol/sirup) |
|---------|-----------|-----|------------------|-------|
| Transparent git wrapper | ✅ | ❌ (separate command) | ❌ (separate command) | ❌ |
| Duplicate prevention | ✅ | ❌ | ❌ | ❌ |
| Clone tracking | ✅ | ✅ (enforced paths) | ✅ | ✅ |
| Free directory structure | ✅ | ❌ (ghq root required) | ❌ | ✅ |
| Repo search | ✅ | ✅ | ✅ | ❌ |
| Existing repo scanning | ✅ | ❌ | ❌ | ❌ |
| Shell alias approach | ✅ | ❌ | ❌ | ❌ |
| Zero behavior change | ✅ | ❌ | ❌ | ❌ |

**Key difference:** `ghq` and others force you into their directory structure. `agent-git` lets you clone wherever you want — it just remembers where everything went.

## How It Works

```
┌──────────────┐     ┌───────────────┐     ┌──────────────┐
│  git clone   │────▶│  agent-git    │────▶│  real git    │
│  (alias)     │     │  intercept    │     │  clone       │
└──────────────┘     └───────┬───────┘     └──────┬───────┘
                             │                     │
                     ┌───────▼───────┐     ┌──────▼───────┐
                     │  Check TOML   │     │  Track new   │
                     │  registry     │     │  clone path  │
                     └───────────────┘     └──────────────┘
```

1. **PATH symlink** (recommended) — `agent-git install` symlinks `<PATH-dir>/git → agent-git`. Every shell, interactive or not, that resolves `git` finds the wrapper first.
2. **Transparent proxy** — Non-clone commands pass straight through to real `git`
3. **Clone interception** — `git clone` checks the TOML registry first. Already cloned? Error with location. New? Clone normally and register.
4. **TOML registry** — `~/.agent-git/repos.toml` stores all tracked repos

## Configuration

Registry lives at `~/.agent-git/repos.toml`:

```toml
[[repos]]
url = "https://github.com/user/repo"
path = "/Users/you/projects/repo"
cloned_at = "2025-01-15T10:30:00Z"
```

## License

[MIT](LICENSE) © 2025 Exis
