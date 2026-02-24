const EXCLUDED_DIRS = new Set([
  'node_modules', '.git', 'vendor', 'dist', 'build', 'coverage',
  '.nyc_output', '__pycache__', '.pytest_cache', 'venv', '.venv',
  'env', '.next', '.nuxt', 'target', 'out', '.cache', '.turbo',
  '.parcel-cache', 'public', 'static',
]);

const EXCLUDED_FILENAMES = new Set([
  // Dependency manifests & lock files — change for non-code reasons
  'package.json', 'package-lock.json', 'yarn.lock', 'pnpm-lock.yaml',
  'composer.lock', 'Gemfile.lock', 'Pipfile.lock', 'poetry.lock',
  'go.sum', 'Cargo.lock', 'packages.lock.json', 'npm-shrinkwrap.json',
  // Config noise & OS files
  '.gitignore', '.gitattributes', '.editorconfig', '.eslintrc', '.prettierrc',
  '.browserslistrc', 'CHANGELOG.md', 'CHANGELOG', '.DS_Store', 'Thumbs.db',
]);

const EXCLUDED_EXTENSIONS = new Set([
  '.lock', '.map', '.snap',
  // Images & fonts
  '.png', '.jpg', '.jpeg', '.gif', '.svg', '.ico', '.webp', '.avif',
  '.woff', '.woff2', '.ttf', '.eot', '.otf',
  // Media
  '.mp4', '.mp3', '.wav', '.ogg', '.webm',
  // Archives & binaries
  '.pdf', '.zip', '.tar', '.gz', '.bz2', '.xz', '.7z', '.dmg', '.exe',
  // Minified assets
  '.min.js', '.min.css',
]);

/**
 * Filters a list of file paths, removing noise that isn't useful for
 * hotspot analysis (deps, lock files, generated assets, binaries, etc.).
 *
 * Security-sensitive files (.env, keys) are NOT filtered here — they're
 * surfaced separately by the security analyzer before this step runs.
 */
export function filterFiles(files: string[], pathFilter: string | null = null): string[] {
  return files.filter(file => {
    if (!file) return false;

    // Apply --path scoping
    if (pathFilter) {
      const normalized = pathFilter.endsWith('/') ? pathFilter : pathFilter + '/';
      if (!file.startsWith(normalized) && file !== pathFilter) return false;
    }

    const segments = file.split('/');
    const filename = segments[segments.length - 1] ?? '';

    // Exclude any path segment that is a known noise directory
    if (segments.some(seg => EXCLUDED_DIRS.has(seg))) return false;

    // Exclude known noise filenames
    if (EXCLUDED_FILENAMES.has(filename)) return false;

    // Exclude by extension (handle compound extensions like .min.js)
    const lower = filename.toLowerCase();
    for (const ext of EXCLUDED_EXTENSIONS) {
      if (lower.endsWith(ext)) return false;
    }

    return true;
  });
}
