//! XDG Base Directory Specification discovery.
//!
//! Resolves the standard XDG_* environment variables with fallback to
//! the spec-defined defaults. See:
//! https://specifications.freedesktop.org/basedir-spec/latest/

use std::env;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct BaseDir {
    pub name: &'static str,
    pub env_var: &'static str,
    pub value: PathBuf,
    pub from_env: bool,
    pub exists: bool,
    pub description: &'static str,
}

impl BaseDir {
    fn resolve(
        name: &'static str,
        env_var: &'static str,
        default: PathBuf,
        description: &'static str,
    ) -> Self {
        let (value, from_env) = match env::var(env_var) {
            Ok(v) if !v.is_empty() => (PathBuf::from(v), true),
            _ => (default, false),
        };
        let exists = value.exists();
        Self {
            name,
            env_var,
            value,
            from_env,
            exists,
            description,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BaseDirs {
    pub dirs: Vec<BaseDir>,
}

impl BaseDirs {
    pub fn discover() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));

        let dirs = vec![
            BaseDir::resolve(
                "XDG_CONFIG_HOME",
                "XDG_CONFIG_HOME",
                home.join(".config"),
                "User-specific configuration files",
            ),
            BaseDir::resolve(
                "XDG_DATA_HOME",
                "XDG_DATA_HOME",
                home.join(".local/share"),
                "User-specific data files",
            ),
            BaseDir::resolve(
                "XDG_STATE_HOME",
                "XDG_STATE_HOME",
                home.join(".local/state"),
                "User-specific state (logs, history, recent files)",
            ),
            BaseDir::resolve(
                "XDG_CACHE_HOME",
                "XDG_CACHE_HOME",
                home.join(".cache"),
                "User-specific non-essential cached data",
            ),
            BaseDir::resolve(
                "XDG_RUNTIME_DIR",
                "XDG_RUNTIME_DIR",
                PathBuf::from(format!(
                    "/run/user/{}",
                    // SAFETY: getuid() is always safe; it has no failure modes
                    unsafe { libc_getuid() }
                )),
                "User-specific runtime files (sockets, pipes)",
            ),
            BaseDir::resolve(
                "XDG_CONFIG_DIRS",
                "XDG_CONFIG_DIRS",
                PathBuf::from("/etc/xdg"),
                "System-wide configuration directories (colon-separated)",
            ),
            BaseDir::resolve(
                "XDG_DATA_DIRS",
                "XDG_DATA_DIRS",
                PathBuf::from("/usr/local/share:/usr/share"),
                "System-wide data directories (colon-separated)",
            ),
        ];

        Self { dirs }
    }
}

// Minimal getuid wrapper to avoid pulling in the libc crate for one syscall.
// Returns the real user ID of the calling process.
#[allow(non_snake_case)]
unsafe fn libc_getuid() -> u32 {
    extern "C" {
        fn getuid() -> u32;
    }
    getuid()
}
