// Totals over the real runs in data/hero-runs.json, shared by the hero and the
// value band so the two can never disagree.
import data from '@/data/hero-runs.json';

type Repo = (typeof data.repos)[number];
const total = (read: (repo: Repo) => number) => data.repos.reduce((sum, repo) => sum + read(repo), 0);

export const repos = data.repos;
export const tests = total((repo) => repo.tests);
export const findingCount = total((repo) => repo.findings);
export const seconds = Math.round(total((repo) => repo.seconds));
export const ofRule = (rule: string) => total((repo) => (repo.byRule as Record<string, number>)[rule] ?? 0);
