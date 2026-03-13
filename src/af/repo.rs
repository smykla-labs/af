use crate::consts::*;
use crate::ides;
use crate::utils;
use anyhow::anyhow;
use log::{debug, warn};
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct Repo {
    pub username: String,
    pub host: String,
    pub org: String,
    pub name: String,
}

impl Repo {
    pub fn parse(url: &str) -> anyhow::Result<Self> {
        let repo = utils::parse_repository_parts(url)
            .map_err(|_| anyhow!("Invalid repository URL: {}", url))?;

        Ok(Self {
            username: GIT.to_string(),
            host: repo.host,
            org: repo.org,
            name: repo.name,
        })
    }

    pub fn short_format(&self) -> String {
        format!("{}/{}/{}", self.host, self.org, self.name)
    }

    pub async fn get_languages(&self) -> anyhow::Result<BTreeMap<i64, String>> {
        octocrab::instance()
            .repos(self.org.as_str(), self.name.as_str())
            .list_languages()
            .await
            .map(|languages| {
                languages
                    .into_iter()
                    .map(|(k, v)| (v, k.to_lowercase()))
                    .collect()
            })
            .map_err(Into::into)
    }

    pub async fn find_ide(&self) -> anyhow::Result<Option<&'static str>> {
        let languages = match self.get_languages().await {
            Ok(languages) => languages,
            Err(err) => {
                warn!(
                    "failed to detect repository languages for {}: {err}",
                    self.short_format()
                );
                return Ok(None);
            }
        };

        debug!("Languages: {:?}", languages);

        for (_, language) in languages.iter().rev() {
            if let Some(ide) = ides::get(language) {
                debug!("found IDE for language {ide}");
                return Ok(Some(ide));
            }
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_https_repository_name_without_git_suffix() {
        let repo = Repo::parse("https://github.com/org/repo.git").unwrap();
        assert_eq!(repo.host, "github.com");
        assert_eq!(repo.org, "org");
        assert_eq!(repo.name, "repo");
    }

    #[test]
    fn parses_browser_style_https_repository_url() {
        let repo = Repo::parse("https://github.com/org/repo/tree/main").unwrap();
        assert_eq!(repo.host, "github.com");
        assert_eq!(repo.org, "org");
        assert_eq!(repo.name, "repo");
    }
}
