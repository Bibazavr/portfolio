// Refreshes content/stats.json: GitHub stars, forks and language for the
// repositories in portfolio.ron's open_source list, and LeetCode solved counts
// and contest rating for the LeetCode profile in its links.
//
// No dependencies; needs Node 20 or newer. portfolio.ron is only read. If a
// fetch fails, that part keeps its previous value, and the script always
// exits 0 so a flaky API never fails the nightly run. The file has no
// timestamps, so it only changes when the numbers do.
//
//   node scripts/update-stats.mjs
//
// GITHUB_TOKEN, when set, raises GitHub's rate limit.

import { readFile, writeFile } from 'node:fs/promises';

const ROOT = new URL('../', import.meta.url);
const PORTFOLIO = new URL('content/portfolio.ron', ROOT);
const STATS = new URL('content/stats.json', ROOT);

const portfolio = await readFile(PORTFOLIO, 'utf8');
const previous = await readFile(STATS, 'utf8').then(JSON.parse, () => ({}));

const stats = {
	repos: await repoStats(openSourceRepos(portfolio), previous.repos ?? {}),
	leetcode: (await leetCodeStats(leetCodeUser(portfolio))) ?? previous.leetcode,
};
if (!stats.leetcode) delete stats.leetcode;

await writeFile(STATS, `${JSON.stringify(stats, null, '\t')}\n`);
console.log('Wrote content/stats.json');

/** `owner/name` of every repository in the open_source list. */
function openSourceRepos(ron) {
	const start = ron.indexOf('open_source:');
	const end = ron.indexOf('skills:', start);
	if (start < 0 || end < 0) return [];
	const section = ron.slice(start, end);
	return [...section.matchAll(/url:\s*"https:\/\/github\.com\/([\w.-]+\/[\w.-]+)"/g)].map((m) => m[1]);
}

/** The username in the LeetCode profile link. */
function leetCodeUser(ron) {
	return ron.match(/https:\/\/leetcode\.com\/u\/([\w-]+)/)?.[1];
}

async function repoStats(repos, before) {
	const headers = { Accept: 'application/vnd.github+json', 'User-Agent': 'me.bibazavr.fyi stats' };
	if (process.env.GITHUB_TOKEN) headers.Authorization = `Bearer ${process.env.GITHUB_TOKEN}`;
	const result = {};
	for (const repo of repos) {
		try {
			const response = await fetch(`https://api.github.com/repos/${repo}`, { headers });
			if (!response.ok) throw new Error(`HTTP ${response.status}`);
			const data = await response.json();
			result[repo] = { stars: data.stargazers_count, forks: data.forks_count, language: data.language };
		} catch (error) {
			console.warn(`GitHub ${repo}: ${error.message}; keeping the previous value`);
			if (before[repo]) result[repo] = before[repo];
		}
	}
	return result;
}

async function leetCodeStats(username) {
	if (!username) return undefined;
	const query = `query stats($username: String!) {
		matchedUser(username: $username) { submitStatsGlobal { acSubmissionNum { difficulty count } } }
		userContestRanking(username: $username) { rating }
	}`;
	try {
		const response = await fetch('https://leetcode.com/graphql', {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json',
				Referer: 'https://leetcode.com',
				'User-Agent': 'me.bibazavr.fyi stats',
			},
			body: JSON.stringify({ query, variables: { username } }),
		});
		if (!response.ok) throw new Error(`HTTP ${response.status}`);
		const { data } = await response.json();
		const counts = data?.matchedUser?.submitStatsGlobal?.acSubmissionNum;
		if (!counts) throw new Error('no solved counts in the response');
		const solved = Object.fromEntries(counts.map(({ difficulty, count }) => [difficulty.toLowerCase(), count]));
		const rating = data.userContestRanking?.rating;
		return {
			solved: { all: solved.all, easy: solved.easy, medium: solved.medium, hard: solved.hard },
			...(rating ? { contestRating: Math.round(rating) } : {}),
		};
	} catch (error) {
		console.warn(`LeetCode ${username}: ${error.message}; keeping the previous value`);
		return undefined;
	}
}
