# Native-tool cleanup policy

This policy defines when `rclean` may delete a directory directly and when it
must delegate cleanup to the owning tool. It applies to package-manager and
tool caches whose on-disk layout can include indexes, hard links, content
stores, daemon-owned state, or installed artifacts.

## Cleanup strategies

| Strategy | Use when | Requirements |
|---|---|---|
| Direct directory deletion | The candidate is a rebuildable file tree and the owner does not provide a safer targeted cleanup command. | The rule must match a narrow cache/build directory, pass normal deletion validation, preserve dry-run and ActionPlan review, and use trash/graveyard/permanent semantics exactly like ordinary candidates. |
| Native command cleanup | The owning tool exposes a cache-clean/prune command that understands indexes, references, locks, or cache-root configuration. | The clean path must use a bounded native command runner: array arguments only, no shell strings, no sudo, explicit timeout, kill and reap on timeout, captured stdout/stderr, and failure on spawn, timeout, or nonzero exit. Dry-run must not execute the tool. |
| Daemon or API cleanup | The storage is owned by a running service or daemon and direct deletion can corrupt metadata or violate references. | Use the daemon API or official CLI command only, with explicit scope and confirmation semantics. Do not delete internal storage paths directly. |

Native-tool operations are not restorable by moving files to the rclean
graveyard or system Trash. A rule that requires a native command must either
run in permanent mode through that command or be skipped/reported in
trash/graveyard mode with an explicit reason.

## Implementation contract

- Scan and ActionPlan output describe the candidate path and risk. Execution
  decides the cleanup mechanism from the rule id.
- `--dry-run` prints the selected candidate and exits before native command
  execution.
- Native command execution must bind the target cache root when the tool
  supports it, for example by setting a documented environment variable.
- Native command execution must use `std::process::Command` with program and
  arguments passed as separate values. No shell interpolation is allowed.
- Nonzero exit, timeout, spawn failure, and output capture failure are cleanup
  failures and must be surfaced to the user and audit log.
- `rclean` must not invoke `sudo`, stop background services, or answer
  interactive prompts as part of cleanup.

## Tool policy matrix

| Tool | Current policy | Rationale | rclean cleanup stance |
|---|---|---|---|
| Go module cache | Native command cleanup | Go owns module-cache metadata and provides `go clean -modcache`; existing rules already warn that modules redownload and offline builds may fail. | Implemented for `go.module_cache` and `go.module_download_cache`: run `go clean -modcache` with `GOMODCACHE` set to the selected module cache root. |
| npm | Native command cleanup | npm documents cache management through `npm cache`; modern npm treats its cache as self-healing and `clean` as a forced operation. | Future rule should prefer `npm cache clean --force` bound to the selected cache root. Do not delete `~/.npm` wholesale. |
| pnpm | Native command cleanup | pnpm's store is shared and content-addressed; `pnpm store prune` removes unreferenced packages without assuming every file is disposable. | Future rule should use `pnpm store prune` for the selected store. Full-store deletion is a caution operation and must not be the first implementation. |
| Yarn | Native command cleanup | Yarn Classic and modern Yarn expose `yarn cache clean`; project `.yarn/cache` can also be an intentional offline mirror. | Future rule should use `yarn cache clean` for tool-owned cache roots. Do not classify project `.yarn/cache` as a global disposable cache without repository-specific evidence. |
| Gradle | Direct deletion only for narrow cache subtrees; daemon-aware policy for global caches | Gradle performs periodic cache cleanup during/around builds and daemon lifecycle, and global user-home caches can be in use by Gradle daemons. | Project build directories may remain direct-delete candidates. Global `~/.gradle` cache cleanup should be report-only or require a future daemon-aware design; do not delete the entire user home. |
| Maven | Conservative direct deletion for narrow repository subtrees; no full repository wipe | The local repository can contain remote artifacts and locally installed artifacts that may not be redownloadable. Maven's dependency plugin supports project-scoped purging, not a universal safe global clean. | Do not auto-clean all `~/.m2/repository`. Future rules may target narrow remote-artifact cache subtrees or use project-scoped Maven purge commands. |
| pip | Native command cleanup | pip documents `pip cache` for listing, removing, and purging wheel/HTTP cache entries. | Implemented for `pip.cache`: run `pip cache purge` with `PIP_CACHE_DIR` set to the selected pip cache root. |
| uv | Native command cleanup | uv documents `uv cache clean`; uv also warns that symlink link mode can couple installed packages to cache contents. | Future rule should use `uv cache clean` and mark as caution because some link modes can make cache removal user-visible. |
| Poetry | Native command cleanup | Poetry exposes `poetry cache clear` for package repository caches. | Future rule should use `poetry cache clear --all` or repository-specific `poetry cache clear <repo> --all` with noninteractive execution. Do not infer every Poetry cache directory is disposable. |
| Homebrew | Native command cleanup | Homebrew owns Cellar/Caskroom and cache relationships and runs `brew cleanup` automatically in some install/upgrade flows. | Future rule should use `brew cleanup` with explicit options. Never direct-delete Homebrew's Cellar, Caskroom, taps, or package metadata. |
| Docker | Daemon/API cleanup | Docker stores images, layers, volumes, networks, containers, and build cache behind daemon-managed metadata. Official prune commands target unused objects through the daemon. | Implemented first as `rclean docker report`: inspect-only daemon reporting with bounded Docker CLI calls and no delete/prune commands. Future deletion support must use Docker CLI/API commands with explicit scope and filters. Never delete Docker storage directories directly. |

## References

- npm CLI `npm cache`: https://docs.npmjs.com/cli/v7/commands/npm-cache/
- pnpm `store prune`: https://pnpm.io/cli/store
- Yarn cache clean: https://yarnpkg.com/cli/cache/clean
- Yarn Classic cache: https://classic.yarnpkg.com/lang/en/docs/cli/cache/
- Gradle daemon cleanup: https://docs.gradle.org/current/userguide/gradle_daemon.html
- Maven dependency purge: https://maven.apache.org/plugins/maven-dependency-plugin/purge-local-repository-mojo.html
- pip cache command: https://pip.pypa.io/en/stable/cli/pip_cache/
- uv cache: https://docs.astral.sh/uv/concepts/cache/
- Poetry cache clear: https://python-poetry.org/docs/cli/
- Homebrew cleanup behavior: https://docs.brew.sh/FAQ
- Docker system prune: https://docs.docker.com/reference/cli/docker/system/prune/
- Docker builder prune: https://docs.docker.com/reference/cli/docker/builder/prune/
