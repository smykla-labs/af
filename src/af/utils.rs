use crate::consts::*;
use anyhow::{Context, Result};
use clio::ClioPath;
use console::style;
use fern::Dispatch;
use fern::colors::{Color, ColoredLevelConfig};
use log::{LevelFilter, trace};
use regex::Regex;
use std::process::Output;
use std::sync::OnceLock;
use std::time::SystemTime;
use std::{env, io};
use tokio::process::Command;
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RepositoryTransport {
    Ssh,
    Http,
    Https,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RepositoryParts {
    pub transport: RepositoryTransport,
    pub host: String,
    pub org: String,
    pub name: String,
}

impl RepositoryParts {
    pub(crate) fn canonical_url(&self) -> String {
        match self.transport {
            RepositoryTransport::Ssh => self.ssh_url(),
            RepositoryTransport::Http => {
                format!("http://{}/{}/{}", self.host, self.org, self.name)
            }
            RepositoryTransport::Https => {
                format!("https://{}/{}/{}", self.host, self.org, self.name)
            }
        }
    }

    pub(crate) fn ssh_url(&self) -> String {
        format!("git@{}:{}/{}.git", self.host, self.org, self.name)
    }
}

pub async fn run_command(command: &str, args: &[&str]) -> Result<Output> {
    let output = Command::new(command)
        .args(args)
        .output()
        .await
        .with_context(|| format!("Failed to execute command: {} {:?}", command, args))?;

    trace!(
        "Running '{} {}'",
        style(command).bold(),
        style(args.join(" ")).bold()
    );
    trace!("  Status: {}", output.status);
    trace!("  Stdout: {}", String::from_utf8_lossy(&output.stdout));
    trace!("  Stderr: {}", String::from_utf8_lossy(&output.stderr));

    Ok(output)
}

pub fn format_directory(directory: &ClioPath) -> String {
    directory
        .display()
        .to_string()
        .replace(env::var(HOME).unwrap_or_default().as_str(), "~")
}

pub fn parse_repository(repository: &str) -> Result<String> {
    parse_repository_parts(repository).map(|parts| parts.canonical_url())
}

pub fn validate_repository(repository: &str) -> Result<()> {
    parse_repository(repository).map(|_| ())
}

pub fn convert_to_ssh<S: AsRef<str>>(repository: S) -> Result<String> {
    parse_repository_parts(repository.as_ref()).map(|parts| parts.ssh_url())
}

pub(crate) fn parse_repository_parts(repository: &str) -> Result<RepositoryParts> {
    let repo = repository.trim();

    static SSH_RE: OnceLock<Regex> = OnceLock::new();
    let re_ssh =
        SSH_RE.get_or_init(|| Regex::new(r"^git@([^/:]+):([^/]+)/([^/]+)\.git$").unwrap());

    if let Some(captures) = re_ssh.captures(repo) {
        return Ok(RepositoryParts {
            transport: RepositoryTransport::Ssh,
            host: captures.get(1).unwrap().as_str().to_string(),
            org: captures.get(2).unwrap().as_str().to_string(),
            name: captures.get(3).unwrap().as_str().to_string(),
        });
    }

    let parsed = Url::parse(repo).map_err(|_| unsupported_repository_url())?;

    let transport = match parsed.scheme() {
        "http" => RepositoryTransport::Http,
        "https" => RepositoryTransport::Https,
        _ => return Err(unsupported_repository_url()),
    };

    let host = parsed
        .host_str()
        .ok_or_else(unsupported_repository_url)?
        .to_string();

    let mut segments = parsed
        .path_segments()
        .ok_or_else(unsupported_repository_url)?
        .filter(|segment| !segment.is_empty());

    let org = segments
        .next()
        .ok_or_else(unsupported_repository_url)?
        .to_string();
    let name = segments
        .next()
        .ok_or_else(unsupported_repository_url)?
        .trim_end_matches(".git")
        .to_string();

    if name.is_empty() {
        return Err(unsupported_repository_url());
    }

    Ok(RepositoryParts {
        transport,
        host,
        org,
        name,
    })
}

fn unsupported_repository_url() -> anyhow::Error {
    anyhow::anyhow!(
        "Unsupported repository URL. Supported formats: [{}, {}]",
        style("git@<host>:<org>/<repo>.git").bold(),
        style("http[s]://<host>/<org>/<repo>[/...]").bold(),
    )
}

pub fn setup_logger(level: LevelFilter) -> (LevelFilter, Box<dyn log::Log>) {
    let colors = ColoredLevelConfig::new()
        .error(Color::Red)
        .warn(Color::Yellow)
        .info(Color::Green)
        .debug(Color::Blue)
        .trace(MUTED_TEAL);

    Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{left_bracket}{timestamp} {level} {target}{right_bracket} {message}",
                left_bracket = format_args!("\x1B[{}m[\x1B[0m", GREY.to_fg_str()),
                timestamp = humantime::format_rfc3339_seconds(SystemTime::now()),
                level = colors.color(record.level()),
                target = record.target(),
                right_bracket = format_args!("\x1B[{}m]\x1B[0m", GREY.to_fg_str()),
                message = message
            ))
        })
        .level(level)
        .chain(io::stderr())
        .into_log()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_ssh_url() {
        let url = "git@github.com:org/repo.git";
        let parsed = parse_repository(url).unwrap();
        assert_eq!(parsed, url.trim());
    }

    #[test]
    fn parses_valid_https_url() {
        let url = "https://github.com/org/repo";
        let parsed = parse_repository(url).unwrap();
        assert_eq!(parsed, url);
    }

    #[test]
    fn parses_valid_https_url_with_trailing_path() {
        let url = "https://github.com/org/repo/tree/main";
        let parsed = parse_repository(url).unwrap();
        assert_eq!(parsed, "https://github.com/org/repo");
    }

    #[test]
    fn parses_valid_https_url_with_git_suffix() {
        let url = "https://github.com/org/repo.git";
        let parsed = parse_repository(url).unwrap();
        assert_eq!(parsed, "https://github.com/org/repo");
    }

    #[test]
    fn preserves_http_scheme() {
        let url = "http://gitlab.com/group/project/tree/main";
        let parsed = parse_repository(url).unwrap();
        assert_eq!(parsed, "http://gitlab.com/group/project");
    }

    #[test]
    fn trims_whitespace_before_parsing() {
        let url = "  git@github.com:org/repo.git  ";
        let parsed = parse_repository(url).unwrap();
        assert_eq!(parsed, "git@github.com:org/repo.git");
    }

    #[test]
    fn fails_on_invalid_url() {
        let url = "ftp://github.com/org/repo";
        let err = parse_repository(url).unwrap_err().to_string();
        assert!(
            err.contains("Unsupported repository URL"),
            "unexpected error message: {}",
            err
        );
    }

    // convert_to_ssh

    #[test]
    fn return_ssh() {
        let url = "git@github.com:org/repo.git";
        let ssh = convert_to_ssh(url).unwrap();
        assert_eq!(ssh, "git@github.com:org/repo.git");
    }

    #[test]
    fn converts_https_to_ssh() {
        let url = "https://github.com/org/repo";
        let ssh = convert_to_ssh(url).unwrap();
        assert_eq!(ssh, "git@github.com:org/repo.git");
    }

    #[test]
    fn converts_http_to_ssh() {
        let url = "http://gitlab.com/group/project";
        let ssh = convert_to_ssh(url).unwrap();
        assert_eq!(ssh, "git@gitlab.com:group/project.git");
    }

    #[test]
    fn strips_trailing_git_suffix() {
        let url = "https://bitbucket.org/team/repo.git";
        let ssh = convert_to_ssh(url).unwrap();
        assert_eq!(ssh, "git@bitbucket.org:team/repo.git");
    }

    #[test]
    fn ignores_extra_path_parts() {
        let url = "https://github.com/org/repo/tree/main";
        let ssh = convert_to_ssh(url).unwrap();
        assert_eq!(ssh, "git@github.com:org/repo.git");
    }

    #[test]
    fn trims_input() {
        let url = "  https://github.com/org/repo  ";
        let ssh = convert_to_ssh(url).unwrap();
        assert_eq!(ssh, "git@github.com:org/repo.git");
    }

    #[test]
    fn parses_repository_parts_from_tree_url() {
        let url = "https://github.com/org/repo/tree/main";
        let parts = parse_repository_parts(url).unwrap();
        assert_eq!(parts.host, "github.com");
        assert_eq!(parts.org, "org");
        assert_eq!(parts.name, "repo");
        assert_eq!(parts.transport, RepositoryTransport::Https);
    }
}
