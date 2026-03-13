use crate::consts::*;
use crate::repo::Repo;
use crate::{ides, utils};
use anyhow::{Context, Result, anyhow};
use clap::{Args, ValueHint, value_parser};
use clio::ClioPath;
use console::style;
use dialoguer::{Confirm, FuzzySelect, Input, theme::ColorfulTheme};
use git2::{Cred, FetchOptions, RemoteCallbacks, Repository, StatusOptions, build::RepoBuilder};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use log::{debug, info, trace};
use std::{fs, time::Duration};
use thiserror::Error;

/// Clone a project repository and optionally open it in an IDE
#[derive(Debug, Args)]
pub struct CloneProject {
    /// The repository URL to clone (e.g. git@github.com:org/project.git)
    #[arg(value_parser = utils::parse_repository)]
    repository_url: Option<String>,

    /// Open the cloned repository in an installed IDE picker, preselecting the best match
    #[arg(long, default_value_t = true, require_equals = true)]
    open_ide: std::primitive::bool,

    /// Force re-cloning even if the destination exists
    #[arg(long, short)]
    force: bool,

    /// Root directory for placing the cloned project (uses $PROJECTS_PATH if set)
    #[arg(
        long,
        env = "PROJECTS_PATH",
        required_unless_present = "directory",
        value_hint = ValueHint::DirPath,
        value_parser = value_parser!(ClioPath).exists().is_dir(),
    )]
    root_directory: Option<ClioPath>,

    /// Exact directory path to clone into (overrides root-directory)
    #[arg(long, value_hint = ValueHint::DirPath)]
    directory: Option<ClioPath>,

    /// Rename remote "origin" to "upstream" after cloning
    #[arg(long, default_value_t = true, require_equals = true)]
    rename_origin: std::primitive::bool,

    /// If used URL is in HTTP(S) format, convert it to SSH format before cloning
    #[arg(long, default_value_t = true, require_equals = true)]
    convert_to_ssh: std::primitive::bool,
}

impl CloneProject {
    pub async fn run(&self, multi_progress: &MultiProgress) -> Result<()> {
        trace!("Arguments: {:?}", self);

        let repository_url = match &self.repository_url {
            Some(url) => self.parse_repository(url)?,
            None => {
                let theme = &ColorfulTheme::default();

                let mut input = Input::with_theme(theme)
                    .with_prompt("Provide project's repository url you wish to clone")
                    .validate_with(|a: &String| utils::validate_repository(a));

                let clipboard = cli_clipboard::get_contents()
                    .unwrap_or_default()
                    .trim()
                    .to_string();

                if utils::validate_repository(&clipboard).is_ok() {
                    info!("Using clipboard contents: {}", &clipboard);
                    input = input.default(self.parse_repository(clipboard)?);
                }

                input.interact()?
            }
        };

        let repo = Repo::parse(&repository_url)?;
        let directory = self
            .directory
            .clone()
            .or_else(|| {
                self.root_directory
                    .clone()
                    .map(|root| root.clone().join(repo.org).join(repo.name))
            })
            .ok_or_else(|| {
                anyhow!("At least one of --directory or --root-directory must be provided")
            })?;

        // Clone repository with progress
        let cloned_repo_maybe = clone_repository(
            multi_progress,
            &repo,
            &repository_url,
            &directory,
            self.force,
        )
        .await;

        if let Err(err) = &cloned_repo_maybe {
            if let Some(err) = err.downcast_ref::<CloneRepositoryError>() {
                return match err {
                    CloneRepositoryError::OperationCancelled => {
                        let confirmed = Confirm::with_theme(&ColorfulTheme::default())
                            .with_prompt("Do you want to open the repository in IDE?")
                            .interact()?;

                        if !confirmed {
                            return Ok(());
                        }

                        self.open_ide_maybe(&repo, &directory).await
                    }
                };
            }
        }

        let cloned_repo = cloned_repo_maybe?;

        // Rename origin if required
        if self.rename_origin {
            cloned_repo.remote_rename(ORIGIN, UPSTREAM)?;
        }

        // Open IDE if requested
        self.open_ide_maybe(&repo, &directory).await?;

        Ok(())
    }

