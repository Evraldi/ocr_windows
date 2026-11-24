import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import {
  allowlistExpired,
  highCriticalFindings,
  unexpectedFindings,
} from "../src/lib/audit.ts";

const root = fileURLToPath(new URL("..", import.meta.url));
const allowlist = JSON.parse(readFileSync(join(root, "audit-allowlist.json"), "utf8"));

if (allowlistExpired(allowlist)) {
  console.error(
    `Allowlist audit kedaluwarsa (reviewBy ${allowlist.reviewBy}). Tinjau ulang sebelum lanjut.`
  );
  process.exit(1);
}

const npmCli = process.env.npm_execpath;
const command = npmCli ? process.execPath : "npm";
const args = npmCli
  ? [npmCli, "audit", "--omit=dev", "--json"]
  : ["audit", "--omit=dev", "--json"];
const result = spawnSync(command, args, {
  cwd: root,
  encoding: "utf8",
  shell: !npmCli && process.platform === "win32",
});

if (result.error) {
  console.error("Gagal menjalankan npm audit:", result.error.message);
  process.exit(1);
}

let report;
try {
  report = JSON.parse(result.stdout);
} catch {
  console.error(result.stderr || result.stdout || "npm audit tidak menghasilkan JSON");
  process.exit(1);
}

const findings = highCriticalFindings(report);
const unexpected = unexpectedFindings(findings, allowlist);
console.log(
  `npm audit (production): ${findings.length} advisory high/critical, ${unexpected.length} di luar allowlist.`
);

if (unexpected.length > 0) {
  for (const finding of unexpected) {
    console.error(`  [${finding.severity}] ${finding.name}: ${finding.title}\n    ${finding.url}`);
  }
  console.error(
    "Advisory baru harus diperbaiki atau ditambahkan ke audit-allowlist.json dengan alasan."
  );
  process.exit(1);
}

for (const finding of findings) {
  console.log(`  accepted [${finding.severity}] ${finding.name}: ${finding.url}`);
}
