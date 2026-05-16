# Security Policy

`rclean` deletes user files. The trust model is the product, so we treat
security reports as first-class issues.

## Supported Versions

While the project is pre-1.0, only the latest `0.1.x` patch release
receives security fixes. Older `0.1.x` releases are not patched —
upgrade to the latest patch.

| Version | Supported          |
| ------- | ------------------ |
| 0.1.latest | :white_check_mark: |
| 0.1.x (older) | :x: |
| anything < 0.1 | :x: |

## Reporting a Vulnerability

**Please do not file a public GitHub issue for security vulnerabilities.**

Preferred channel: GitHub's [private vulnerability reporting](https://github.com/majiayu000/rclean/security/advisories/new).
This lets us coordinate a fix before details become public.

> **Maintainer setup note (one-time, takes ~30 seconds).** Private
> vulnerability reporting is opt-in per repository. If the link
> above returns 404 or "page not found", the feature has not been
> enabled yet. Maintainer action:
> *Repository → Settings → Code security → "Private vulnerability
> reporting" → Enable*.

### Fallback channels

If the private-reporting form is not yet available, in priority order:

1. Open a [GitHub Discussion](https://github.com/majiayu000/rclean/discussions)
   with the title prefix `[SECURITY — DO NOT REPRODUCE PUBLICLY]`.
   This is visible but signals intent. The maintainer will move it
   private on first response.
2. Reach the maintainer through the contact on the
   [@majiayu000 GitHub profile](https://github.com/majiayu000).

Filing a regular public GitHub issue is the **last** resort — only
do this if (1) and (2) have both been tried and failed, and even
then, **redact the reproduction steps** and request a private
channel in the issue body.

### What to include

A reproducible report is more valuable than a polished one. Useful
information:

- The version (`rclean --version`) and platform (Linux / macOS / Windows)
- A minimal directory layout that triggers the issue (`tree -L 2` is
  fine; no need for real data)
- The command line invoked and the observed vs expected outcome
- Whether the issue requires a `.rcleanignore` / `.rclean.toml` /
  ActionPlan file to trigger, and the contents of that file

### Response time

Best effort, pre-1.0:

- Acknowledge receipt within 7 days
- Triage and confirm or close within 14 days
- Coordinated public disclosure once a fix lands on `main`

If you don't hear back within those windows, ping the issue you filed
or reach out via the contact above.

## In Scope

Things the trust model promises and that we treat as security issues:

- **Symlink bypass** — a candidate that resolves to a symlink target
  outside the scan root being deleted. (See closed issues #2, #3, #6.)
- **Broad-scope guard bypass** — `clean` operating against `/`, `$HOME`,
  `/etc`, `/usr`, system roots, or `C:\` without `--allow-broad-root`.
- **ActionPlan tampering** — a JSON plan that promotes a blocked path
  to `safe`, or names a path outside the original scan roots, being
  executed without revalidation.
- **TOCTOU window** — the scanned candidate being swapped (e.g. to a
  symlink to a system path) between scan and delete.
- **`.rcleanignore` / `.rclean.toml` injection** — a project-local
  config file weakening built-in safety classification (e.g. user
  rules producing `blocked`-safety candidates, or marker files giving
  a candidate `safe` status when it shouldn't).
- **Root-boundary escape** — canonicalization that ends up outside the
  declared scan root being treated as in-scope.
- **Dirty-git-as-safe bypass** — a candidate inside a dirty git
  worktree being auto-selected by `clean --all` without
  `--include-caution`.
- **Permanent-delete on TOCTOU symlink** — `clean --permanent`
  following a symlink target swap.

## Out of Scope

Things that aren't security issues even though they involve data loss:

- **User-initiated deletion of files the user wanted to keep but didn't
  commit / back up.** rclean prints a plan, requires `--yes` or
  interactive confirmation for `--all`, and writes to OS Trash by
  default. If a user passes `--permanent --yes` and loses uncommitted
  work, that's a user-input issue.
- **Permission denied / disk full / read-only filesystem during clean.**
  These are operational errors, not policy failures.
- **Issues that require physical access or local root on the host.**
  rclean does not defend against an attacker who already controls the
  filesystem it runs on.
- **`cargo install` integrity.** Cargo + crates.io supply-chain trust
  is owned by upstream, not by rclean. We do run `cargo audit` weekly
  on our own dependency tree — see [`Audit` workflow](.github/workflows/audit.yml).

## Past Security Work

The threat model is shaped by issues already filed and closed against
the project:

- **#2** ActionPlan trusts safety field from JSON — blocked-bypass risk
- **#3** No deletion-time guard on interactive `--all` path (TOCTOU)
- **#6** No broad-scope guard for clean against /, $HOME, /usr, /etc
- **#7** Non-atomic ActionPlan write — SIGINT corrupts file
- **#8** Walkdir errors silently swallowed — under-reports project size

These shaped the safe / caution / blocked tiering, the revalidation
step in `clean --plan`, the broad-root guard, and the atomic plan
writer. New reports should reference these to clarify whether they
extend an existing class or are something new.
