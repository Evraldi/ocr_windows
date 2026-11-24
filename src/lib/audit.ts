export interface AuditAllowlist {
  reviewBy: string;
  accepted: Record<string, string>;
}

export interface AuditFinding {
  url: string;
  name: string;
  severity: string;
  title: string;
}

interface AuditVia {
  severity?: string;
  url?: string;
  title?: string;
}

interface AuditVulnerability {
  via?: Array<string | AuditVia>;
}

export interface AuditReport {
  vulnerabilities?: Record<string, AuditVulnerability>;
}

/** Kumpulkan advisory high/critical unik dari `npm audit --json`. */
export function highCriticalFindings(report: AuditReport): AuditFinding[] {
  const found = new Map<string, AuditFinding>();
  for (const [name, vulnerability] of Object.entries(report.vulnerabilities ?? {})) {
    for (const via of vulnerability.via ?? []) {
      if (
        typeof via === "object" &&
        ["high", "critical"].includes(via.severity ?? "") &&
        via.url
      ) {
        found.set(via.url, {
          url: via.url,
          name,
          severity: via.severity ?? "",
          title: via.title ?? "",
        });
      }
    }
  }
  return [...found.values()];
}

export function unexpectedFindings(
  findings: AuditFinding[],
  allowlist: AuditAllowlist
): AuditFinding[] {
  return findings.filter((finding) => !Object.hasOwn(allowlist.accepted, finding.url));
}

export function allowlistExpired(allowlist: AuditAllowlist, now = new Date()): boolean {
  return now.getTime() > new Date(allowlist.reviewBy).getTime();
}
