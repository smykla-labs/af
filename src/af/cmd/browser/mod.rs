mod structures;

use crate::cmd::browser::structures::Kind;
use crate::consts::{DEFAULT_HOMEBREW_PREFIX, FLAG_NEW_TAB, FLAG_URL, HOMEBREW_PREFIX};

use anyhow::bail;
use clap::Subcommand;
use std::{
    env,
    process::{Command, Stdio},
};
use tokio::task;
use url::Url;

#[derive(Debug, Subcommand)]
pub enum Browser {
    /// Group of browser subcommands (alias: b)
    #[command(visible_aliases = ["o"])]
    Open {
        /// Open each URL in a new tab (default: true)
        #[arg(long, default_value_t = true, require_equals = true)]
        new_tab: std::primitive::bool,

        /// Browser to use for opening the URL
        #[arg(
            long,
            short,
            value_enum,
            value_name = "BROWSER",
            default_value_t = Kind::Firefox,
        )]
        browser: Kind,

        /// The URL to open (must be a valid absolute URL)
        url: Url,
    },

    /// Find installed browsers (alias: f)
    #[command(visible_aliases = ["f"])]
    Find {
        /// Filter results by browser kinds (comma-separated)
        #[arg(
            long = "kind",
            short,
            value_enum,
            value_name = "BROWSER",
            value_delimiter = ',',
            default_values_t = Kind::all(),
        )]
        kinds: Vec<Kind>,

        /// Show all matching results (not just the first one per kind)
        #[arg(long, short)]
        all: bool,

        /// Display only the full paths to executables
        #[arg(long, short)]
        path: bool,
    },
}

impl Browser {
    pub async fn run(&self) -> anyhow::Result<()> {
        let dirs = vec![
            env::var(HOMEBREW_PREFIX)
                .unwrap_or(DEFAULT_HOMEBREW_PREFIX.to_string())
                .into(),
            "/Applications".into(),
        ];

        match self {
            Self::Open {
                new_tab,
                browser,
                url,
            } => {
                if !cfg!(target_os = "macos") {
                    bail!("This subcommand is not supported on non-macOS systems");
                }

                let mut args = vec![FLAG_URL, url.as_str()];

                if *new_tab {
                    args.insert(0, FLAG_NEW_TAB);
                }

                let browser_kind = *browser;
                let browser_obj = task::spawn_blocking(move || {
                    structures::find_browser(&dirs, browser_kind)
                })
                .await??;

                Command::new(browser_obj.path())
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .args(&args)
                    .spawn()?;

                Ok(())
            }
            Browser::Find { kinds, all, path } => {
                let kinds = kinds.clone();
                let all = *all;
                let path = *path;

                let browsers =
                    task::spawn_blocking(move || structures::find_browsers(&dirs, &kinds, all))
                        .await?;

                for b in browsers {
                    if path {
                        println!("{}", b.path().display());
                    } else {
                        println!("{b}");
                    }
                }

                Ok(())
            }
        }
    }
}
