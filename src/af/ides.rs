use crate::consts::*;
use anyhow::Result;
use console::{Term, measure_text_width, truncate_str};
use phf::ordered_map::OrderedMap;
use phf::phf_ordered_map;
use regex::Regex;
use std::collections::{BTreeMap, HashSet};
use std::env;
use std::ffi::OsStr;
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

const ELLIPSIS: &str = "…";
const COLUMN_GAP: &str = "  ";
const HOME_PLACEHOLDER: &str = "~";
const ZED_PREVIEW_APP: &str = "Zed Preview.app";
const ZED_APP: &str = "Zed.app";
const VSCODE_APP: &str = "Visual Studio Code.app";
const VSCODE_INSIDERS_APP: &str = "Visual Studio Code - Insiders.app";
const VSCODE_BIN: &str = "Electron";

const SUPPORTED_IDES: &[IdeKind] = &[
    IdeKind::Zed,
    IdeKind::ZedPreview,
    IdeKind::Clion,
    IdeKind::Goland,
    IdeKind::Rubymine,
    IdeKind::Rustrover,
    IdeKind::Webstorm,
    IdeKind::VsCode,
    IdeKind::VsCodeInsiders,
];

const VSCODE_IDES: &[IdeKind] = &[IdeKind::VsCode, IdeKind::VsCodeInsiders];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IdeKind {
    Zed,
    ZedPreview,
    Clion,
    Goland,
    Rubymine,
    Rustrover,
    Webstorm,
    VsCode,
    VsCodeInsiders,
}

impl IdeKind {
    fn command(self) -> &'static str {
        match self {
            IdeKind::Zed | IdeKind::ZedPreview => ZED,
            IdeKind::Clion => CLION,
            IdeKind::Goland => GOLAND,
            IdeKind::Rubymine => RUBYMINE,
            IdeKind::Rustrover => RUSTROVER,
            IdeKind::Webstorm => WEBSTORM,
            IdeKind::VsCode => CODE,
            IdeKind::VsCodeInsiders => CODE_INSIDERS,
        }
    }

    fn display_name(self) -> &'static str {
        match self {
            IdeKind::Zed => "Zed",
            IdeKind::ZedPreview => "Zed Preview",
            IdeKind::Clion => "CLion",
            IdeKind::Goland => "GoLand",
            IdeKind::Rubymine => "RubyMine",
            IdeKind::Rustrover => "RustRover",
            IdeKind::Webstorm => "WebStorm",
            IdeKind::VsCode => "VS Code",
            IdeKind::VsCodeInsiders => "VS Code Insiders",
        }
    }

    fn app_name(self) -> Option<&'static str> {
        match self {
            IdeKind::Zed => Some(ZED_APP),
            IdeKind::ZedPreview => Some(ZED_PREVIEW_APP),
            IdeKind::VsCode => Some(VSCODE_APP),
            IdeKind::VsCodeInsiders => Some(VSCODE_INSIDERS_APP),
            _ => None,
        }
    }

    fn app_executable_name(self) -> &'static str {
        match self {
            IdeKind::Zed | IdeKind::ZedPreview => "cli",
            IdeKind::VsCode | IdeKind::VsCodeInsiders => VSCODE_BIN,
            _ => self.command(),
        }
    }

    fn family_rank(self) -> usize {
        match self {
            IdeKind::Zed | IdeKind::ZedPreview => 0,
            IdeKind::Clion => 1,
            IdeKind::Goland => 2,
            IdeKind::Rubymine => 3,
            IdeKind::Rustrover => 4,
            IdeKind::Webstorm => 5,
            IdeKind::VsCode | IdeKind::VsCodeInsiders => 6,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Launcher {
    kind: IdeKind,
    launcher_path: PathBuf,
    app_path: Option<PathBuf>,
    version: Option<String>,
    label: String,
}

impl Launcher {
    fn new(
        kind: IdeKind,
        launcher_path: PathBuf,
        app_path: Option<PathBuf>,
        version: Option<String>,
    ) -> Self {
        let label = format_label(
            kind,
            version.as_deref(),
            app_path.as_deref(),
            &launcher_path,
        );

        Self {
            kind,
            launcher_path,
            app_path,
            version,
            label,
        }
    }

    pub fn kind(&self) -> IdeKind {
        self.kind
    }

    pub fn command(&self) -> &'static str {
        self.kind.command()
    }

    pub fn launcher_path(&self) -> &Path {
        &self.launcher_path
    }

    pub fn path(&self) -> &Path {
        self.launcher_path()
    }

    pub fn label(&self) -> &str {
        &self.label
    }
}

#[derive(Debug)]
struct Candidate {
    kind: IdeKind,
    launcher_path: PathBuf,
    app_path: Option<PathBuf>,
    path_rank: usize,
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
                first_index_by_kinds(launchers, VSCODE_IDES)
            } else {
                None
            }
        })
        .or_else(|| first_index_by_kind_family(launchers, IdeKind::Zed))
        .or_else(|| repo_ide.and_then(|ide| first_index_by_command(launchers, ide)))
        .or(Some(0))
}

