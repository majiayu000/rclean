#!/usr/bin/env bash

set -euo pipefail

script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
repo_root=$(cd "$script_dir/../.." && pwd)

for required_command in cargo jq; do
  if ! command -v "$required_command" >/dev/null 2>&1; then
    echo "::error::required command '$required_command' is unavailable" >&2
    exit 1
  fi
done

cd "$repo_root"

if ! metadata=$(cargo metadata --no-deps --format-version 1); then
  echo "::error::failed to read Cargo package metadata" >&2
  exit 1
fi

if ! package_version=$(
  jq -er '
    [.packages[] | select(.name == "rclean-cli") | .version]
    | if length == 1 then .[0]
      else error("expected exactly one rclean-cli package")
      end
  ' <<<"$metadata"
); then
  echo "::error::Cargo metadata must contain exactly one rclean-cli package" >&2
  exit 1
fi

printf '%s\n' "$package_version"
