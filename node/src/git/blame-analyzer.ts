import type { Commit, SiloData } from '../types.js';

/**
 * Analyzes author concentration per file using commit history.
 *
 * Rather than running `git blame` on every file (which is slow), we use the
 * commit graph already parsed by log-parser — each commit knows its author and
 * which files it touched.
 *
 * Returns: Map<filename, SiloData>
 */
export function analyzeAuthors(
  commits: Commit[],
  files: string[]
): Map<string, SiloData> {
  const fileSet = new Set(files);

  // filename → Map<author, commitCount>
  const fileAuthorMap = new Map<string, Map<string, number>>();

  for (const commit of commits) {
    for (const file of commit.files) {
      if (!fileSet.has(file)) continue;

      let authorMap = fileAuthorMap.get(file);
      if (!authorMap) {
        authorMap = new Map<string, number>();
        fileAuthorMap.set(file, authorMap);
      }
      authorMap.set(commit.author, (authorMap.get(commit.author) ?? 0) + 1);
    }
  }

  const result = new Map<string, SiloData>();

  for (const file of files) {
    const authorMap = fileAuthorMap.get(file);

    if (!authorMap || authorMap.size === 0) {
      result.set(file, { topAuthor: 'unknown', topAuthorPercent: 100, authorCount: 1 });
      continue;
    }

    let topAuthor = '';
    let topCount = 0;
    let totalCommits = 0;

    for (const [author, count] of authorMap) {
      totalCommits += count;
      if (count > topCount) {
        topCount = count;
        topAuthor = author;
      }
    }

    result.set(file, {
      topAuthor,
      topAuthorPercent: totalCommits > 0
        ? Math.round((topCount / totalCommits) * 100)
        : 100,
      authorCount: authorMap.size,
    });
  }

  return result;
}