fn select_for_dot_with(
    active_jetbrains_ide: Option<&'static str>,
    launchers: &[Launcher],
) -> Option<usize> {
    active_jetbrains_ide
        .and_then(|ide| first_index_by_command(launchers, ide))
        .or_else(|| first_index_by_kind_family(launchers, IdeKind::Zed))
        .or_else(|| first_index_by_command(launchers, GOLAND))
        .or_else(|| first_index_by_kinds(launchers, VSCODE_IDES))
}

fn discover_from_path_var(path_var: Option<std::ffi::OsString>) -> Vec<Launcher> {
    let path_dirs = path_var
        .as_ref()
        .map(env::split_paths)
        .map(Iterator::collect::<Vec<_>>)
        .unwrap_or_default();

    let mut candidates = discover_path_candidates(&path_dirs);
    candidates.extend(discover_known_app_candidates());

    let mut by_identity = BTreeMap::new();

    for candidate in candidates {
        let identity = candidate_identity(&candidate);
        let replace = by_identity
            .get(&identity)
            .map(|current: &Candidate| candidate.path_rank < current.path_rank)
            .unwrap_or(true);

        if replace {
            by_identity.insert(identity, candidate);
        }
    }

    let mut launchers = by_identity
        .into_values()
        .map(candidate_to_launcher)
        .collect::<Vec<_>>();

    launchers.sort_by_key(|launcher| {
        (
            launcher.kind.family_rank(),
            launcher.kind.display_name().to_string(),
            home_relative(
                launcher
                    .app_path
                    .as_deref()
                    .unwrap_or(launcher.launcher_path()),
            ),
            home_relative(launcher.launcher_path()),
        )
    });

    launchers
}

fn discover_path_candidates(path_dirs: &[PathBuf]) -> Vec<Candidate> {
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();

    for kind in SUPPORTED_IDES {
        for (index, base) in path_dirs.iter().enumerate() {
            let launcher_path = base.join(kind.command());
            if !is_launcher(&launcher_path) || !seen.insert(launcher_path.clone()) {
                continue;
            }

            let resolved = fs::canonicalize(&launcher_path).ok();
            let app_path = resolved
                .as_deref()
                .and_then(find_app_path_from_executable)
                .or_else(|| launcher_path_to_app_path(*kind, &launcher_path));

            let detected_kind = app_path
                .as_deref()
                .and_then(kind_from_app_path)
                .unwrap_or(*kind);

            candidates.push(Candidate {
                kind: detected_kind,
                launcher_path,
                app_path,
                path_rank: index,
            });
        }
    }

    candidates
}

fn discover_known_app_candidates() -> Vec<Candidate> {
    let mut candidates = Vec::new();

    for kind in SUPPORTED_IDES {
        for executable in known_app_executables(*kind) {
            if !is_launcher(&executable) {
                continue;
            }

            candidates.push(Candidate {
                kind: *kind,
                app_path: find_app_path_from_executable(&executable),
                launcher_path: executable,
                path_rank: usize::MAX,
            });
        }
    }

    candidates
}

