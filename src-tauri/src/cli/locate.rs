use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

/// Environment configuration used for probing binaries and provider credentials.
///
/// Allows tests to run hermetically without mutating process-wide environment variables.
#[derive(Debug, Clone)]
pub struct ProbeEnv {
    pub home: Option<PathBuf>,
    pub path: Option<OsString>,
    pub tokscale_config_dir: Option<PathBuf>,
    pub clear_tokscale_config_dir: bool,
    pub xdg_config_home: Option<PathBuf>,
    pub system_bin_dirs: Vec<PathBuf>,
    pub local_app_data: Option<PathBuf>,
    pub app_data: Option<PathBuf>,
    pub user_profile: Option<PathBuf>,
}

impl ProbeEnv {
    /// Capture environment settings from the current process.
    pub fn from_process() -> Self {
        let home = std::env::var_os("HOME")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .or_else(dirs::home_dir);

        Self {
            home,
            path: std::env::var_os("PATH"),
            tokscale_config_dir: std::env::var_os("TOKSCALE_CONFIG_DIR")
                .filter(|v| !v.is_empty())
                .map(PathBuf::from),
            clear_tokscale_config_dir: false,
            xdg_config_home: std::env::var_os("XDG_CONFIG_HOME")
                .filter(|v| !v.is_empty())
                .map(PathBuf::from),
            system_bin_dirs: vec![
                PathBuf::from("/opt/homebrew/bin"),
                PathBuf::from("/usr/local/bin"),
            ],
            local_app_data: std::env::var_os("LOCALAPPDATA")
                .filter(|v| !v.is_empty())
                .map(PathBuf::from),
            app_data: std::env::var_os("APPDATA")
                .filter(|v| !v.is_empty())
                .map(PathBuf::from),
            user_profile: std::env::var_os("USERPROFILE")
                .filter(|v| !v.is_empty())
                .map(PathBuf::from),
        }
    }

    /// Create an isolated environment rooted at a temporary home directory.
    pub fn isolated(home: PathBuf) -> Self {
        Self {
            home: Some(home),
            path: None,
            tokscale_config_dir: None,
            clear_tokscale_config_dir: true,
            xdg_config_home: None,
            system_bin_dirs: Vec::new(),
            local_app_data: None,
            app_data: None,
            user_profile: None,
        }
    }

    pub fn with_path(mut self, dirs: impl IntoIterator<Item = PathBuf>) -> Self {
        if let Ok(joined) = std::env::join_paths(dirs) {
            self.path = Some(joined);
        }
        self
    }

    #[cfg(all(test, unix))]
    pub(crate) fn with_test_path(self, dirs: impl IntoIterator<Item = PathBuf>) -> Self {
        let mut all: Vec<PathBuf> = dirs.into_iter().collect();
        all.push(PathBuf::from("/usr/bin"));
        all.push(PathBuf::from("/bin"));
        self.with_path(all)
    }

    pub fn with_system_bin_dirs(mut self, dirs: Vec<PathBuf>) -> Self {
        self.system_bin_dirs = dirs;
        self
    }

    pub fn with_tokscale_config_dir(mut self, dir: PathBuf) -> Self {
        self.tokscale_config_dir = Some(dir);
        self.clear_tokscale_config_dir = false;
        self
    }

    /// Resolve the tokscale configuration directory (`$TOKSCALE_CONFIG_DIR` or `$HOME/.config/tokscale`).
    pub fn tokscale_config_dir(&self) -> Option<PathBuf> {
        if let Some(ref custom) = self.tokscale_config_dir {
            return Some(custom.clone());
        }
        #[cfg(target_os = "linux")]
        if let Some(ref xdg) = self.xdg_config_home {
            return Some(xdg.join("tokscale"));
        }
        self.home
            .as_ref()
            .map(|h| h.join(".config").join("tokscale"))
    }
}

/// How `tokscale` should be invoked once located.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliTarget {
    Direct {
        path: PathBuf,
        resolved_via: &'static str,
    },
    Runner {
        runner_path: PathBuf,
        resolved_via: &'static str,
        fallback: &'static str,
    },
    None,
}

