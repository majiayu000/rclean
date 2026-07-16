#!/usr/bin/env bash

set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "::error::usage: verify-release-version.sh v<package-version>" >&2
  exit 2
fi

tag=$1
if [[ "$tag" != v* || "$tag" == "v" ]]; then
  echo "::error::release tag must start with v and include a version; got '$tag'" >&2
  exit 1
fi

for required_command in cargo jq; do
  if ! command -v "$required_command" >/dev/null 2>&1; then
    echo "::error::required command '$required_command' is unavailable" >&2
    exit 1
  fi
done

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

expected_tag="v${package_version}"
if [[ "$tag" != "$expected_tag" ]]; then
  echo "::error::release tag '$tag' does not match Cargo package version '$expected_tag'" >&2
  exit 1
fi

echo "verified release tag '$tag' matches rclean-cli package version '$package_version'"
