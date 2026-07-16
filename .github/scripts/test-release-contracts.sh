#!/usr/bin/env bash

set -euo pipefail

script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
repo_root=$(cd "$script_dir/../.." && pwd)
package_version_script="$script_dir/package-version.sh"
verify_script="$script_dir/verify-release-version.sh"
extract_script="$script_dir/extract-release-notes.sh"

cd "$repo_root"

package_version=$("$package_version_script")
package_id=$(cargo pkgid -p rclean-cli)
if [[ "$package_id" != *@"$package_version" ]]; then
  echo "contract failure: package-version.sh disagrees with cargo pkgid '$package_id'" >&2
  exit 1
fi

"$verify_script" "v${package_version}"

expect_failure() {
  local label=$1
  local expected_message=$2
  shift 2

  local output
  if output=$("$@" 2>&1); then
    echo "contract failure: $label unexpectedly succeeded" >&2
    exit 1
  fi
  if [[ "$output" != *"$expected_message"* ]]; then
    echo "contract failure: $label returned an unexpected error" >&2
    printf '%s\n' "$output" >&2
    exit 1
  fi
  printf 'verified expected failure: %s\n' "$label"
}

expect_failure "missing tag argument" "usage: verify-release-version.sh" "$verify_script"
expect_failure "missing v prefix" "release tag must start with v" "$verify_script" "$package_version"
expect_failure "empty tag version" "release tag must start with v" "$verify_script" "v"
expect_failure "malformed tag version" "does not match Cargo package version" "$verify_script" "vnot-a-version"

mismatched_tag="v0.0.0-version-contract-mismatch"
if [[ "$mismatched_tag" == "v${package_version}" ]]; then
  mismatched_tag="v9999.0.0-version-contract-mismatch"
fi
expect_failure "mismatched tag version" "does not match Cargo package version" "$verify_script" "$mismatched_tag"

temporary_dir=$(mktemp -d)
cleanup() {
  rm -f \
    "$temporary_dir/current.md" \
    "$temporary_dir/missing.md" \
    "$temporary_dir/empty.md" \
    "$temporary_dir/empty-output.md"
  rmdir "$temporary_dir"
}
trap cleanup EXIT

"$extract_script" "$package_version" CHANGELOG.md "$temporary_dir/current.md"
if ! grep -q '[^[:space:]]' "$temporary_dir/current.md"; then
  echo "contract failure: current release notes are empty" >&2
  exit 1
fi
if grep -q '^## ' "$temporary_dir/current.md"; then
  echo "contract failure: extraction crossed into the next release section" >&2
  exit 1
fi

expect_failure \
  "missing changelog version" \
  "missing CHANGELOG.md section" \
  "$extract_script" "9999.0.0-release-contract-missing" CHANGELOG.md "$temporary_dir/missing.md"

printf '## %s\n\n## previous\n\ncontent\n' "$package_version" >"$temporary_dir/empty.md"
expect_failure \
  "empty changelog version" \
  "CHANGELOG.md section for $package_version is empty" \
  "$extract_script" "$package_version" "$temporary_dir/empty.md" "$temporary_dir/empty-output.md"

if [[ -e "$temporary_dir/missing.md" || -e "$temporary_dir/empty-output.md" ]]; then
  echo "contract failure: failed extraction left an output file" >&2
  exit 1
fi

echo "release contract tests passed"
