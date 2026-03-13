use crate::consts::*;
use anyhow::Result;
use phf::ordered_map::OrderedMap;
use phf::phf_ordered_map;
use regex::Regex;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

static LANGS_IDES_MAP: OrderedMap<&str, &str> = phf_ordered_map! {
    "c" => CLION,
    "c++" => CLION,
    "go" => GOLAND,
    "javascript" => WEBSTORM,
    "ruby" => RUBYMINE,
    "rust" => RUSTROVER,
};

const SUPPORTED_IDES: &[&str] = &[
    ZED,
    CLION,
    GOLAND,
    RUBYMINE,
    RUSTROVER,
    WEBSTORM,
    CODE,
    CODE_INSIDERS,
];

const VSCODE_IDES: &[&str] = &[CODE, CODE_INSIDERS];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Launcher {
    command: &'static str,
    path: PathBuf,
    label: String,
}

impl Launcher {
    fn new(command: &'static str, path: PathBuf) -> Self {
        let label = build_label(command, &path);

        Self {
            command,
            path,
            label,
        }
    }

    pub fn command(&self) -> &'static str {
        self.command
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn label(&self) -> &str {
        &self.label
    }
}

pub fn get(language: &str) -> Option<&'static str> {
    LANGS_IDES_MAP.get(language).copied()
}

pub fn discover() -> Vec<Launcher> {
    discover_from_path_var(env::var_os(PATH))
}

pub fn active_jetbrains() -> Result<Option<&'static str>> {
    let re = Regex::new(r"application\.com\.jetbrains\.(\w+)(?:-.+)?(?:\.\d+)*")?;
    let xpc_service_name = env::var(XPC_SERVICE_NAME).unwrap_or_default();

    Ok(re
        .captures(&xpc_service_name)
        .and_then(|c| c.get(1).map(|m| m.as_str()))
        .and_then(supported_command))
}

pub fn in_vscode_terminal() -> bool {
    env::var(TERM_PROGRAM)
        .map(|value| value.eq_ignore_ascii_case("vscode"))
        .unwrap_or(false)
}

pub fn select_for_clone(launchers: &[Launcher], repo_ide: Option<&'static str>) -> Option<usize> {
    select_for_clone_with(
        active_jetbrains().ok().flatten(),
        in_vscode_terminal(),
        launchers,
        repo_ide,
    )
}

pub fn select_for_dot(launchers: &[Launcher]) -> Option<usize> {
    select_for_dot_with(active_jetbrains().ok().flatten(), launchers)
}

fn select_for_clone_with(
    active_jetbrains_ide: Option<&'static str>,
    in_vscode_terminal: bool,
    launchers: &[Launcher],
    repo_ide: Option<&'static str>,
) -> Option<usize> {
    if launchers.is_empty() {
        return None;
    }

    active_jetbrains_ide
        .and_then(|ide| first_index_by_command(launchers, ide))
        .or_else(|| {
            if in_vscode_terminal {
                first_index_by_commands(launchers, VSCODE_IDES)
            } else {
                None
            }
        })
        .or_else(|| first_index_by_command(launchers, ZED))
        .or_else(|| repo_ide.and_then(|ide| first_index_by_command(launchers, ide)))
        .or(Some(0))
}

fn select_for_dot_with(
    active_jetbrains_ide: Option<&'static str>,
    launchers: &[Launcher],
) -> Option<usize> {
    active_jetbrains_ide
        .and_then(|ide| first_index_by_command(launchers, ide))
        .or_else(|| first_index_by_command(launchers, ZED))
        .or_else(|| first_index_by_command(launchers, GOLAND))
        .or_else(|| first_index_by_commands(launchers, VSCODE_IDES))
}

fn discover_from_path_var(path_var: Option<std::ffi::OsString>) -> Vec<Launcher> {
    let Some(path_var) = path_var else {
        return Vec::new();
    };

    let search_paths = env::split_paths(&path_var).collect::<Vec<_>>();
    let mut seen = HashSet::new();
    let mut launchers = Vec::new();

    for command in SUPPORTED_IDES {
        for base in &search_paths {
            let path = base.join(command);

            if !is_launcher(&path) || !seen.insert(path.clone()) {
                continue;
            }

            launchers.push(Launcher::new(command, path));
        }
    }

    launchers
}

fn first_index_by_command(launchers: &[Launcher], command: &str) -> Option<usize> {
    launchers
        .iter()
        .position(|launcher| launcher.command == command)
}

fn first_index_by_commands(launchers: &[Launcher], commands: &[&str]) -> Option<usize> {
    commands
        .iter()
        .find_map(|command| first_index_by_command(launchers, command))
}

