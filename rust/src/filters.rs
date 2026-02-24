use once_cell::sync::Lazy;
use std::collections::HashSet;

static EXCLUDED_DIRS: Lazy<HashSet<&'static str>> = Lazy::new(|| HashSet::from([
    "node_modules", ".git", "vendor", "dist", "build", "coverage",
    ".nyc_output", "__pycache__", ".pytest_cache", "venv", ".venv",
    "env", ".next", ".nuxt", "target", "out", ".cache", ".turbo",
    ".parcel-cache", "public", "static",
]));

static EXCLUDED_FILENAMES: Lazy<HashSet<&'static str>> = Lazy::new(|| HashSet::from([
    // Dependency manifests & lock files — change for non-code reasons
    "package.json", "package-lock.json", "yarn.lock", "pnpm-lock.yaml",
    "composer.lock", "Gemfile.lock", "Pipfile.lock", "poetry.lock",
    "go.sum", "Cargo.lock", "packages.lock.json", "npm-shrinkwrap.json",
    // Config noise & OS files
    ".gitignore", ".gitattributes", ".editorconfig", ".eslintrc", ".prettierrc",
    ".browserslistrc", "CHANGELOG.md", "CHANGELOG", ".DS_Store", "Thumbs.db",
]));

static EXCLUDED_EXTENSIONS: Lazy<Vec<&'static str>> = Lazy::new(|| vec![
    ".lock", ".map", ".snap",
    ".png", ".jpg", ".jpeg", ".gif", ".svg", ".ico", ".webp", ".avif",
    ".woff", ".woff2", ".ttf", ".eot", ".otf",
    ".mp4", ".mp3", ".wav", ".ogg", ".webm",
    ".pdf", ".zip", ".tar", ".gz", ".bz2", ".xz", ".7z", ".dmg", ".exe",
    ".min.js", ".min.css",
]);

/// Filters out files that aren't useful for hotspot analysis.
/// Security-sensitive files (.env, keys) are NOT filtered here —
/// they're surfaced separately by the security analyzer.
pub fn filter_files(files: &[String], path_filter: Option<&str>) -> Vec<String> {
    files.iter().filter(|file| {
        let f = file.as_str();

        if let Some(pf) = path_filter {
            let normalized = if pf.ends_with('/') { pf.to_string() } else { format!("{pf}/") };
            if !f.starts_with(&normalized) && f != pf { return false; }
        }

        let segments: Vec<&str> = f.split('/').collect();
        let filename = segments.last().unwrap_or(&"");

        if segments.iter().any(|seg| EXCLUDED_DIRS.contains(*seg)) { return false; }
        if EXCLUDED_FILENAMES.contains(*filename) { return false; }

        let lower = filename.to_lowercase();
        if EXCLUDED_EXTENSIONS.iter().any(|ext| lower.ends_with(ext)) { return false; }

        true
    }).cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn to_strings(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn test_removes_package_json_and_locks() {
        let files = to_strings(&["src/app.rs", "package.json", "yarn.lock", "Cargo.lock", "package-lock.json"]);
        let filtered = filter_files(&files, None);
        assert!(!filtered.contains(&"package.json".to_string()), "package.json must be filtered");
        assert!(!filtered.contains(&"yarn.lock".to_string()), "yarn.lock must be filtered");
        assert!(!filtered.contains(&"Cargo.lock".to_string()), "Cargo.lock must be filtered");
        assert!(filtered.contains(&"src/app.rs".to_string()), "src/app.rs must be kept");
    }

    #[test]
    fn test_removes_node_modules() {
        let files = to_strings(&["src/lib.ts", "node_modules/lodash/index.js", "dist/bundle.js"]);
        let filtered = filter_files(&files, None);
        assert!(!filtered.iter().any(|f| f.contains("node_modules")), "node_modules must be filtered");
        assert!(!filtered.iter().any(|f| f.contains("dist/")), "dist/ must be filtered");
        assert!(filtered.contains(&"src/lib.ts".to_string()), "src/lib.ts must be kept");
    }

    #[test]
    fn test_respects_path_filter() {
        let files = to_strings(&["src/app.rs", "tests/foo.rs", "lib/util.rs"]);
        let filtered = filter_files(&files, Some("src"));
        assert!(filtered.contains(&"src/app.rs".to_string()), "src/ file should be kept");
        assert!(!filtered.contains(&"tests/foo.rs".to_string()), "tests/ should be excluded");
        assert!(!filtered.contains(&"lib/util.rs".to_string()), "lib/ should be excluded");
    }

    #[test]
    fn test_removes_image_and_binary_extensions() {
        let files = to_strings(&["src/main.rs", "assets/logo.png", "docs/spec.pdf", "app.min.js"]);
        let filtered = filter_files(&files, None);
        assert!(!filtered.contains(&"assets/logo.png".to_string()));
        assert!(!filtered.contains(&"docs/spec.pdf".to_string()));
        assert!(!filtered.contains(&"app.min.js".to_string()));
        assert!(filtered.contains(&"src/main.rs".to_string()));
    }
}