fn known_app_executables(kind: IdeKind) -> Vec<PathBuf> {
    let Some(app_name) = kind.app_name() else {
        return Vec::new();
    };

    [Path::new("/Applications").join(app_name)]
        .into_iter()
        .flat_map(|app_path| {
            let executable = app_path
                .join("Contents")
                .join("MacOS")
                .join(kind.app_executable_name());
            executable.exists().then_some(executable)
        })
        .collect()
}

fn candidate_identity(candidate: &Candidate) -> String {
    if let Some(app_path) = &candidate.app_path {
        return format!("{}:{}", candidate.kind.display_name(), app_path.display());
    }

    format!(
        "{}:{}",
        candidate.kind.display_name(),
        candidate.launcher_path.display()
    )
}

fn candidate_to_launcher(candidate: Candidate) -> Launcher {
    let version = version_line(&candidate.launcher_path)
        .or_else(|| candidate.app_path.as_deref().and_then(version_line));

    Launcher::new(
        candidate.kind,
        candidate.launcher_path,
        candidate.app_path,
        version,
    )
}

fn first_index_by_command(launchers: &[Launcher], command: &str) -> Option<usize> {
    launchers
        .iter()
        .position(|launcher| launcher.command() == command)
}

fn first_index_by_kinds(launchers: &[Launcher], kinds: &[IdeKind]) -> Option<usize> {
    kinds
        .iter()
        .find_map(|kind| launchers.iter().position(|launcher| launcher.kind == *kind))
}

fn first_index_by_kind_family(launchers: &[Launcher], kind: IdeKind) -> Option<usize> {
    launchers.iter().position(|launcher| {
        matches!(
            (kind, launcher.kind),
            (IdeKind::Zed, IdeKind::Zed | IdeKind::ZedPreview)
        )
    })
}

fn supported_command(command: &str) -> Option<&'static str> {
    SUPPORTED_IDES
        .iter()
        .map(|kind| kind.command())
        .find(|candidate| *candidate == command)
}

fn format_label(
    kind: IdeKind,
    version: Option<&str>,
    app_path: Option<&Path>,
    launcher_path: &Path,
) -> String {
    let home = env::var(HOME).ok();
    format_label_with_home(kind, version, app_path, launcher_path, home.as_deref())
}

fn format_label_with_home(
    kind: IdeKind,
    version: Option<&str>,
    app_path: Option<&Path>,
    launcher_path: &Path,
    home: Option<&str>,
) -> String {
    let width = Term::stdout().size().1 as usize;
    let total_width = width.max(80).saturating_sub(8);

    let name = kind.display_name().to_string();
    let version = version
        .map(|value| extract_version(kind, value))
        .unwrap_or_default();
    let app_path = app_path
        .map(|path| home_relative_with(&path.display().to_string(), home))
        .unwrap_or_default();
    let launcher_path = home_relative_with(&launcher_path.display().to_string(), home);

    let gap_width = COLUMN_GAP.len() * 3;
    let name_width = name.chars().count().max(12);
    let version_width = version.chars().count().max(7);
    let remaining = total_width.saturating_sub(name_width + version_width + gap_width);
    let app_width = remaining * 2 / 3;
    let launcher_width = remaining.saturating_sub(app_width);

    format!(
        "{}{}{}{}{}{}{}",
        pad(&name, name_width),
        COLUMN_GAP,
        pad(&version, version_width),
        COLUMN_GAP,
        pad(&app_path, app_width.max(12)),
        COLUMN_GAP,
        pad(&launcher_path, launcher_width.max(12)),
    )
}

fn pad(value: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }

    let rendered = if measure_text_width(&value) > width {
        truncate_str(&value, width, ELLIPSIS).into_owned()
    } else {
        value.to_string()
    };

    let padding = width.saturating_sub(measure_text_width(&rendered));
    format!("{rendered}{}", " ".repeat(padding))
}

