import assert from "node:assert/strict";
import { test } from "node:test";

import {
  allowlistExpired,
  highCriticalFindings,
  unexpectedFindings,
} from "../src/lib/audit.ts";

const report = {
  vulnerabilities: {
    next: {
      via: [
        "next@16.3.0",
        {
          severity: "critical",
          url: "https://github.com/advisories/GHSA-example",
          title: "critical issue",
        },
        {
          severity: "moderate",
          url: "https://github.com/advisories/GHSA-low",
          title: "moderate issue",
        },
      ],
    },
    sharp: {
      via: [
        {
          severity: "high",
          url: "https://github.com/advisories/GHSA-sharp",
          title: "sharp issue",
        },
      ],
    },
  },
};

test("collects unique high/critical advisories and ignores strings/moderate", () => {
  const findings = highCriticalFindings(report);
  assert.deepEqual(
    findings.map((f) => f.url),
    ["https://github.com/advisories/GHSA-example", "https://github.com/advisories/GHSA-sharp"]
  );
  assert.equal(findings[0].name, "next");
});

test("flags advisories that are not in the allowlist", () => {
  const findings = highCriticalFindings(report);
  const unexpected = unexpectedFindings(findings, {
    reviewBy: "2099-01-01",
    accepted: { "https://github.com/advisories/GHSA-sharp": "accepted" },
  });
  assert.deepEqual(
    unexpected.map((f) => f.url),
    ["https://github.com/advisories/GHSA-example"]
  );
});

test("detects expired allowlist", () => {
  const allowlist = { reviewBy: "2026-01-01", accepted: {} };
  assert.equal(allowlistExpired(allowlist, new Date("2026-06-01")), true);
  assert.equal(allowlistExpired(allowlist, new Date("2025-06-01")), false);
});