impl CliTarget {
    pub fn resolved_via(&self) -> &'static str {
        match self {
            Self::Direct { resolved_via, .. } => resolved_via,
            Self::Runner { resolved_via, .. } => resolved_via,
            Self::None => "none",
        }
    }

    pub fn binary_path(&self) -> Option<String> {
        match self {
            Self::Direct { path, .. } => Some(path.to_string_lossy().into_owned()),
            Self::Runner { runner_path, .. } => Some(runner_path.to_string_lossy().into_owned()),
            Self::None => None,
        }
    }

    pub fn fallback(&self) -> Option<String> {
        match self {
            Self::Runner { fallback, .. } => Some((*fallback).to_string()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedTool {
    pub path: PathBuf,
    pub via: &'static str,
}

/// Locate the `tokscale` executable (or `bunx`/`npx` fallback) using the configured discovery order.
pub fn locate_tokscale(custom_tokscale_path: Option<&str>, env: &ProbeEnv) -> CliTarget {
    if let Some(raw) = custom_tokscale_path.map(str::trim).filter(|s| !s.is_empty()) {
        if let Some(resolved) = resolve_explicit_path(Path::new(raw)) {
            return CliTarget::Direct {
                path: resolved,
                resolved_via: "custom",
            };
        }
    }

    if let Some(tool) = resolve_tool("tokscale", env) {
        return CliTarget::Direct {
            path: tool.path,
            resolved_via: tool.via,
        };
    }

    if let Some(bunx) = resolve_tool("bunx", env) {
        return CliTarget::Runner {
            runner_path: bunx.path,
            resolved_via: "bunx",
            fallback: "bunx tokscale",
        };
    }

    if let Some(npx) = resolve_tool("npx", env) {
        return CliTarget::Runner {
            runner_path: npx.path,
            resolved_via: "npx",
            fallback: "npx tokscale",
        };
    }

    CliTarget::None
}

/// Resolve an arbitrary CLI tool (`tokscale`, `bunx`, `npx`, `bun`, `npm`) across `PATH` and probe dirs.
pub(crate) fn resolve_tool(name: &str, env: &ProbeEnv) -> Option<ResolvedTool> {
    if let Some(ref path_var) = env.path {
        for dir in std::env::split_paths(path_var) {
            if let Some(candidate) = resolve_in_dir(&dir, name) {
                return Some(ResolvedTool {
                    path: candidate,
                    via: "path",
                });
            }
        }
    }

    if let Some(candidate) = probe_tool_dirs(name, env) {
        return Some(ResolvedTool {
            path: candidate,
            via: "probe",
        });
    }

    None
}

/// Collect probe bin directories that exist so child processes can also find sibling runtimes (`node`, `bun`).
pub(crate) fn existing_probe_bin_dirs(env: &ProbeEnv) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(ref home) = env.home {
        let bun_bin = home.join(".bun").join("bin");
        if bun_bin.is_dir() {
            dirs.push(bun_bin);
        }
        if let Some(mise_bin) = latest_mise_bin_dir(&home.join(".local/share/mise/installs")) {
            dirs.push(mise_bin);
        }
        if let Some(nvm_bin) = latest_nvm_bin_dir(&home.join(".nvm/versions/node")) {
            dirs.push(nvm_bin);
        }
        let volta_bin = home.join(".volta").join("bin");
        if volta_bin.is_dir() {
            dirs.push(volta_bin);
        }
        let cargo_bin = home.join(".cargo").join("bin");
        if cargo_bin.is_dir() {
            dirs.push(cargo_bin);
        }
    }
    for sys_dir in &env.system_bin_dirs {
        if sys_dir.is_dir() {
            dirs.push(sys_dir.clone());
        }
    }
    dirs
}

fn probe_tool_dirs(name: &str, env: &ProbeEnv) -> Option<PathBuf> {
    if let Some(ref home) = env.home {
        // 3. $HOME/.bun/bin/<name>
        if let Some(p) = resolve_in_dir(&home.join(".bun").join("bin"), name) {
            return Some(p);
        }

        // 4. $HOME/.local/share/mise/installs/**/bin/<name> (newest version wins by lexical sort)
        if let Some(p) = find_mise_binary(&home.join(".local/share/mise/installs"), name) {
            return Some(p);
        }

        // 5. $HOME/.nvm/versions/node/*/bin/<name> (newest version wins by lexical sort)
        if let Some(p) = find_nvm_binary(&home.join(".nvm/versions/node"), name) {
            return Some(p);
        }

        // 6. $HOME/.volta/bin/<name>
        if let Some(p) = resolve_in_dir(&home.join(".volta").join("bin"), name) {
            return Some(p);
        }

        // 7. $HOME/.cargo/bin/<name>
        if let Some(p) = resolve_in_dir(&home.join(".cargo").join("bin"), name) {
            return Some(p);
        }
    }

    // 8 & 9. /opt/homebrew/bin/<name>, /usr/local/bin/<name> (via configurable system_bin_dirs)
    for sys_dir in &env.system_bin_dirs {
        if let Some(p) = resolve_in_dir(sys_dir, name) {
            return Some(p);
        }
    }

    // 10. Windows-specific locations
    #[cfg(target_os = "windows")]
    if let Some(p) = probe_windows_dirs(name, env) {
        return Some(p);
    }

    None
}

#[cfg(target_os = "windows")]
fn probe_windows_dirs(name: &str, env: &ProbeEnv) -> Option<PathBuf> {
    if let Some(ref local_app_data) = env.local_app_data {
        let dir = local_app_data.join("tokscale");
        if let Some(p) = resolve_in_dir(&dir, name) {
            return Some(p);
        }
        if name == "tokscale" {
            if let Some(p) = resolve_in_dir(local_app_data, name) {
                return Some(p);
            }
        }
    }
    if let Some(ref app_data) = env.app_data {
        if let Some(p) = resolve_in_dir(&app_data.join("npm"), name) {
            return Some(p);
        }
    }
    if let Some(ref user_profile) = env.user_profile {
        if let Some(p) = resolve_in_dir(&user_profile.join(".bun").join("bin"), name) {
            return Some(p);
        }
    }
    None
}

fn find_mise_binary(installs_dir: &Path, name: &str) -> Option<PathBuf> {
    if !installs_dir.is_dir() {
        return None;
    }

    let mut matches: Vec<(String, PathBuf)> = Vec::new();
    collect_mise_bins(installs_dir, 0, &mut |version_name, bin_dir| {
        if let Some(candidate) = resolve_in_dir(bin_dir, name) {
            matches.push((version_name.to_string(), candidate));
        }
    });

    matches.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.cmp(&a.1)));
    matches.into_iter().next().map(|(_, p)| p)
}

