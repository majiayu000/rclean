# Free ActionPlan Default Location - Product Spec

GitHub issue: `#349`
Locale: `en-US`
Route: `implement`

## Summary

Stop `rclean free <target>` from writing its ActionPlan JSON into the
current working directory. Resolve the default plan path under the
user's state directory instead, keeping `--write-plan` as the explicit
override.

## Problem

`free` is read-only by design: it proposes a set and never deletes.
Today it still leaves a file behind in whatever directory the user was
standing in.

```
$ cd /some/git/repo
$ rclean free 20gb --home
wrote action plan: rclean-free-20260723T121738Z.json
$ ls
rclean-free-20260723T121738Z.json     # now untracked in the repo
```

Each invocation adds another timestamped file, so repeated use
accumulates litter that shows up in `git status` and can be committed by
accident. A command that inspects without deleting should not modify the
user's project tree as a side effect.

## Goals

- Default plan output resolves under the user's state directory.
- The printed path stays absolute and copy-pasteable into the follow-up
  `rclean clean --plan <path>` command.
- `--write-plan <PATH>` continues to override the default, unchanged.
- Environments with no home directory (CI sandboxes with stripped
  environments) still get a working path.

## Non-Goals

- Do not change what `free` selects or how it ranks candidates.
- Do not change ActionPlan schema, contents, or replay semantics.
- Do not add plan pruning, garbage collection, or retention policy.
- Do not change `--interactive`, which writes no plan file at all.
- Do not converge `graveyard::default_root()` onto the shared resolver
  in this change; AGENTS.md routes graveyard behavior to maintainer
  review. See Follow-up.

## User-Visible Behavior

Before:

```
wrote action plan: rclean-free-20260723T121738Z.json
review it, then run: rclean clean --plan rclean-free-20260723T121738Z.json
```

After:

```
wrote action plan: /Users/x/.local/state/rclean/plans/rclean-free-20260723T121738Z.json
review it, then run: rclean clean --plan /Users/x/.local/state/rclean/plans/rclean-free-20260723T121738Z.json
```

The working directory is left untouched.

## Acceptance Criteria

1. `rclean free <target>` with no `--write-plan` creates no file in the
   current working directory.
2. The plan file exists at the resolved state-directory path and is a
   valid ActionPlan that `rclean clean --plan <path>` accepts.
3. `$XDG_STATE_HOME` is honored when set and non-empty.
4. `--write-plan <PATH>` still writes exactly to `<PATH>`.
5. The printed `wrote action plan:` and `rclean clean --plan` lines both
   show the same resolved path.
6. With every home-directory environment variable unset, the command
   still succeeds, writing into a namespaced `./.rclean-plans/`
   directory rather than dropping loose files in the working directory.
7. A state directory that cannot be created is reported as an error.
   `free` never silently falls back to writing into the working
   directory, which would reintroduce the reported problem.

## Known Limitations

With every home-directory environment variable stripped — mainly CI
sandboxes — there is no user directory to resolve, so the plan is
written relative to the working directory under `./.rclean-plans/`.
This is one ignorable directory rather than the accumulating loose
files the issue reports, and it matches `graveyard::default_root()`'s
`./.rclean-graveyard` fallback, but it is not a fully clean working
directory. Callers that need a guaranteed location in such an
environment should pass `--write-plan`.

## Follow-up

`graveyard::default_root()` (`src/graveyard/mod.rs:44`) hand-rolls the
same platform environment-variable chain that this change introduces for
plans. Converging the two onto one resolver is the correct end state, but
graveyard behavior is a maintainer-review gate under AGENTS.md, so it is
tracked separately rather than folded in here.
