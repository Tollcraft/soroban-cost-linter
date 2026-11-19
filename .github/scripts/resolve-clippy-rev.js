#!/usr/bin/env node
// resolve-clippy-rev.js
//
// Resolve the clippy_utils git revision that matches a target nightly date.
//
// clippy_utils lives in rust-lang/rust-clippy. Historically the `rustup`
// branch carried one commit per nightly. That branch has since been retired
// (see docs/NIGHTLY_UPGRADE_RUNBOOK.md), so we fall back to `main` and take
// the most recent commit dated on or before the target nightly. A clippy
// commit from that day is the closest available proxy for the clippy bundled
// into that nightly.
//
// Usage: node resolve-clippy-rev.js <YYYY-MM-DD>
// Prints the 40-char rev on stdout, or exits 1 if none could be resolved.
// Diagnostics go to stderr so the caller can report why a run failed.

"use strict";

const https = require("https");

const target = process.argv[2];
if (!target || !/^\d{4}-\d{2}-\d{2}$/.test(target)) {
  process.stderr.write("usage: node resolve-clippy-rev.js <YYYY-MM-DD>\n");
  process.exit(2);
}

const token = process.env.GITHUB_TOKEN || process.env.GITHUB_TOKEN_FOR_PR || "";

// The repo renamed `master` to `main`; `rustup` and `master` now 404.
const branches = ["main"];

const REQUEST_TIMEOUT_MS = 20_000;
const MAX_ATTEMPTS = 4;

function get(url, useToken) {
  return new Promise((resolve, reject) => {
    const headers = {
      "User-Agent": "soroban-cost-linter-nightly-bump",
      Accept: "application/vnd.github+json",
    };
    if (useToken && token) headers["Authorization"] = `Bearer ${token}`;
    const req = https.get(url, { headers }, (res) => {
      let body = "";
      res.on("data", (c) => (body += c));
      res.on("end", () => resolve({ status: res.statusCode, body, headers: res.headers }));
    });
    req.setTimeout(REQUEST_TIMEOUT_MS, () => {
      req.destroy(new Error(`timeout after ${REQUEST_TIMEOUT_MS}ms`));
    });
    req.on("error", reject);
  });
}

// The nightly-bump job passes the workflow's GITHUB_TOKEN. That token is
// scoped to this repository's installation, so a read of a public repo owned
// by another org can come back 401/403/404; the same request works
// unauthenticated. Transient runner networking shows up as an error or 5xx.
// Retry both, with a short backoff, and remember the last response so a real
// failure reports its status instead of a bare exit code.
async function fetchPage(url) {
  let last = null;
  for (let attempt = 1; attempt <= MAX_ATTEMPTS; attempt++) {
    for (const useToken of token ? [true, false] : [false]) {
      try {
        const r = await get(url, useToken);
        last = r;
        if (r.status === 200) return r;
        if (!useToken) continue;
        // Authenticated access to an external public repo can be denied by
        // the installation scope; try the same URL without the token.
        if (r.status === 401 || r.status === 403 || r.status === 404) continue;
      } catch (err) {
        last = { status: 0, error: err.message };
      }
    }
    if (attempt < MAX_ATTEMPTS) {
      await new Promise((r) => setTimeout(r, 1000 * attempt));
    }
  }
  return last;
}

(async () => {
  let last = null;
  for (const branch of branches) {
    const url = `https://api.github.com/repos/rust-lang/rust-clippy/commits?sha=${branch}&until=${target}T23:59:59Z&per_page=1`;
    const r = await fetchPage(url);
    last = r;
    if (r && r.status === 200) {
      let json;
      try {
        json = JSON.parse(r.body);
      } catch (err) {
        process.stderr.write(`resolve-clippy-rev: unparseable response for ${url}: ${err.message}\n`);
        continue;
      }
      if (Array.isArray(json) && json.length && /^[a-f0-9]{40}$/.test(json[0].sha)) {
        process.stdout.write(json[0].sha);
        return;
      }
      process.stderr.write(`resolve-clippy-rev: no commits on/before ${target} for branch ${branch}\n`);
    }
  }
  if (last && last.status) {
    const remaining = last.headers && last.headers["x-ratelimit-remaining"];
    process.stderr.write(
      `resolve-clippy-rev: GitHub API returned HTTP ${last.status}` +
        (remaining !== undefined ? ` (rate limit remaining: ${remaining})` : "") +
        "\n",
    );
  } else if (last && last.error) {
    process.stderr.write(`resolve-clippy-rev: network error: ${last.error}\n`);
  }
  process.exit(1);
})();