fn latest_mise_bin_dir(installs_dir: &Path) -> Option<PathBuf> {
    if !installs_dir.is_dir() {
        return None;
    }
    let mut bins: Vec<(String, PathBuf)> = Vec::new();
    collect_mise_bins(installs_dir, 0, &mut |version_name, bin_dir| {
        bins.push((version_name.to_string(), bin_dir.to_path_buf()));
    });
    bins.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.cmp(&a.1)));
    bins.into_iter().next().map(|(_, p)| p)
}

fn collect_mise_bins(dir: &Path, depth: usize, on_bin: &mut impl FnMut(&str, &Path)) {
    const MAX_DEPTH: usize = 6;
    if depth > MAX_DEPTH {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        let path = entry.path();
        let is_dir = if file_type.is_symlink() {
            fs::metadata(&path).map(|m| m.is_dir()).unwrap_or(false)
        } else {
            file_type.is_dir()
        };
        if !is_dir {
            continue;
        }
        if entry.file_name() == "bin" {
            let version_name = dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            on_bin(version_name, &path);
        } else if !file_type.is_symlink() {
            collect_mise_bins(&path, depth + 1, on_bin);
        }
    }
}

fn find_nvm_binary(nvm_node_dir: &Path, name: &str) -> Option<PathBuf> {
    if !nvm_node_dir.is_dir() {
        return None;
    }
    let Ok(entries) = fs::read_dir(nvm_node_dir) else {
        return None;
    };
    let mut matches: Vec<(String, PathBuf)> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let version_name = entry.file_name().to_string_lossy().into_owned();
        if let Some(candidate) = resolve_in_dir(&path.join("bin"), name) {
            matches.push((version_name, candidate));
        }
    }
    matches.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.cmp(&a.1)));
    matches.into_iter().next().map(|(_, p)| p)
}

