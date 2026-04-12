use anyhow::Result;
use blake3::Hasher;
use gittriage_core::{CloneRecord, ManifestKind};
use ignore::WalkBuilder;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ScanMode {
    /// Only directories with `.git` are considered project roots (default).
    #[default]
    GitOnly,
    /// Directories with `.git` or common manifest files.
    ProjectRoots,
}

#[derive(Debug, Clone)]
pub struct ScanOptions {
    pub respect_gitignore: bool,
    pub include_hidden: bool,
    pub max_readme_bytes: usize,
    pub max_hash_files: usize,
    pub scan_mode: ScanMode,
    pub max_depth: Option<usize>,
    pub include_nested_git: bool,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            respect_gitignore: true,
            include_hidden: false,
            max_readme_bytes: 16 * 1024,
            max_hash_files: 64,
            scan_mode: ScanMode::default(),
            max_depth: None,
            include_nested_git: false,
        }
    }
}

/// Result of [`scan_roots`]: discovered roots plus nested git dirs skipped by default rules.
#[derive(Debug, Clone)]
pub struct ScanOutcome {
    pub clones: Vec<CloneRecord>,
    /// Nested `.git` directories skipped because they sit under another scanned git root.
    pub skipped_nested_git: Vec<PathBuf>,
}

fn load_gittriageignore(root: &Path) -> Vec<glob::Pattern> {
    let path = root.join(".gittriageignore");
    let Ok(content) = fs::read_to_string(&path) else {
        return Vec::new();
    };
    content
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| glob::Pattern::new(l).ok())
        .collect()
}

fn is_gittriageignored(path: &Path, patterns: &[glob::Pattern]) -> bool {
    if patterns.is_empty() {
        return false;
    }
    let path_str = path.display().to_string();
    patterns.iter().any(|p| {
        p.matches(&path_str)
            || path
                .file_name()
                .is_some_and(|n| p.matches(n.to_str().unwrap_or("")))
    })
}

pub fn scan_roots(roots: &[PathBuf], options: &ScanOptions) -> Result<ScanOutcome> {
    let mut repos = Vec::new();
    let mut skipped_nested_git = Vec::new();

    for root in roots {
        let ignore_patterns = load_gittriageignore(root);
        let mut found_git_roots: HashSet<PathBuf> = HashSet::new();

        let mut walker = WalkBuilder::new(root);
        walker.hidden(!options.include_hidden);
        walker.git_ignore(options.respect_gitignore);
        walker.git_global(options.respect_gitignore);
        walker.git_exclude(options.respect_gitignore);
        if let Some(depth) = options.max_depth {
            walker.max_depth(Some(depth));
        }

        for entry in walker.build() {
            let entry = match entry {
                Ok(v) => v,
                Err(_) => continue,
            };

            let path = entry.path();

            if !entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                continue;
            }

            if is_gittriageignored(path, &ignore_patterns) {
                continue;
            }

            let is_descendant_of_git_root = found_git_roots
                .iter()
                .any(|gr| path.starts_with(gr) && path != *gr);
            if is_descendant_of_git_root {
                let nested_git = path.join(".git").exists();
                if !nested_git {
                    continue;
                }
                if !options.include_nested_git {
                    skipped_nested_git.push(path.to_path_buf());
                    continue;
                }
                found_git_roots.insert(path.to_path_buf());
            }

            if !looks_like_project_root(path, options.scan_mode) {
                continue;
            }

            if path.join(".git").exists() {
                found_git_roots.insert(path.to_path_buf());
            }

            repos.push(build_clone_record(path, options)?);
        }
    }

    repos.sort_by(|a, b| a.path.cmp(&b.path));
    repos.dedup_by(|a, b| a.path == b.path);
    skipped_nested_git.sort();
    skipped_nested_git.dedup();
    Ok(ScanOutcome {
        clones: repos,
        skipped_nested_git,
    })
}