    /// Opens the cloned project in an IDE if available.
    async fn open_ide_maybe(&self, repo: &Repo<'_>, directory: &ClioPath) -> Result<()> {
        if !self.open_ide {
            return Ok(());
        }

        let repo_ide = repo.find_ide().await?;
        debug!("Repository IDE: {:?}", repo_ide);

        let launchers = ides::discover();
        if launchers.is_empty() {
            info!("No supported IDE launchers found on PATH, skipping IDE selection");
            return Ok(());
        }

        let labels = launchers
            .iter()
            .map(|launcher| launcher.label().to_string())
            .collect::<Vec<_>>();
        let index = ides::select_for_clone(&launchers, repo_ide).unwrap_or_default();

        let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
            .with_prompt("Select an IDE to open the project, or press 'Esc' to skip")
            .default(index)
            .items(&labels)
            .interact_opt()?;

        if let Some(selected) = selection {
            let command = launchers[selected]
                .path()
                .to_str()
                .context("IDE launcher path is not valid UTF-8")?;
            let directory = directory
                .to_str()
                .context("repository directory is not valid UTF-8")?;

            utils::run_command(command, &[directory])?;
        }

        Ok(())
    }

    fn parse_repository<S: AsRef<str>>(&self, s: S) -> Result<String> {
        if !self.convert_to_ssh {
            return utils::parse_repository(s.as_ref());
        }

        utils::convert_to_ssh(s.as_ref())
    }
}

#[derive(Error, Debug)]
enum CloneRepositoryError {
    #[error("Operation Canceled")]
    OperationCancelled,
}

/// Clones a repository with real-time progress updates.
async fn clone_repository(
    mp: &MultiProgress,
    repo: &Repo<'_>,
    url: &str,
    directory: &ClioPath,
    force: bool,
) -> Result<Repository> {
    if directory.exists() && directory.read_dir()?.next().is_some() {
        if let Ok(r) = Repository::open(directory.to_path_buf()) {
            debug!("{} is a git repository", utils::format_directory(directory));

            let dirty = !r
                .statuses(Some(StatusOptions::new().include_untracked(true)))?
                .is_empty();

            if dirty {
                debug!(
                    "{} contains uncommitted changes",
                    utils::format_directory(directory)
                );

                println!(
                    "{} exists and is a Git repository with uncommitted changes",
                    style(utils::format_directory(directory)).bold(),
                );

                let are_you_sure = format!(
                    "{} {} {}",
                    style("Are you").yellow().bold(),
                    style(" REALLY ").bold().red().reverse(),
                    style("sure you want to continue and remove it?")
                        .yellow()
                        .bold()
                );

                let confirmed = Confirm::with_theme(&ColorfulTheme::default())
                    .with_prompt(are_you_sure)
                    .interact()?;

                if !confirmed {
                    debug!("Aborting");
                    return Err(CloneRepositoryError::OperationCancelled.into());
                }
            } else if !force {
                let msg = format!(
                    "{} is a Git repository in a clean state. {}",
                    style(utils::format_directory(directory)).bold(),
                    style("Are you sure you want to continue and remove it?").yellow(),
                );

                let confirmed = Confirm::with_theme(&ColorfulTheme::default())
                    .with_prompt(msg.as_str())
                    .interact()?;

                if !confirmed {
                    debug!("Aborting");
                    return Err(CloneRepositoryError::OperationCancelled.into());
                }
            }
        }

        info!(
            "Removing existing directory: {}",
            style(utils::format_directory(directory)).bold(),
        );
        fs::remove_dir_all(directory.to_path_buf())?;
    }

    let pb = mp.add(ProgressBar::no_length().with_message("Cloning"));
    pb.enable_steady_tick(Duration::from_millis(80));
    pb.set_style(
        ProgressStyle::with_template(
            "{msg:.green.bold} {spinner}[{elapsed_precise}] {wide_bar} {percent:>3}%",
        )?
        .tick_strings(&["⢎  ", "⠎⠁ ", "⠊⠑ ", "⠈⠱ ", " ⡱ ", "⢀⡰ ", "⢄⡠ ", "⢆⡀ ", ""]),
    );

    let mut callbacks = RemoteCallbacks::new();

    callbacks
        .credentials(|_url, _username_from_url, _allowed| Cred::ssh_key_from_agent(repo.username));

    callbacks.transfer_progress(|progress| {
        if pb.length().is_none() {
            pb.set_length(progress.total_objects() as u64);
        }

        pb.set_position(progress.received_objects() as u64);

        true
    });

    let mut fetch_opts = FetchOptions::new();
    fetch_opts.remote_callbacks(callbacks);

    let mut builder = RepoBuilder::new();
    builder.fetch_options(fetch_opts);

    info!(
        "Cloning {} into {}",
        style(repo.short_format()).bold(),
        style(utils::format_directory(directory)).bold(),
    );
    let cloned_repo = builder.clone(url, directory)?;

    pb.finish_and_clear();

    pb.println(format!(
        "Project {} was cloned to {}",
        style(repo.short_format()).bold(),
        style(utils::format_directory(directory)).bold(),
    ));

    mp.remove(&pb);

    Ok(cloned_repo)
}