fn latest_nvm_bin_dir(nvm_node_dir: &Path) -> Option<PathBuf> {
    if !nvm_node_dir.is_dir() {
        return None;
    }
    let Ok(entries) = fs::read_dir(nvm_node_dir) else {
        return None;
    };
    let mut dirs: Vec<(String, PathBuf)> = Vec::new();
    for entry in entries.flatten() {
        let bin = entry.path().join("bin");
        if bin.is_dir() {
            let version_name = entry.file_name().to_string_lossy().into_owned();
            dirs.push((version_name, bin));
        }
    }
    dirs.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.cmp(&a.1)));
    dirs.into_iter().next().map(|(_, p)| p)
}

fn resolve_explicit_path(path: &Path) -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase);
        if ext.as_deref() == Some("ps1") {
            let cmd = path.with_extension("cmd");
            if is_executable_file(&cmd) {
                return Some(cmd);
            }
            let exe = path.with_extension("exe");
            if is_executable_file(&exe) {
                return Some(exe);
            }
            return None;
        }
        if is_executable_file(path) {
            return Some(path.to_path_buf());
        }
        if ext.is_none() {
            let cmd = path.with_extension("cmd");
            if is_executable_file(&cmd) {
                return Some(cmd);
            }
            let exe = path.with_extension("exe");
            if is_executable_file(&exe) {
                return Some(exe);
            }
        }
        None
    }

    #[cfg(not(target_os = "windows"))]
    {
        if is_executable_file(path) {
            Some(path.to_path_buf())
        } else {
            None
        }
    }
}

fn resolve_in_dir(dir: &Path, name: &str) -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        // Prefer .cmd over .exe, and never execute .ps1 directly.
        let cmd = dir.join(format!("{name}.cmd"));
        if is_executable_file(&cmd) {
            return Some(cmd);
        }
        let exe = dir.join(format!("{name}.exe"));
        if is_executable_file(&exe) {
            return Some(exe);
        }
        let bare = dir.join(name);
        let is_ps1 = bare
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("ps1"));
        if !is_ps1 && is_executable_file(&bare) {
            return Some(bare);
        }
        None
    }

    #[cfg(not(target_os = "windows"))]
    {
        let candidate = dir.join(name);
        if is_executable_file(&candidate) {
            Some(candidate)
        } else {
            None
        }
    }
}

pub(crate) fn is_executable_file(path: &Path) -> bool {
    let Ok(meta) = fs::metadata(path) else {
        return false;
    };
    if !meta.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        (meta.permissions().mode() & 0o111) != 0
    }

    #[cfg(not(unix))]
    {
        let is_ps1 = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("ps1"));
        !is_ps1
    }
}

