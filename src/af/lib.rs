pub mod cmd;
pub mod consts;
pub mod ides;
pub mod repo;
pub mod utils;

use crate::cmd::{
    browser::Browser, dot::DotCmd, git::Git, git::clone_project::CloneProject,
    shortcuts::abbreviations::Shortcut,
};
use crate::consts::AF;

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use indicatif::MultiProgress;
use log::LevelFilter;

const DEFAULT_LOG_LEVEL: LevelFilter = LevelFilter::Warn;

/// The afrael CLI tool
#[derive(Debug, Parser)] // requires `derive` feature
#[command(name = AF)]
#[command(multicall = true)]
pub enum Cli {
    #[command(flatten)]
    Applet(Applet),

    #[command(subcommand)]
    Af(Applet),
}

impl Cli {
    pub fn log_level_filter(&self) -> LevelFilter {
        match self {
            Cli::Applet(cmd) => cmd.log_level_filter(),
            Cli::Af(cmd) => cmd.log_level_filter(),
        }
    }

    pub async fn run(&self, multi: MultiProgress) -> Result<()> {
        match self {
            Cli::Applet(cmd) => cmd.run(multi).await,
            Cli::Af(cmd) => cmd.run(multi).await,
        }
    }
}

#[derive(Debug, Subcommand)]
#[command(version)]
pub enum Applet {
    /// Generate shell completion scripts
    Completions {
        /// Target shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete_command::Shell,
    },

    /// Helper commands related to dotfiles (defaults to `dot ide` if no subcommand is used)
    #[command(version)]
    Dot {
        /// Dotfile-related subcommands
        #[command(flatten)]
        dot: DotCmd,

        /// Increase output verbosity (-v, -vv, -vvv, etc.)
        #[command(flatten)]
        verbose: clap_verbosity_flag::Verbosity,
    },

    /// Git-related helper commands
    #[command(visible_alias = "g")]
    Git {
        /// Git subcommands
        #[command(subcommand)]
        git: Git,

        /// Increase output verbosity (-v, -vv, -vvv, etc.)
        #[command(flatten)]
        verbose: clap_verbosity_flag::Verbosity,
    },

    /// Shortcut for `af git clone-project`
    #[command(name = "pgc")]
    #[command(version)]
    ProjectGitClone {
        /// Clone project arguments
        #[command(flatten)]
        clone_project: CloneProject,

        /// Increase output verbosity (-v, -vv, -vvv, etc.)
        #[command(flatten)]
        verbose: clap_verbosity_flag::Verbosity,
    },

    /// Short aliases for common command combinations (e.g. gcmff)
    #[command(version)]
    Shortcuts {
        /// Abbreviated/shortcut commands
        #[command(subcommand)]
        shortcut: Shortcut,

        /// Increase output verbosity (-v, -vv, -vvv, etc.)
        #[command(flatten)]
        verbose: clap_verbosity_flag::Verbosity,
    },

    /// Open and inspect installed browsers
    #[command(version)]
    Browser {
        /// Browser-related subcommands
        #[command(subcommand)]
        browser: Browser,

        /// Increase output verbosity (-v, -vv, -vvv, etc.)
        #[command(flatten)]
        verbose: clap_verbosity_flag::Verbosity,
    },
}

impl Applet {
    fn log_level_filter(&self) -> LevelFilter {
        match self {
            Applet::Dot { verbose, .. } => verbose.log_level_filter(),
            Applet::Git { verbose, .. } => verbose.log_level_filter(),
            Applet::ProjectGitClone { verbose, .. } => verbose.log_level_filter(),
            Applet::Shortcuts { verbose, .. } => verbose.log_level_filter(),
            Applet::Browser { verbose, .. } => verbose.log_level_filter(),
            Applet::Completions { .. } => DEFAULT_LOG_LEVEL,
        }
    }

    pub async fn run(&self, multi: MultiProgress) -> Result<()> {
        match self {
            Applet::Completions { shell, .. } => {
                shell.generate(&mut Cli::command(), &mut std::io::stdout());
                Ok(())
            }

            Applet::Dot { dot, .. } => dot.run().await,

            Applet::Git { git, .. } => git.run(&multi).await,

            Applet::Shortcuts { shortcut, .. } => {
                shortcut.run();
                Ok(())
            }

            Applet::ProjectGitClone { clone_project, .. } => clone_project.run(&multi).await,

            Applet::Browser { browser, .. } => browser.run().await,
        }
    }
}
