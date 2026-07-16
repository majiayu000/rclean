#!/usr/bin/env bash

set -euo pipefail

script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
package_version_script="$script_dir/package-version.sh"

if [[ $# -ne 1 ]]; then
  echo "::error::usage: verify-release-version.sh v<package-version>" >&2
  exit 2
fi

tag=$1
if [[ "$tag" != v* || "$tag" == "v" ]]; then
  echo "::error::release tag must start with v and include a version; got '$tag'" >&2
  exit 1
fi

if ! package_version=$("$package_version_script"); then
  echo "::error::failed to determine rclean-cli package version" >&2
  exit 1
fi

expected_tag="v${package_version}"
if [[ "$tag" != "$expected_tag" ]]; then
  echo "::error::release tag '$tag' does not match Cargo package version '$expected_tag'" >&2
  exit 1
fi

echo "verified release tag '$tag' matches rclean-cli package version '$package_version'"
