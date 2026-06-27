# GH160 product spec: conservative developer cache rules

## Summary

Add conservative cache coverage for common developer tool roots without
turning rclean into a broad application cleaner. The implemented slices cover
exact Android SDK Manager caches, JetBrains IDE and Android Studio system
cache/log anchors, plus existing Homebrew and Dart pub-cache coverage.

## Problem

Developer machines accumulate large cache and log trees outside individual
projects. Users want `rclean scan --home` to surface those artifacts, but many
nearby directories contain configuration, SDKs, virtual devices, projects, or
other user-authored state.

## Goals

- Report exact Homebrew download, Dart pub-cache, Android SDK Manager,
  JetBrains IDE, and Android Studio cache/log anchors.
- Keep global dependency or IDE state conservative: `safe` only when the path
  is narrowly rebuildable, otherwise `caution`.
- Include the rules in `doctor`, `rules`, README tables, and `scan --home`
  reachability only where the root can be proven.
- Preserve scan-first behavior and ActionPlan review before cleanup.

## Non-goals

- No broad cleanup of `Application Support`, `.config`, `.local/share`,
  Android SDK components, Android virtual devices, IDE config, plugins,
  projects, or LocalHistory.
- No native `brew cleanup`, Android SDK manager integration, or IDE command
  execution in this slice.
- No arbitrary `caches`, `log`, `downloads`, `hosted`, or `git` name matching
  outside exact anchors.

## Safety Policy

- `homebrew.downloads` is `safe`: bottle/source archives are redownloaded by
  Homebrew.
- `dart.pub_hosted_cache` and `dart.pub_git_cache` are `caution`: deleting them
  can break offline builds and requires package redownload/reclone.
- Android SDK cache rules are `caution`: users should close Android Studio or
  `sdkmanager`; installed SDK packages, system images, NDKs, and AVDs are not
  selected.
- JetBrains and Android Studio cache/log rules are `caution`: users should
  close the IDE before removal, and the IDE recreates these directories on
  launch.

## Done When

- Positive and negative tests cover canonical and non-canonical paths.
- `scan --home` reaches the implemented roots on supported platforms.
- `doctor`, `rules`, and README list the new rule ids.
- CI passes on Linux, macOS, and Windows.
