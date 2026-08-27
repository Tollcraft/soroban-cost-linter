#!/usr/bin/env node
// resolve-clippy-rev.js
//
// Resolve the clippy_utils git revision that matches a target nightly date.
//
// clippy_utils lives in rust-lang/rust-clippy. Historically the `rustup`
// branch carried one commit per nightly. That branch has since been retired
// (see docs/NIGHTLY_UPGRADE_RUNBOOK.md), so we fall back to `master` and take
// the most recent commit dated on or before the target nightly. A clippy
// commit from that day is the closest available proxy for the clippy bundled
// into that nightly.
//
// Usage: node resolve-clippy-rev.js <YYYY-MM-DD>
// Prints the 40-char rev on stdout, or exits 1 if none could be resolved.

"use strict";

const https = require("https");

const target = process.argv[2];
if (!target || !/^\d{4}-\d{2}-\d{2}$/.test(target)) {
  process.stderr.write("usage: node resolve-clippy-rev.js <YYYY-MM-DD>\n");
  process.exit(2);
}

const token = process.env.GITHUB_TOKEN || process.env.GITHUB_TOKEN_FOR_PR || "";

// Prefer the historical branch, fall back to master.
const branches = ["rustup", "master"];

function get(url) {
  return new Promise((resolve, reject) => {
    const headers = {
      "User-Agent": "soroban-cost-linter-nightly-bump",
      Accept: "application/vnd.github+json",
    };
    if (token) headers["Authorization"] = `Bearer ${token}`;
    https
      .get(url, { headers }, (res) => {
        let body = "";
        res.on("data", (c) => (body += c));
        res.on("end", () => resolve({ status: res.statusCode, body }));
      })
      .on("error", reject);
  });
}

(async () => {
  for (const branch of branches) {
    const url = `https://api.github.com/repos/rust-lang/rust-clippy/commits?sha=${branch}&until=${target}T23:59:59Z&per_page=1`;
    try {
      const r = await get(url);
      if (r.status !== 200) continue;
      const json = JSON.parse(r.body);
      if (Array.isArray(json) && json.length && /^[a-f0-9]{40}$/.test(json[0].sha)) {
        process.stdout.write(json[0].sha);
        return;
      }
    } catch (_) {
      // try next branch
    }
  }
  process.exit(1);
})();