fn looks_like_project_root(path: &Path, mode: ScanMode) -> bool {
    let has_git = path.join(".git").exists();
    match mode {
        ScanMode::GitOnly => has_git,
        ScanMode::ProjectRoots => {
            has_git
                || path.join("Cargo.toml").exists()
                || path.join("package.json").exists()
                || path.join("pyproject.toml").exists()
                || path.join("requirements.txt").exists()
                || path.join("CMakeLists.txt").exists()
                || path.join("Makefile").exists()
        }
    }
}

fn build_clone_record(path: &Path, options: &ScanOptions) -> Result<CloneRecord> {
    let display_name = path
        .file_name()
        .map(|v| v.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string());

    let manifest_kind = detect_manifest(path);
    let readme_title = extract_readme_title(path, options.max_readme_bytes)?;
    let license_spdx = detect_license(path);
    let FingerprintResult {
        fingerprint,
        size_bytes,
    } = compute_fingerprint_and_size(path, options.max_hash_files);

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
        size_bytes: Some(size_bytes),
        manifest_kind,
        readme_title,
        license_spdx,
        fingerprint: Some(fingerprint),
        has_lockfile: detect_lockfile(path),
        has_ci: detect_ci(path),
        has_tests_dir: detect_tests_dir(path),
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

fn heading_regex() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"(?m)^\s*#\s+(.+?)\s*$").unwrap())
}

fn readme_title_quality_ok(s: &str) -> bool {
    let t = s.trim();
    if t.len() < 3 {
        return false;
    }
    let lower = t.to_lowercase();
    if matches!(
        lower.as_str(),
        "or" | "and" | "the" | "a" | "an" | "to" | "of" | "in" | "for" | "is" | "it" | "if"
    ) {
        return false;
    }
    if t.chars()
        .all(|c| c.is_ascii_punctuation() || c.is_whitespace())
    {
        return false;
    }
    true
}

fn extract_readme_title(path: &Path, max_bytes: usize) -> Result<Option<String>> {
    let candidates = ["README.md", "README", "readme.md"];
    let re = heading_regex();

    for file in candidates {
        let readme = path.join(file);
        if !readme.exists() {
            continue;
        }

        let mut content = fs::read_to_string(&readme)?;
        if content.len() > max_bytes {
            content.truncate(max_bytes);
        }

        if let Some(caps) = re.captures(&content) {
            let t = caps[1].trim().to_string();
            if readme_title_quality_ok(&t) {
                return Ok(Some(t));
            }
        }
    }

    Ok(None)
}

fn detect_license(path: &Path) -> Option<String> {
    let candidates = [
        "LICENSE",
        "LICENSE.md",
        "LICENSE.txt",
        "LICENCE",
        "LICENCE.md",
    ];
    for name in candidates {
        let file = path.join(name);
        if !file.exists() {
            continue;
        }
        if let Ok(content) = std::fs::read_to_string(&file) {
            let snippet = if content.len() > 2048 {
                &content[..2048]
            } else {
                &content
            };
            let upper = snippet.to_uppercase();
            if let Some(spdx) = extract_spdx_header(&upper) {
                return Some(spdx);
            }
            return Some(sniff_license_body(&upper));
        }
        return Some("UNKNOWN".into());
    }
    None
}

fn extract_spdx_header(upper: &str) -> Option<String> {
    for line in upper.lines().take(5) {
        if line.contains("SPDX-LICENSE-IDENTIFIER") {
            if let Some(id) = line.split(':').nth(1) {
                let id = id.trim();
                if !id.is_empty() {
                    return Some(id.to_string());
                }
            }
        }
    }
    None
}