fn extract_version(kind: IdeKind, raw: &str) -> String {
    let trimmed = raw.trim();
    match kind {
        IdeKind::Zed | IdeKind::ZedPreview => trimmed
            .strip_prefix("Zed ")
            .and_then(|value| value.split(" – ").next())
            .unwrap_or(trimmed)
            .to_string(),
        IdeKind::VsCode | IdeKind::VsCodeInsiders => {
            trimmed.lines().next().unwrap_or(trimmed).trim().to_string()
        }
        _ => trimmed
            .split_whitespace()
            .last()
            .filter(|value| value.chars().any(|ch| ch.is_ascii_digit()))
            .unwrap_or(trimmed)
            .to_string(),
    }
}

fn version_line(path: &Path) -> Option<String> {
    Command::new(path)
        .arg(FLAG_VERSION)
        .output()
        .ok()
        .filter(|output| output.status.success() || !output.stdout.is_empty())
        .and_then(|output| {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            stdout
                .lines()
                .chain(stderr.lines())
                .map(str::trim)
                .find(|line| !line.is_empty())
                .map(str::to_string)
        })
}

fn find_app_path_from_executable(path: &Path) -> Option<PathBuf> {
    let mut current = path;
    while let Some(parent) = current.parent() {
        if parent.extension().and_then(OsStr::to_str) == Some("app") {
            return Some(parent.to_path_buf());
        }
        current = parent;
    }
    None
}

fn launcher_path_to_app_path(kind: IdeKind, launcher_path: &Path) -> Option<PathBuf> {
    if kind.command() != ZED {
        return None;
    }

    let contents = fs::read_to_string(launcher_path).ok()?;
    let re = Regex::new(r"'(/Applications/[^']+\.app)/Contents/MacOS/cli'").ok()?;
    re.captures(&contents)
        .and_then(|captures| captures.get(1))
        .map(|value| PathBuf::from(value.as_str()))
}

fn kind_from_app_path(path: &Path) -> Option<IdeKind> {
    match path.file_name().and_then(OsStr::to_str) {
        Some(ZED_APP) => Some(IdeKind::Zed),
        Some(ZED_PREVIEW_APP) => Some(IdeKind::ZedPreview),
        Some(VSCODE_APP) => Some(IdeKind::VsCode),
        Some(VSCODE_INSIDERS_APP) => Some(IdeKind::VsCodeInsiders),
        _ => None,
    }
}

fn home_relative(path: &Path) -> String {
    let home = env::var(HOME).ok();
    home_relative_with(&path.display().to_string(), home.as_deref())
}

