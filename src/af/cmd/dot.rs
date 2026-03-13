use crate::{ides, utils};
use anyhow::{Context, Result, anyhow};
use clap::{Args, Subcommand, ValueHint, value_parser};
use clio::ClioPath;

#[derive(Debug, Args)]
#[command(visible_alias = ".")]
#[command(args_conflicts_with_subcommands = true)]
#[command(flatten_help = true)]
#[command(disable_help_subcommand = true)]
pub struct DotCmd {
    /// Optional dotfiles subcommand
    #[command(subcommand)]
    pub command: Option<DotCommands>,

    /// IDE-related options (used when no subcommand is given)
    #[command(flatten)]
    pub ide: Ide,
}

impl DotCmd {
    pub fn run(&self) -> Result<()> {
        match &self.command {
            Some(DotCommands::Ide(args)) => args.run(),
            None => self.ide.run(),
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum DotCommands {
    /// Open the dotfiles directory in an IDE
    ///
    /// If inside a JetBrains IDE, it will use that IDE to open the path.
    /// Otherwise, it prefers Zed, then GoLand, then VS Code.
    Ide(Ide),
}

#[derive(Debug, Args)]
pub struct Ide {
    /// Path to the dotfiles directory (overrides $DOTFILES_PATH)
    #[arg(
        long,
        env = "DOTFILES_PATH",
        value_hint = ValueHint::DirPath,
        value_parser = value_parser!(ClioPath).exists().is_dir(),
    )]
    pub path: Option<ClioPath>,
}

impl Ide {
    pub fn run(&self) -> Result<()> {
        let launchers = ides::discover();
        let index = ides::select_for_dot(&launchers).ok_or_else(|| {
            anyhow!("No supported IDE launcher found on PATH for dot ide fallback order")
        })?;

        if let Some(p) = &self.path {
            let command = launchers[index]
                .path()
                .to_str()
                .context("IDE launcher path is not valid UTF-8")?;
            let path = p.to_str().context("dotfiles path is not valid UTF-8")?;

            utils::run_command(command, &[path])?;
        }

        Ok(())
    }
}