fn sniff_license_body(upper: &str) -> String {
    if upper.contains("MIT LICENSE")
        || upper.contains("PERMISSION IS HEREBY GRANTED, FREE OF CHARGE")
    {
        "MIT".into()
    } else if upper.contains("APACHE LICENSE") && upper.contains("VERSION 2.0") {
        "Apache-2.0".into()
    } else if upper.contains("GNU GENERAL PUBLIC LICENSE") {
        if upper.contains("VERSION 3") {
            "GPL-3.0".into()
        } else if upper.contains("VERSION 2") {
            "GPL-2.0".into()
        } else {
            "GPL".into()
        }
    } else if upper.contains("GNU LESSER GENERAL PUBLIC LICENSE") {
        "LGPL".into()
    } else if upper.contains("BSD 2-CLAUSE")
        || (upper.contains("REDISTRIBUTION AND USE") && !upper.contains("NEITHER THE NAME"))
    {
        "BSD-2-Clause".into()
    } else if upper.contains("BSD 3-CLAUSE") || upper.contains("NEITHER THE NAME") {
        "BSD-3-Clause".into()
    } else if upper.contains("ISC LICENSE") || upper.contains("PERMISSION TO USE, COPY, MODIFY") {
        "ISC".into()
    } else if upper.contains("MOZILLA PUBLIC LICENSE") && upper.contains("2.0") {
        "MPL-2.0".into()
    } else if upper.contains("THE UNLICENSE") || upper.contains("UNLICENSE") {
        "Unlicense".into()
    } else if upper.contains("CREATIVE COMMONS") {
        "CC".into()
    } else if upper.contains("DO WHAT THE FUCK YOU WANT") || upper.contains("WTFPL") {
        "WTFPL".into()
    } else {
        "UNKNOWN".into()
    }
}

fn detect_lockfile(path: &Path) -> bool {
    [
        "Cargo.lock",
        "package-lock.json",
        "yarn.lock",
        "pnpm-lock.yaml",
        "poetry.lock",
        "Pipfile.lock",
        "bun.lockb",
        "go.sum",
        "Gemfile.lock",
    ]
    .iter()
    .any(|f| path.join(f).exists())
}

fn detect_ci(path: &Path) -> bool {
    path.join(".github/workflows").is_dir()
        || path.join(".gitlab-ci.yml").exists()
        || path.join(".circleci").is_dir()
        || path.join("Jenkinsfile").exists()
        || path.join(".travis.yml").exists()
}

fn detect_tests_dir(path: &Path) -> bool {
    ["tests", "test", "spec", "__tests__", "test_suite"]
        .iter()
        .any(|d| path.join(d).is_dir())
}

struct FingerprintResult {
    fingerprint: String,
    size_bytes: u64,
}

