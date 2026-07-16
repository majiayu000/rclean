#!/usr/bin/env bash

set -euo pipefail

script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
repo_root=$(cd "$script_dir/../.." && pwd)
verify_script="$script_dir/verify-release-version.sh"

cd "$repo_root"

package_version=$(
  cargo metadata --no-deps --format-version 1 \
    | jq -er '
        [.packages[] | select(.name == "rclean-cli") | .version]
        | if length == 1 then .[0]
          else error("expected exactly one rclean-cli package")
          end
      '
)

"$verify_script" "v${package_version}"

expect_failure() {
  local label=$1
  local expected_message=$2
  shift 2

  local output
  if output=$("$verify_script" "$@" 2>&1); then
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

expect_failure "missing argument" "usage: verify-release-version.sh"
expect_failure "missing v prefix" "release tag must start with v" "$package_version"
expect_failure "empty version" "release tag must start with v" "v"
expect_failure "malformed version" "does not match Cargo package version" "vnot-a-version"

mismatched_tag="v0.0.0-version-contract-mismatch"
if [[ "$mismatched_tag" == "v${package_version}" ]]; then
  mismatched_tag="v9999.0.0-version-contract-mismatch"
fi
expect_failure "mismatched version" "does not match Cargo package version" "$mismatched_tag"

echo "release version contract tests passed"
