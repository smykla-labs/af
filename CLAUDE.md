# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Org-managed files

The following files are synced from `smykla-skalski/.github` and must not be edited directly. Customize them via `.github/sync-config.yml` instead (schema: `https://raw.githubusercontent.com/smykla-skalski/.github/main/schemas/sync-config.schema.json`):

- `CODE_OF_CONDUCT.md`, `CONTRIBUTING.md`, `LICENSE`, `SECURITY.md`
- `renovate.json` (currently merged with repo-specific overrides in sync-config.yml)
- `.github/PULL_REQUEST_TEMPLATE.md`
- `.github/ISSUE_TEMPLATE/bug_report.yml`, `.github/ISSUE_TEMPLATE/config.yml`
- `.github/workflows/sync-trigger.yml`
- `.github/workflows/smyklot-pr-commands.yml`, `.github/workflows/smyklot-poll.yml`

## Build Commands

```bash
# Build
cargo build
cargo build --release

# Run
cargo run -- <args>

# Test
cargo test
cargo test --lib              # Library tests only
cargo test -p af              # Main package only

# Lint and format
cargo fmt
cargo clippy

# Generate documentation
mise run gen                  # Generate all docs
mise run gen::docs::markdown  # Markdown docs only
mise run gen::docs::man       # Man pages only
cargo genman                  # Alias for man page generation
cargo genmd                   # Alias for markdown generation

# Clean generated docs
mise run clean::gen

# Nix
nix build .#af               # Build with Nix
nix flake check              # Check flake
nix develop                  # Enter dev shell
```

## Architecture

**af** is a personal CLI tool built with Rust (edition 2024) using Clap for argument parsing. It uses a multicall binary pattern where the same binary can behave differently based on its invocation name.

### Core Structure

- `src/af/lib.rs` - Main CLI definition using Clap's derive API. Defines `Cli` (multicall enum) and `Applet` (subcommands)
- `src/bin/af.rs` - Binary entry point with async runtime (Tokio) and logging setup
- `src/af/cmd/` - Command implementations organized by feature

### Command Modules

- **git/clone_project** - Clone repositories with automatic IDE detection, SSH URL conversion, and remote renaming. Uses `git2` for Git operations and `octocrab` for GitHub API
- **shortcuts/abbreviations** - Shell command abbreviations (`gcmff`, `gp`, `gd`) that output git commands to stdout for shell evaluation
- **browser** - macOS browser management (find installed browsers, open URLs)
- **dot** - Dotfiles helper, opens dotfiles directory in appropriate IDE

### Key Patterns

- Async operations use Tokio runtime
- Progress bars via `indicatif` with `indicatif_log_bridge` for log integration
- Interactive prompts via `dialoguer` with `FuzzySelect` for IDE selection
- Repository URL parsing supports both SSH (`git@host:org/repo.git`) and HTTP formats with automatic conversion

### Workspace Members

- `af-alfred-workflow` - Alfred workflow integration using `powerpack` crate
- `xtasks/genman` - Man page generation using `clap_mangen`
- `xtasks/genmd` - Markdown documentation generation using `clap_markdown`

### Environment Variables

- `PROJECTS_PATH` - Root directory for cloning projects
- `DOTFILES_PATH` - Path to dotfiles directory
- `HOMEBREW_PREFIX` - Homebrew installation path (defaults to `/opt/homebrew`)

## Claude Code skills

The `git-stage-hunk` SAI plugin stages partial file changes without a TTY. Use `/stage-hunk` when only some changes in a file belong in the current commit, multiple sessions modified the same file, or `git add -p` is unavailable.

Install: `claude --plugin-dir ~/Projects/github.com/smykla-skalski/sai/git-stage-hunk/`
Modes: `--list`, `--hunk H1,H2`, `--pattern REGEX`, `--file PATH`, `--range FILE:S-E`, `--verify`, `--dry-run`
