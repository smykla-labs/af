use crate::consts::*;
use crate::ides;
use anyhow::anyhow;
use log::{debug, warn};
use regex::Regex;
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct Repo<'a> {
    pub username: &'a str,
    pub host: &'a str,
    pub org: &'a str,
    pub name: &'a str,
}

impl<'a> Repo<'a> {
    pub fn parse(url: &'a str) -> anyhow::Result<Self> {
        let re = Regex::new(r"^git@([^:]+):([^/]+)/(.+?)\.git$")?;

        let captures = re
            .captures(url)
            .ok_or_else(|| anyhow!("Invalid repository URL: {}", url))?;

        Ok(Self {
            username: GIT, // Hardcoded since it's always 'git'
            host: captures.get(1).unwrap().as_str(),
            org: captures.get(2).unwrap().as_str(),
            name: captures.get(3).unwrap().as_str(),
        })
    }

    pub fn short_format(&self) -> String {
        format!("{}/{}/{}", self.host, self.org, self.name)
    }

    pub async fn get_languages(&self) -> anyhow::Result<BTreeMap<i64, String>> {
        octocrab::instance()
            .repos(self.org, self.name)
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
