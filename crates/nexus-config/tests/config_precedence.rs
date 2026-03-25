//! Config load order: `--config` > `NEXUS_CONFIG` > `./nexus.toml` (see `ConfigBundle::load`).
//! Serialized with a global lock because tests mutate `current_dir` and environment variables.

use nexus_config::{ConfigBundle, ENV_NEXUS_CONFIG};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

static SUITE_LOCK: Mutex<()> = Mutex::new(());

struct EnvVarGuard {
    key: &'static str,
    previous: Option<std::ffi::OsString>,
}

impl EnvVarGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let previous = std::env::var_os(key);
        std::env::set_var(key, value);
        Self { key, previous }
    }

    fn remove(key: &'static str) -> Self {
        let previous = std::env::var_os(key);
        std::env::remove_var(key);
        Self { key, previous }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(v) => std::env::set_var(self.key, v),
            None => std::env::remove_var(self.key),
        }
    }
}

struct CwdGuard {
    original: PathBuf,
}

impl CwdGuard {
    fn chdir(path: &Path) -> std::io::Result<Self> {
        let original = std::env::current_dir()?;
        std::env::set_current_dir(path)?;
        Ok(Self { original })
    }
}

impl Drop for CwdGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.original);
    }
}

fn canonical_or_identity(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

#[test]
fn explicit_config_path_beats_nexus_config_env() {
    let _lock = SUITE_LOCK.lock().expect("suite lock");
    let dir = tempfile::tempdir().expect("tempdir");
    let path_a = dir.path().join("a.toml");
    let path_b = dir.path().join("b.toml");
    fs::write(
        &path_a,
        r#"db_path = "/from-a/db.sqlite"
default_roots = []
"#,
    )
    .unwrap();
    fs::write(
        &path_b,
        r#"db_path = "/from-b/db.sqlite"
default_roots = []
"#,
    )
    .unwrap();

    let _env = EnvVarGuard::set(ENV_NEXUS_CONFIG, path_b.to_str().unwrap());
    let bundle = ConfigBundle::load(Some(&path_a)).expect("load explicit");
    assert_eq!(bundle.config.db_path, PathBuf::from("/from-a/db.sqlite"));
    assert_eq!(bundle.source_path.as_ref(), Some(&path_a));
}

#[test]
fn nexus_config_env_loads_when_no_explicit_path() {
    let _lock = SUITE_LOCK.lock().expect("suite lock");
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("from-env.toml");
    fs::write(
        &path,
        r#"db_path = "/env/db.sqlite"
default_roots = []
"#,
    )
    .unwrap();

    let _clear = EnvVarGuard::remove(ENV_NEXUS_CONFIG);
    let _env = EnvVarGuard::set(ENV_NEXUS_CONFIG, path.to_str().unwrap());
    let bundle = ConfigBundle::load(None).expect("load from env");
    assert_eq!(bundle.config.db_path, PathBuf::from("/env/db.sqlite"));
    assert_eq!(bundle.source_path.as_ref(), Some(&path));
}

#[test]
fn cwd_nexus_toml_used_when_no_explicit_and_no_env() {
    let _lock = SUITE_LOCK.lock().expect("suite lock");
    let _env = EnvVarGuard::remove(ENV_NEXUS_CONFIG);

    let dir = tempfile::tempdir().expect("tempdir");
    let local = dir.path().join("nexus.toml");
    fs::write(
        &local,
        r#"db_path = "/cwd/db.sqlite"
default_roots = []
"#,
    )
    .unwrap();

    let _cwd = CwdGuard::chdir(dir.path()).expect("chdir");
    let bundle = ConfigBundle::load(None).expect("load cwd file");
    assert_eq!(bundle.config.db_path, PathBuf::from("/cwd/db.sqlite"));
    let loaded = bundle.source_path.as_ref().expect("source path should be set");
    assert_eq!(
        canonical_or_identity(loaded),
        canonical_or_identity(&local),
        "source path should resolve to local nexus.toml",
    );
}
