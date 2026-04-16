# Contributing to agent-git

Thanks for your interest in contributing! 🎉

## Getting Started

1. Fork the repo
2. Clone your fork:
   ```bash
   git clone https://github.com/YOUR_USERNAME/agent-git.git
   cd agent-git
   ```
3. Create a branch:
   ```bash
   git checkout -b feat/my-feature
   ```

## Development

### Prerequisites

- Rust (stable, latest)
- `cargo fmt` and `cargo clippy` installed (included with rustup)

### Build & Test

```bash
# Build
cargo build

# Run tests
cargo test

# Format check
cargo fmt --check

# Lint
cargo clippy -- -D warnings
```

### Before Submitting

- [ ] `cargo fmt` — code is formatted
- [ ] `cargo clippy -- -D warnings` — no lint warnings
- [ ] `cargo test` — all tests pass
- [ ] Commit messages are clear and descriptive

## Pull Requests

1. Push your branch to your fork
2. Open a PR against `main`
3. Describe what changed and why
4. Link any related issues

## Issues

Found a bug? Have a feature idea? [Open an issue](https://github.com/exisz/agent-git/issues/new)!

- **Bug reports:** Include OS, Rust version, and steps to reproduce
- **Feature requests:** Describe the problem you're trying to solve

## Code Style

- Follow standard Rust conventions
- Use `cargo fmt` for formatting
- Keep functions focused and well-named
- Add tests for new functionality

## License

By contributing, you agree that your contributions will be licensed under the [MIT License](LICENSE).