fn home_relative_with(rendered: &str, home: Option<&str>) -> String {
    let Some(home) = home.filter(|value| !value.is_empty()) else {
        return rendered.to_string();
    };

    rendered.replacen(home, HOME_PLACEHOLDER, 1)
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
    fn dedupes_same_preview_app_and_keeps_path_first_launcher() -> anyhow::Result<()> {
        let dir = TempDir::new()?;
        let first = dir.path().join("zed");
        let second = dir.path().join("zed-second");
        write_launcher(
            &first,
            "#!/bin/sh\nexec '/Applications/Zed Preview.app/Contents/MacOS/cli' \"$@\"\n",
        )?;
        write_launcher(
            &second,
            "#!/bin/sh\nexec '/Applications/Zed Preview.app/Contents/MacOS/cli' \"$@\"\n",
        )?;

        let mut by_identity = BTreeMap::new();
        for candidate in [
            Candidate {
                kind: IdeKind::ZedPreview,
                launcher_path: first.clone(),
                app_path: Some(PathBuf::from("/Applications/Zed Preview.app")),
                path_rank: 0,
            },
            Candidate {
                kind: IdeKind::ZedPreview,
                launcher_path: second,
                app_path: Some(PathBuf::from("/Applications/Zed Preview.app")),
                path_rank: 1,
            },
            Candidate {
                kind: IdeKind::Zed,
                launcher_path: PathBuf::from("/Applications/Zed.app/Contents/MacOS/cli"),
                app_path: Some(PathBuf::from("/Applications/Zed.app")),
                path_rank: usize::MAX,
            },
        ] {
            if by_identity
                .get(&candidate_identity(&candidate))
                .map(|current: &Candidate| candidate.path_rank < current.path_rank)
                .unwrap_or(true)
            {
                by_identity.insert(candidate_identity(&candidate), candidate);
            }
        }

        let launchers = by_identity
            .into_values()
            .map(candidate_to_launcher)
            .collect::<Vec<_>>();

        assert_eq!(launchers.len(), 2);
        assert!(
            launchers
                .iter()
                .any(|launcher| launcher.kind == IdeKind::ZedPreview)
        );
        assert!(
            launchers
                .iter()
                .any(|launcher| launcher.kind == IdeKind::Zed)
        );

        Ok(())
    }

    #[test]
    fn detects_zed_regular_and_preview_from_apps() {
        assert_eq!(
            kind_from_app_path(Path::new("/Applications/Zed.app")),
            Some(IdeKind::Zed)
        );
        assert_eq!(
            kind_from_app_path(Path::new("/Applications/Zed Preview.app")),
            Some(IdeKind::ZedPreview)
        );
    }

    #[test]
    fn formats_vscode_name_and_home_relative_paths() {
        let label = format_label_with_home(
            IdeKind::VsCode,
            Some("1.109.5"),
            Some(Path::new("/Users/test/Applications/Visual Studio Code.app")),
            Path::new("/Users/test/.cargo/bin/code"),
            Some("/Users/test"),
        );

        assert!(label.contains("VS Code"));
        assert!(label.contains("1.109.5"));
        assert!(!label.contains('\n'));
        assert!(!label.contains("/Users/test"));
    }

    #[test]
    fn selects_active_vscode_before_zed() {
        let launchers = vec![
            launcher(IdeKind::Zed, "/tmp/bin/zed"),
            launcher(IdeKind::VsCode, "/tmp/bin/code"),
        ];

        assert_eq!(
            select_for_clone_with(None, true, &launchers, Some(RUSTROVER)),
            Some(1)
        );
    }

    #[test]
    fn selects_zed_family_before_language_match_when_not_in_editor() {
        let launchers = vec![
            launcher(IdeKind::ZedPreview, "/tmp/bin/zed"),
            launcher(IdeKind::Rustrover, "/tmp/bin/rustrover"),
        ];

        assert_eq!(
            select_for_clone_with(None, false, &launchers, Some(RUSTROVER)),
            Some(0)
        );
    }

    #[test]
    fn dot_prefers_zed_then_goland_then_vscode() {
        let launchers = vec![
            launcher(IdeKind::VsCode, "/tmp/bin/code"),
            launcher(IdeKind::Goland, "/tmp/bin/goland"),
        ];
        assert_eq!(select_for_dot_with(None, &launchers), Some(1));

        let launchers = vec![
            launcher(IdeKind::VsCode, "/tmp/bin/code"),
            launcher(IdeKind::Zed, "/tmp/bin/zed"),
        ];
        assert_eq!(select_for_dot_with(None, &launchers), Some(1));
    }

    fn launcher(kind: IdeKind, path: &str) -> Launcher {
        Launcher::new(kind, PathBuf::from(path), None, None)
    }

    fn write_launcher(path: &Path, contents: &str) -> anyhow::Result<()> {
        fs::write(path, contents)?;

        #[cfg(unix)]
        {
            let mut permissions = fs::metadata(path)?.permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(path, permissions)?;
        }

        Ok(())
    }

    #[test]
    fn home_relative_replaces_home_with_tilde() {
        assert_eq!(
            home_relative_with("/Users/test/bin/zed", Some("/Users/test")),
            "~/bin/zed".to_string()
        );
    }
}
