#!/usr/bin/env node
// resolve-clippy-rev.js
//
// Resolve the clippy_utils git revision that matches a target nightly date.
//
// clippy_utils lives in rust-lang/rust-clippy. Rule (see
// docs/NIGHTLY_UPGRADE_RUNBOOK.md, step 2): take the most recent commit dated
// on or before the target nightly. A clippy commit from that day is the closest
// available proxy for the clippy bundled into that nightly.
//
// The `rustup` branch that used to carry one commit per nightly is retired, and
// the repository has since renamed its default branch (`master` -> `main`).
// Hardcoding *any* branch name is what silently broke the scheduled bump, so we
// ask the API for the current default branch and only fall back to the
// well-known names when that lookup is unavailable. Transient API failures are
// retried, and every failure reason is printed to stderr so the workflow log
// says why nothing resolved instead of just "could not resolve".
//
// Usage: node resolve-clippy-rev.js <YYYY-MM-DD>
// Prints the 40-char rev on stdout, or exits 1 if none could be resolved.

"use strict";

const https = require("https");

const REPO = "rust-lang/rust-clippy";
const FALLBACK_BRANCHES = ["main", "master", "rustup"];
const MAX_ATTEMPTS = 3;
const TIMEOUT_MS = 15000;

const target = process.argv[2];
if (!target || !/^\d{4}-\d{2}-\d{2}$/.test(target)) {
  process.stderr.write("usage: node resolve-clippy-rev.js <YYYY-MM-DD>\n");
  process.exit(2);
}

const token = process.env.GITHUB_TOKEN || process.env.GITHUB_TOKEN_FOR_PR || "";

function note(message) {
  process.stderr.write(`resolve-clippy-rev: ${message}\n`);
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function get(url) {
  return new Promise((resolve, reject) => {
    const headers = {
      "User-Agent": "soroban-cost-linter-nightly-bump",
      Accept: "application/vnd.github+json",
    };
    if (token) headers["Authorization"] = `Bearer ${token}`;
    const req = https.get(url, { headers }, (res) => {
      let body = "";
      res.on("data", (c) => (body += c));
      res.on("end", () => resolve({ status: res.statusCode, headers: res.headers, body }));
    });
    req.setTimeout(TIMEOUT_MS, () => req.destroy(new Error(`no response after ${TIMEOUT_MS}ms`)));
    req.on("error", reject);
  });
}

// GET + parse with a bounded retry on timeouts, 5xx and rate limiting. Returns
// the parsed payload on 200, null on a definitive 404/422 (branch or resource
// does not exist), and throws only once the retries are exhausted.
async function getJson(url, what) {
  let last = "";
  for (let attempt = 1; attempt <= MAX_ATTEMPTS; attempt++) {
    if (attempt > 1) await sleep(1000 * (attempt - 1));
    let res;
    try {
      res = await get(url);
    } catch (err) {
      last = `request failed (${err.message})`;
      note(`${what}: ${last}, attempt ${attempt}/${MAX_ATTEMPTS}`);
      continue;
    }
    if (res.status === 200) {
      try {
        return JSON.parse(res.body);
      } catch (err) {
        last = `invalid JSON (${err.message})`;
      }
    } else if (res.status === 404 || res.status === 422) {
      note(`${what}: HTTP ${res.status} (does not exist)`);
      return null;
    } else {
      last = `HTTP ${res.status}`;
      if (res.headers["x-ratelimit-remaining"] === "0") {
        const reset = Number(res.headers["x-ratelimit-reset"]);
        last += `, rate limit exhausted${reset ? ` until ${new Date(reset * 1000).toISOString()}` : ""}`;
      }
      const message = res.body && res.body.slice(0, 200);
      if (message) last += ` (${message})`;
    }
    note(`${what}: ${last}, attempt ${attempt}/${MAX_ATTEMPTS}`);
  }
  throw new Error(`${what}: ${last}`);
}

// Default branch first, then the names this repository has used or still uses.
async function candidateBranches() {
  const branches = [];
  try {
    const repo = await getJson(`https://api.github.com/repos/${REPO}`, "repository metadata");
    if (repo && typeof repo.default_branch === "string" && repo.default_branch) {
      branches.push(repo.default_branch);
    }
  } catch (err) {
    note(`${err.message}; falling back to known branch names`);
  }
  for (const branch of FALLBACK_BRANCHES) {
    if (!branches.includes(branch)) branches.push(branch);
  }
  return branches;
}

(async () => {
  const branches = await candidateBranches();
  note(`resolving ${target} against ${REPO} branch(es): ${branches.join(", ")}`);

  for (const branch of branches) {
    const url =
      `https://api.github.com/repos/${REPO}/commits?` +
      `sha=${encodeURIComponent(branch)}&until=${target}T23:59:59Z&per_page=1`;
    let json;
    try {
      json = await getJson(url, `commits on ${branch}`);
    } catch (err) {
      note(`giving up on ${branch}: ${err.message}`);
      continue;
    }
    if (!Array.isArray(json) || json.length === 0) {
      note(`${branch}: no commit dated on or before ${target}`);
      continue;
    }
    const sha = json[0] && json[0].sha;
    if (!/^[a-f0-9]{40}$/.test(sha || "")) {
      note(`${branch}: unexpected response (sha=${JSON.stringify(sha)})`);
      continue;
    }
    const date = json[0].commit && json[0].commit.committer && json[0].commit.committer.date;
    note(`${branch}: ${sha} (${date})`);
    process.stdout.write(sha);
    return;
  }

  note(`no ${REPO} commit dated on or before ${target} on any candidate branch`);
  process.exit(1);
})().catch((err) => {
  note(err && err.stack ? err.stack : String(err));
  process.exit(1);
});