fn supported_command(command: &str) -> Option<&'static str> {
    SUPPORTED_IDES
        .iter()
        .copied()
        .find(|candidate| *candidate == command)
}

fn build_label(command: &str, path: &Path) -> String {
    version_line(path)
        .map(|version| format!("{version} [{}]", path.display()))
        .unwrap_or_else(|| format!("{command} [{}]", path.display()))
}

fn version_line(path: &Path) -> Option<String> {
    Command::new(path)
        .arg(FLAG_VERSION)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            stdout
                .lines()
                .chain(stderr.lines())
                .map(str::trim)
                .find(|line| !line.is_empty())
                .map(str::to_string)
        })?
}

fn is_launcher(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };

    if !metadata.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        metadata.permissions().mode() & 0o111 != 0
    }

    #[cfg(not(unix))]
    {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn language_mapping_values() {
        assert_eq!(get(C), Some(CLION));
        assert_eq!(get(CPP), Some(CLION));
        assert_eq!(get(GO), Some(GOLAND));
        assert_eq!(get(JS), Some(WEBSTORM));
        assert_eq!(get(RUBY), Some(RUBYMINE));
        assert_eq!(get(RUST), Some(RUSTROVER));
    }

    #[test]
    fn discovers_multiple_zed_launchers_in_path_order() -> anyhow::Result<()> {
        let first = TempDir::new()?;
        let second = TempDir::new()?;
        write_launcher(first.path(), ZED, "Zed Preview 1")?;
        write_launcher(second.path(), ZED, "Zed Stable 2")?;

        let launchers = discover_from_paths(&[first.path(), second.path()]);

        assert_eq!(commands(&launchers), vec![ZED, ZED]);
        assert_eq!(launchers[0].path(), first.path().join(ZED));
        assert_eq!(launchers[1].path(), second.path().join(ZED));

        Ok(())
    }

    #[test]
    fn keeps_duplicate_launchers_with_same_version_when_paths_differ() -> anyhow::Result<()> {
        let first = TempDir::new()?;
        let second = TempDir::new()?;
        write_launcher(first.path(), ZED, "Zed 0.228.0")?;
        write_launcher(second.path(), ZED, "Zed 0.228.0")?;

        let launchers = discover_from_paths(&[first.path(), second.path()]);

        assert_eq!(launchers.len(), 2);
        assert_ne!(launchers[0].path(), launchers[1].path());

        Ok(())
    }

    #[test]
    fn selects_active_vscode_before_zed() {
        let launchers = vec![
            launcher(ZED, "/tmp/bin/zed"),
            launcher(CODE, "/tmp/bin/code"),
            launcher(CODE_INSIDERS, "/tmp/bin/code-insiders"),
        ];

        assert_eq!(
            select_for_clone_with(None, true, &launchers, Some(RUSTROVER)),
            Some(1)
        );
    }

    #[test]
    fn selects_zed_before_language_match_when_not_in_editor() {
        let launchers = vec![
            launcher(ZED, "/tmp/bin/zed"),
            launcher(RUSTROVER, "/tmp/bin/rustrover"),
        ];

        assert_eq!(
            select_for_clone_with(None, false, &launchers, Some(RUSTROVER)),
            Some(0)
        );
    }

    #[test]
    fn dot_prefers_zed_then_goland_then_vscode() {
        let launchers = vec![
            launcher(CODE, "/tmp/bin/code"),
            launcher(GOLAND, "/tmp/bin/goland"),
        ];
        assert_eq!(select_for_dot_with(None, &launchers), Some(1));

        let launchers = vec![
            launcher(CODE, "/tmp/bin/code"),
            launcher(ZED, "/tmp/bin/zed"),
        ];
        assert_eq!(select_for_dot_with(None, &launchers), Some(1));
    }

    fn discover_from_paths(paths: &[&Path]) -> Vec<Launcher> {
        let joined = env::join_paths(paths).unwrap();
        discover_from_path_var(Some(joined))
    }

    fn commands(launchers: &[Launcher]) -> Vec<&'static str> {
        launchers.iter().map(Launcher::command).collect()
    }

    fn launcher(command: &'static str, path: &str) -> Launcher {
        Launcher {
            command,
            path: PathBuf::from(path),
            label: command.to_string(),
        }
    }

    fn write_launcher(dir: &Path, name: &str, version: &str) -> anyhow::Result<()> {
        let path = dir.join(name);
        fs::write(&path, format!("#!/bin/sh\necho '{version}'\n"))?;

        #[cfg(unix)]
        {
            let mut permissions = fs::metadata(&path)?.permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&path, permissions)?;
        }

        Ok(())
    }
}
