use crate::consts::*;
use crate::repo::Repo;
use crate::{ides, utils};
use anyhow::{Context, Result, anyhow};
use clap::{Args, ValueHint, value_parser};
use clio::ClioPath;
use console::style;
use dialoguer::{Confirm, FuzzySelect, Input, theme::ColorfulTheme};
use git2::{
    Config, Cred, CredentialType, FetchOptions, RemoteCallbacks, Repository, StatusOptions,
    build::RepoBuilder,
};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use log::{debug, info, trace};
use std::path::Path;
use std::{fs, time::Duration};
use thiserror::Error;
use tokio::task;

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
                    .map(|root| root.clone().join(&repo.org).join(&repo.name))
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

        if let Err(err) = &cloned_repo_maybe
            && let Some(err) = err.downcast_ref::<CloneRepositoryError>() {
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
    async fn open_ide_maybe(&self, repo: &Repo, directory: &ClioPath) -> Result<()> {
        if !self.open_ide {
            return Ok(());
        }

        let repo_ide = repo.find_ide().await?;
        debug!("Repository IDE: {:?}", repo_ide);

        let (launchers, index) = task::spawn_blocking(move || {
            let launchers = ides::discover();
            let index = ides::select_for_clone(&launchers, repo_ide);
            (launchers, index)
        })
        .await?;

        if launchers.is_empty() {
            info!("No supported IDE launchers found on PATH, skipping IDE selection");
            return Ok(());
        }

        let labels = launchers
            .iter()
            .map(|launcher| launcher.label().to_string())
            .collect::<Vec<_>>();

        let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
            .with_prompt("Select an IDE to open the project, or press 'Esc' to skip")
            .default(index.unwrap_or_default())
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

            utils::run_command(command, &[directory]).await?;
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
    repo: &Repo,
    url: &str,
    directory: &ClioPath,
    force: bool,
) -> Result<Repository> {
    let directory_clone = directory.clone();
    let target_state =
        task::spawn_blocking(move || inspect_target_directory(&directory_clone)).await??;
    let replacement = replacement_action(target_state, force);

    match replacement {
        ReplacementAction::CloneDirectly => {}
        ReplacementAction::Prompt(prompt_kind) => confirm_replace(prompt_kind, directory)?,
        ReplacementAction::RemoveWithoutPrompt => {}
    }

    if !matches!(replacement, ReplacementAction::CloneDirectly) {
        info!(
            "Removing existing directory: {}",
            style(utils::format_directory(directory)).bold(),
        );
        let dir_path = directory.to_path_buf();
        task::spawn_blocking(move || fs::remove_dir_all(dir_path)).await??;
    }

    let pb = mp.add(ProgressBar::no_length().with_message("Cloning"));
    pb.enable_steady_tick(Duration::from_millis(80));
    pb.set_style(
        ProgressStyle::with_template(
            "{msg:.green.bold} {spinner}[{elapsed_precise}] {wide_bar} {percent:>3}%",
        )?
        .tick_strings(&["⢎  ", "⠎⠁ ", "⠊⠑ ", "⠈⠱ ", " ⡱ ", "⢀⡰ ", "⢄⡠ ", "⢆⡀ ", ""]),
    );

    let url = url.to_string();
    let directory_path = directory.to_path_buf();
    let pb_clone = pb.clone();
    let username = repo.username.clone();

    info!(
        "Cloning {} into {}",
        style(repo.short_format()).bold(),
        style(utils::format_directory(directory)).bold(),
    );

    let cloned_repo = task::spawn_blocking(move || {
        let mut callbacks = RemoteCallbacks::new();
        let git_config = Config::open_default()?;

        callbacks.credentials(move |remote_url, username_from_url, allowed| {
            match select_credential_strategy(remote_url, username_from_url, allowed, &username) {
                Ok(CredentialStrategy::Username(name)) => Cred::username(&name),
                Ok(CredentialStrategy::SshAgent(name)) => Cred::ssh_key_from_agent(&name),
                Ok(CredentialStrategy::CredentialHelper(helper_username)) => {
                    Cred::credential_helper(&git_config, remote_url, helper_username.as_deref())
                }
                Ok(CredentialStrategy::Default) => Cred::default(),
                Err(err) => Err(err),
            }
        });

        callbacks.transfer_progress(move |progress| {
            if pb_clone.length().is_none() {
                pb_clone.set_length(progress.total_objects() as u64);
            }

            pb_clone.set_position(progress.received_objects() as u64);

            true
        });

        let mut fetch_opts = FetchOptions::new();
        fetch_opts.remote_callbacks(callbacks);

        let mut builder = RepoBuilder::new();
        builder.fetch_options(fetch_opts);

        builder.clone(&url, &directory_path)
    })
    .await??;

    pb.finish_and_clear();

    pb.println(format!(
        "Project {} was cloned to {}",
        style(repo.short_format()).bold(),
        style(utils::format_directory(directory)).bold(),
    ));

    mp.remove(&pb);

    Ok(cloned_repo)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TargetDirectoryState {
    MissingOrEmpty,
    NonGitDirectory,
    GitRepository { dirty: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PromptKind {
    DirtyGitRepository,
    CleanGitRepository,
    NonGitDirectory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReplacementAction {
    CloneDirectly,
    Prompt(PromptKind),
    RemoveWithoutPrompt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CredentialStrategy {
    Username(String),
    SshAgent(String),
    CredentialHelper(Option<String>),
    Default,
}

fn inspect_target_directory(directory: &Path) -> Result<TargetDirectoryState> {
    if !directory.exists() {
        return Ok(TargetDirectoryState::MissingOrEmpty);
    }

    if directory.read_dir()?.next().is_none() {
        return Ok(TargetDirectoryState::MissingOrEmpty);
    }

    if let Ok(repository) = Repository::open(directory) {
        debug!("{} is a git repository", directory.display());

        let dirty = !repository
            .statuses(Some(StatusOptions::new().include_untracked(true)))?
            .is_empty();

        return Ok(TargetDirectoryState::GitRepository { dirty });
    }

    Ok(TargetDirectoryState::NonGitDirectory)
}

fn replacement_action(state: TargetDirectoryState, force: bool) -> ReplacementAction {
    match state {
        TargetDirectoryState::MissingOrEmpty => ReplacementAction::CloneDirectly,
        TargetDirectoryState::NonGitDirectory if force => ReplacementAction::RemoveWithoutPrompt,
        TargetDirectoryState::NonGitDirectory => {
            ReplacementAction::Prompt(PromptKind::NonGitDirectory)
        }
        TargetDirectoryState::GitRepository { dirty: true } => {
            ReplacementAction::Prompt(PromptKind::DirtyGitRepository)
        }
        TargetDirectoryState::GitRepository { dirty: false } if force => {
            ReplacementAction::RemoveWithoutPrompt
        }
        TargetDirectoryState::GitRepository { dirty: false } => {
            ReplacementAction::Prompt(PromptKind::CleanGitRepository)
        }
    }
}

fn confirm_replace(prompt_kind: PromptKind, directory: &ClioPath) -> Result<()> {
    let dir = style(utils::format_directory(directory)).bold();

    let prompt = match prompt_kind {
        PromptKind::DirtyGitRepository => {
            debug!("{dir} contains uncommitted changes");
            println!("{dir} exists and is a Git repository with uncommitted changes");

            format!(
                "{} {} {}",
                style("Are you").yellow().bold(),
                style(" REALLY ").bold().red().reverse(),
                style("sure you want to continue and remove it?")
                    .yellow()
                    .bold()
            )
        }
        PromptKind::CleanGitRepository => format!(
            "{dir} is a Git repository in a clean state. {}",
            style("Are you sure you want to continue and remove it?").yellow(),
        ),
        PromptKind::NonGitDirectory => format!(
            "{dir} exists and is not a Git repository. {}",
            style("Are you sure you want to continue and remove it?").yellow(),
        ),
    };

    let confirmed = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(&prompt)
        .interact()?;

    if !confirmed {
        debug!("Aborting");
        return Err(CloneRepositoryError::OperationCancelled.into());
    }

    Ok(())
}

fn select_credential_strategy(
    remote_url: &str,
    username_from_url: Option<&str>,
    allowed: CredentialType,
    fallback_username: &str,
) -> std::result::Result<CredentialStrategy, git2::Error> {
    if remote_url.starts_with("http://") || remote_url.starts_with("https://") {
        if allowed.contains(CredentialType::USER_PASS_PLAINTEXT) {
            return Ok(CredentialStrategy::CredentialHelper(
                username_from_url.map(std::string::ToString::to_string),
            ));
        }

        if allowed.contains(CredentialType::DEFAULT) {
            return Ok(CredentialStrategy::Default);
        }
    } else {
        let username = username_from_url.unwrap_or(fallback_username).to_string();

        if allowed.contains(CredentialType::USERNAME) {
            return Ok(CredentialStrategy::Username(username));
        }

        if allowed.contains(CredentialType::SSH_KEY) {
            return Ok(CredentialStrategy::SshAgent(username));
        }
    }

    Err(git2::Error::from_str(
        "no supported authentication methods available",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn classifies_non_git_directories() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("file.txt"), "content").unwrap();

        let state = inspect_target_directory(dir.path()).unwrap();
        assert_eq!(state, TargetDirectoryState::NonGitDirectory);
    }

    #[test]
    fn classifies_clean_git_repository() {
        let dir = TempDir::new().unwrap();
        Repository::init(dir.path()).unwrap();

        let state = inspect_target_directory(dir.path()).unwrap();
        assert_eq!(state, TargetDirectoryState::GitRepository { dirty: false });
    }

    #[test]
    fn classifies_dirty_git_repository() {
        let dir = TempDir::new().unwrap();
        Repository::init(dir.path()).unwrap();
        fs::write(dir.path().join("file.txt"), "content").unwrap();

        let state = inspect_target_directory(dir.path()).unwrap();
        assert_eq!(state, TargetDirectoryState::GitRepository { dirty: true });
    }

    #[test]
    fn non_git_directory_requires_confirmation_without_force() {
        let action = replacement_action(TargetDirectoryState::NonGitDirectory, false);
        assert_eq!(
            action,
            ReplacementAction::Prompt(PromptKind::NonGitDirectory)
        );
    }

    #[test]
    fn non_git_directory_is_removed_with_force() {
        let action = replacement_action(TargetDirectoryState::NonGitDirectory, true);
        assert_eq!(action, ReplacementAction::RemoveWithoutPrompt);
    }

    #[test]
    fn https_uses_credential_helper_when_plaintext_auth_is_allowed() {
        let strategy = select_credential_strategy(
            "https://github.com/org/repo",
            Some("oauth2"),
            CredentialType::USER_PASS_PLAINTEXT,
            "git",
        )
        .unwrap();
        assert_eq!(
            strategy,
            CredentialStrategy::CredentialHelper(Some("oauth2".to_string()))
        );
    }

    #[test]
    fn ssh_uses_username_probe_before_agent_auth() {
        let strategy = select_credential_strategy(
            "git@github.com:org/repo.git",
            None,
            CredentialType::USERNAME,
            "git",
        )
        .unwrap();
        assert_eq!(strategy, CredentialStrategy::Username("git".to_string()));
    }

    #[test]
    fn ssh_uses_agent_when_ssh_key_auth_is_allowed() {
        let strategy = select_credential_strategy(
            "git@github.com:org/repo.git",
            Some("git"),
            CredentialType::SSH_KEY,
            "fallback",
        )
        .unwrap();
        assert_eq!(strategy, CredentialStrategy::SshAgent("git".to_string()));
    }

    #[test]
    fn unsupported_auth_strategy_returns_error() {
        let err = select_credential_strategy(
            "https://github.com/org/repo",
            None,
            CredentialType::empty(),
            "git",
        )
        .unwrap_err();
        assert!(
            err.message()
                .contains("no supported authentication methods available")
        );
    }
}