#[cfg(test)]
pub(crate) fn write_test_script(path: &Path, body: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create script parent dir");
    }
    fs::write(path, body).expect("write test script");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755))
            .expect("chmod +x test script");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_exec(path: &Path) {
        write_test_script(path, "#!/bin/sh\necho 'tokscale 4.23.0'\n");
    }

    #[test]
    fn discovery_order_honors_custom_then_path_then_probes_then_fallbacks() {
        let tmp = TempDir::new().expect("tempdir");
        let home = tmp.path().join("home");
        let path_dir = tmp.path().join("path-bin");
        let sys_dir = tmp.path().join("homebrew-bin");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&path_dir).unwrap();
        fs::create_dir_all(&sys_dir).unwrap();

        let custom_bin = tmp.path().join("custom-tokscale");
        let path_bin = path_dir.join("tokscale");
        let bun_bin = home.join(".bun/bin/tokscale");
        let mise_v1 = home.join(".local/share/mise/installs/node/20.1.0/bin/tokscale");
        let mise_v2 = home.join(".local/share/mise/installs/node/22.9.0/bin/tokscale");
        let nvm_v1 = home.join(".nvm/versions/node/v20.0.0/bin/tokscale");
        let nvm_v2 = home.join(".nvm/versions/node/v22.5.0/bin/tokscale");
        let volta_bin = home.join(".volta/bin/tokscale");
        let cargo_bin = home.join(".cargo/bin/tokscale");
        let homebrew_bin = sys_dir.join("tokscale");
        let bunx_bin = home.join(".bun/bin/bunx");
        let npx_bin = path_dir.join("npx");

        make_exec(&npx_bin);
        let env_base = ProbeEnv::isolated(home.clone())
            .with_path([path_dir.clone()])
            .with_system_bin_dirs(vec![sys_dir.clone()]);

        // Only npx present -> fallback "npx"
        assert_eq!(
            locate_tokscale(None, &env_base),
            CliTarget::Runner {
                runner_path: npx_bin.clone(),
                resolved_via: "npx",
                fallback: "npx tokscale",
            }
        );

        // Add bunx -> bunx beats npx
        make_exec(&bunx_bin);
        assert_eq!(
            locate_tokscale(None, &env_base),
            CliTarget::Runner {
                runner_path: bunx_bin.clone(),
                resolved_via: "bunx",
                fallback: "bunx tokscale",
            }
        );

        // Homebrew / system probe beats runner fallback
        make_exec(&homebrew_bin);
        assert_eq!(
            locate_tokscale(None, &env_base),
            CliTarget::Direct {
                path: homebrew_bin,
                resolved_via: "probe",
            }
        );

        // Cargo beats homebrew
        make_exec(&cargo_bin);
        assert_eq!(
            locate_tokscale(None, &env_base),
            CliTarget::Direct {
                path: cargo_bin,
                resolved_via: "probe",
            }
        );

        // Volta beats cargo
        make_exec(&volta_bin);
        assert_eq!(
            locate_tokscale(None, &env_base),
            CliTarget::Direct {
                path: volta_bin,
                resolved_via: "probe",
            }
        );

        // NVM newest version beats volta
        make_exec(&nvm_v1);
        make_exec(&nvm_v2);
        assert_eq!(
            locate_tokscale(None, &env_base),
            CliTarget::Direct {
                path: nvm_v2,
                resolved_via: "probe",
            }
        );

        // Mise newest version (by lexical sort of version dir) beats nvm
        make_exec(&mise_v1);
        make_exec(&mise_v2);
        assert_eq!(
            locate_tokscale(None, &env_base),
            CliTarget::Direct {
                path: mise_v2,
                resolved_via: "probe",
            }
        );

        // ~/.bun/bin beats mise
        make_exec(&bun_bin);
        assert_eq!(
            locate_tokscale(None, &env_base),
            CliTarget::Direct {
                path: bun_bin,
                resolved_via: "probe",
            }
        );

        // PATH beats probe directories
        make_exec(&path_bin);
        assert_eq!(
            locate_tokscale(None, &env_base),
            CliTarget::Direct {
                path: path_bin,
                resolved_via: "path",
            }
        );

        // Non-existent custom path falls through to PATH
        assert_eq!(
            locate_tokscale(Some("/does/not/exist/tokscale"), &env_base),
            CliTarget::Direct {
                path: path_dir.join("tokscale"),
                resolved_via: "path",
            }
        );

        // Executable custom path wins over everything
        make_exec(&custom_bin);
        assert_eq!(
            locate_tokscale(Some(&custom_bin.to_string_lossy()), &env_base),
            CliTarget::Direct {
                path: custom_bin,
                resolved_via: "custom",
            }
        );
    }

    #[test]
    fn empty_environment_returns_none() {
        let tmp = TempDir::new().unwrap();
        let env = ProbeEnv::isolated(tmp.path().join("empty-home"));
        assert_eq!(locate_tokscale(None, &env), CliTarget::None);
    }

    #[cfg(unix)]
    #[test]
    fn non_executable_file_is_ignored() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("home");
        let bin = home.join(".bun/bin/tokscale");
        fs::create_dir_all(bin.parent().unwrap()).unwrap();
        fs::write(&bin, "#!/bin/sh\necho hi\n").unwrap();
        // No +x permission bit set.
        let env = ProbeEnv::isolated(home);
        assert_eq!(locate_tokscale(None, &env), CliTarget::None);
    }
}