/// Single walk: collect fingerprint and total size simultaneously.
fn compute_fingerprint_and_size(path: &Path, max_files: usize) -> FingerprintResult {
    let mut files = Vec::with_capacity(max_files);
    let mut total_size: u64 = 0;

    for entry in walkdir::WalkDir::new(path) {
        let entry = match entry {
            Ok(v) => v,
            Err(_) => continue,
        };

        if entry.file_type().is_file() {
            if let Ok(meta) = entry.metadata() {
                total_size += meta.len();
            }
            if files.len() < max_files {
                let rel = entry
                    .path()
                    .strip_prefix(path)
                    .unwrap_or(entry.path())
                    .display()
                    .to_string();
                files.push(rel);
            }
        }
    }

    files.sort();
    let mut hasher = Hasher::new();
    for f in files {
        hasher.update(f.as_bytes());
    }

    FingerprintResult {
        fingerprint: hasher.finalize().to_hex().to_string(),
        size_bytes: total_size,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_dir(base: &Path, rel: &str) -> PathBuf {
        let p = base.join(rel);
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn touch(base: &Path, file: &str) {
        let p = base.join(file);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&p, "").unwrap();
    }

    #[test]
    fn git_only_mode_skips_manifest_only_dirs() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        let git_repo = make_dir(root, "real-repo");
        fs::create_dir_all(git_repo.join(".git")).unwrap();
        touch(&git_repo, "Cargo.toml");

        let manifest_only = make_dir(root, "just-cargo");
        touch(&manifest_only, "Cargo.toml");

        let opts = ScanOptions {
            scan_mode: ScanMode::GitOnly,
            ..Default::default()
        };

        let results = scan_roots(&[root.to_path_buf()], &opts).unwrap().clones;
        let paths: Vec<&str> = results.iter().map(|r| r.path.as_str()).collect();

        assert!(
            paths.iter().any(|p| p.contains("real-repo")),
            "git_only should include dirs with .git"
        );
        assert!(
            !paths.iter().any(|p| p.contains("just-cargo")),
            "git_only should skip dirs with only Cargo.toml"
        );
    }

    #[test]
    fn project_roots_mode_includes_manifest_dirs() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        let git_repo = make_dir(root, "real-repo");
        fs::create_dir_all(git_repo.join(".git")).unwrap();

        let manifest_only = make_dir(root, "just-cargo");
        touch(&manifest_only, "Cargo.toml");

        let opts = ScanOptions {
            scan_mode: ScanMode::ProjectRoots,
            ..Default::default()
        };

        let results = scan_roots(&[root.to_path_buf()], &opts).unwrap().clones;
        let paths: Vec<&str> = results.iter().map(|r| r.path.as_str()).collect();

        assert!(
            paths.iter().any(|p| p.contains("real-repo")),
            "project_roots should include dirs with .git"
        );
        assert!(
            paths.iter().any(|p| p.contains("just-cargo")),
            "project_roots should include dirs with Cargo.toml"
        );
    }

    #[test]
    fn gittriageignore_excludes_matching_paths() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        let keep = make_dir(root, "keep-me");
        fs::create_dir_all(keep.join(".git")).unwrap();

        let skip = make_dir(root, "skip-me");
        fs::create_dir_all(skip.join(".git")).unwrap();

        fs::write(root.join(".gittriageignore"), "skip-me\n").unwrap();

        let opts = ScanOptions {
            scan_mode: ScanMode::GitOnly,
            ..Default::default()
        };

        let results = scan_roots(&[root.to_path_buf()], &opts).unwrap().clones;
        let paths: Vec<&str> = results.iter().map(|r| r.path.as_str()).collect();

        assert!(
            paths.iter().any(|p| p.contains("keep-me")),
            "should keep non-ignored repos"
        );
        assert!(
            !paths.iter().any(|p| p.contains("skip-me")),
            ".gittriageignore should exclude matching dirs"
        );
    }

    #[test]
    fn does_not_descend_into_git_root_subdirs() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        let monorepo = make_dir(root, "monorepo");
        fs::create_dir_all(monorepo.join(".git")).unwrap();
        touch(&monorepo, "Cargo.toml");

        let sub = make_dir(root, "monorepo/packages/sub-pkg");
        fs::create_dir_all(sub.join(".git")).unwrap();
        touch(&sub, "package.json");

        let opts = ScanOptions {
            scan_mode: ScanMode::ProjectRoots,
            ..Default::default()
        };

        let outcome = scan_roots(&[root.to_path_buf()], &opts).unwrap();
        let paths: Vec<&str> = outcome.clones.iter().map(|r| r.path.as_str()).collect();

        assert!(
            paths
                .iter()
                .any(|p| p.contains("monorepo") && !p.contains("sub-pkg")),
            "should find the top-level monorepo"
        );
        assert!(
            !paths.iter().any(|p| p.contains("sub-pkg")),
            "should NOT descend into git root subdirectories"
        );
        assert!(
            outcome
                .skipped_nested_git
                .iter()
                .any(|p| p.to_string_lossy().contains("sub-pkg")),
            "nested git roots should be listed as skipped"
        );
    }

    #[test]
    fn max_depth_limits_traversal() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        let shallow = make_dir(root, "level1");
        fs::create_dir_all(shallow.join(".git")).unwrap();

        let deep = make_dir(root, "a/b/c/deep-repo");
        fs::create_dir_all(deep.join(".git")).unwrap();

        let opts = ScanOptions {
            scan_mode: ScanMode::GitOnly,
            max_depth: Some(2),
            ..Default::default()
        };

        let results = scan_roots(&[root.to_path_buf()], &opts).unwrap().clones;
        let paths: Vec<&str> = results.iter().map(|r| r.path.as_str()).collect();

        assert!(
            paths.iter().any(|p| p.contains("level1")),
            "shallow repos within depth should be found"
        );
        assert!(
            !paths.iter().any(|p| p.contains("deep-repo")),
            "repos beyond max_depth should not be found"
        );
    }

    // ── SPDX license sniffing ────────────────────────────────────────────────

    #[test]
    fn spdx_header_extraction() {
        let text = "SPDX-LICENSE-IDENTIFIER: MIT\nSOME OTHER TEXT";
        assert_eq!(extract_spdx_header(text), Some("MIT".into()));
    }

    #[test]
    fn sniff_mit_license() {
        let body = "MIT LICENSE\n\nPermission is hereby granted, free of charge...";
        assert_eq!(sniff_license_body(&body.to_uppercase()), "MIT");
    }

    #[test]
    fn sniff_apache2_license() {
        let body = "Apache License\nVersion 2.0, January 2004";
        assert_eq!(sniff_license_body(&body.to_uppercase()), "Apache-2.0");
    }

    #[test]
    fn sniff_gpl3_license() {
        let body = "GNU GENERAL PUBLIC LICENSE\nVersion 3, 29 June 2007";
        assert_eq!(sniff_license_body(&body.to_uppercase()), "GPL-3.0");
    }

    #[test]
    fn detect_license_reads_file() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        fs::write(
            root.join("LICENSE"),
            "MIT License\n\nPermission is hereby granted, free of charge...",
        )
        .unwrap();
        assert_eq!(detect_license(root), Some("MIT".into()));
    }

    #[test]
    fn detect_license_spdx_header_takes_priority() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        fs::write(
            root.join("LICENSE"),
            "SPDX-License-Identifier: Apache-2.0\n\nSome license text...",
        )
        .unwrap();
        assert_eq!(detect_license(root), Some("APACHE-2.0".into()));
    }

    #[test]
    fn detect_license_returns_none_without_file() {
        let tmp = TempDir::new().unwrap();
        assert_eq!(detect_license(tmp.path()), None);
    }

    // ── Project cue detection ────────────────────────────────────────────────

    #[test]
    fn detect_lockfile_finds_cargo_lock() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        touch(root, "Cargo.lock");
        assert!(detect_lockfile(root));
    }

    #[test]
    fn detect_lockfile_false_when_absent() {
        let tmp = TempDir::new().unwrap();
        assert!(!detect_lockfile(tmp.path()));
    }

    #[test]
    fn detect_ci_finds_github_workflows() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        fs::create_dir_all(root.join(".github/workflows")).unwrap();
        assert!(detect_ci(root));
    }

    #[test]
    fn detect_ci_finds_gitlab_ci() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        touch(root, ".gitlab-ci.yml");
        assert!(detect_ci(root));
    }

    #[test]
    fn detect_ci_false_when_absent() {
        let tmp = TempDir::new().unwrap();
        assert!(!detect_ci(tmp.path()));
    }

    #[test]
    fn detect_tests_dir_finds_tests() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        fs::create_dir_all(root.join("tests")).unwrap();
        assert!(detect_tests_dir(root));
    }

    #[test]
    fn detect_tests_dir_finds_spec() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        fs::create_dir_all(root.join("spec")).unwrap();
        assert!(detect_tests_dir(root));
    }

    #[test]
    fn detect_tests_dir_false_when_absent() {
        let tmp = TempDir::new().unwrap();
        assert!(!detect_tests_dir(tmp.path()));
    }
}
