use anyhow::Result;
use blake3::Hasher;
use ignore::WalkBuilder;
use nexus_core::{CloneRecord, ManifestKind};
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ScanOptions {
    pub respect_gitignore: bool,
    pub include_hidden: bool,
    pub max_readme_bytes: usize,
    pub max_hash_files: usize,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            respect_gitignore: true,
            include_hidden: false,
            max_readme_bytes: 16 * 1024,
            max_hash_files: 64,
        }
    }
}

pub fn scan_roots(roots: &[PathBuf], options: &ScanOptions) -> Result<Vec<CloneRecord>> {
    let mut repos = Vec::new();

    for root in roots {
        let mut walker = WalkBuilder::new(root);
        walker.hidden(!options.include_hidden);
        walker.git_ignore(options.respect_gitignore);
        walker.git_global(options.respect_gitignore);
        walker.git_exclude(options.respect_gitignore);

        for entry in walker.build() {
            let entry = match entry {
                Ok(v) => v,
                Err(_) => continue,
            };

            let path = entry.path();

            if !entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                continue;
            }

            if !looks_like_project_root(path) {
                continue;
            }

            repos.push(build_clone_record(path, options)?);
        }
    }

    repos.sort_by(|a, b| a.path.cmp(&b.path));
    repos.dedup_by(|a, b| a.path == b.path);
    Ok(repos)
}

fn looks_like_project_root(path: &Path) -> bool {
    path.join(".git").exists()
        || path.join("Cargo.toml").exists()
        || path.join("package.json").exists()
        || path.join("pyproject.toml").exists()
        || path.join("requirements.txt").exists()
        || path.join("CMakeLists.txt").exists()
        || path.join("Makefile").exists()
}

fn build_clone_record(path: &Path, options: &ScanOptions) -> Result<CloneRecord> {
    let display_name = path
        .file_name()
        .map(|v| v.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string());

    let manifest_kind = detect_manifest(path);
    let readme_title = extract_readme_title(path, options.max_readme_bytes)?;
    let license_spdx = detect_license(path);
    let fingerprint = Some(compute_fingerprint(path, options.max_hash_files)?);
    let size_bytes = dir_size(path).ok();

    Ok(CloneRecord {
        id: format!("clone-{}", Uuid::new_v4()),
        path: path.display().to_string(),
        display_name,
        is_git: path.join(".git").exists(),
        head_oid: None,
        active_branch: None,
        default_branch: None,
        is_dirty: false,
        last_commit_at: None,
        size_bytes,
        manifest_kind,
        readme_title,
        license_spdx,
        fingerprint,
    })
}

fn detect_manifest(path: &Path) -> Option<ManifestKind> {
    if path.join("Cargo.toml").exists() {
        return Some(ManifestKind::Cargo);
    }
    if path.join("package.json").exists() {
        return Some(ManifestKind::PackageJson);
    }
    if path.join("pyproject.toml").exists() {
        return Some(ManifestKind::PyProject);
    }
    if path.join("requirements.txt").exists() {
        return Some(ManifestKind::RequirementsTxt);
    }
    if path.join("CMakeLists.txt").exists() {
        return Some(ManifestKind::CMake);
    }
    if path.join("Makefile").exists() {
        return Some(ManifestKind::Makefile);
    }
    None
}

fn extract_readme_title(path: &Path, max_bytes: usize) -> Result<Option<String>> {
    let candidates = ["README.md", "README", "readme.md"];
    let heading_re = Regex::new(r"(?m)^\s*#\s+(.+?)\s*$")?;

    for file in candidates {
        let readme = path.join(file);
        if !readme.exists() {
            continue;
        }

        let mut content = fs::read_to_string(readme)?;
        if content.len() > max_bytes {
            content.truncate(max_bytes);
        }

        if let Some(caps) = heading_re.captures(&content) {
            return Ok(Some(caps[1].trim().to_string()));
        }
        return Ok(Some(file.to_string()));
    }

    Ok(None)
}

fn detect_license(path: &Path) -> Option<String> {
    if path.join("LICENSE").exists() || path.join("LICENSE.md").exists() {
        return Some("UNKNOWN".into());
    }
    None
}

fn compute_fingerprint(path: &Path, max_files: usize) -> Result<String> {
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(path) {
        let entry = match entry {
            Ok(v) => v,
            Err(_) => continue,
        };

        if entry.file_type().is_file() {
            let rel = entry
                .path()
                .strip_prefix(path)
                .unwrap_or(entry.path())
                .display()
                .to_string();
            files.push(rel);
            if files.len() >= max_files {
                break;
            }
        }
    }

    files.sort();
    let mut hasher = Hasher::new();
    for f in files {
        hasher.update(f.as_bytes());
    }
    Ok(hasher.finalize().to_hex().to_string())
}

fn dir_size(path: &Path) -> Result<u64> {
    let mut total = 0_u64;
    for entry in walkdir::WalkDir::new(path) {
        let entry = match entry {
            Ok(v) => v,
            Err(_) => continue,
        };
        if entry.file_type().is_file() {
            total += entry.metadata()?.len();
        }
    }
    Ok(total)
}
