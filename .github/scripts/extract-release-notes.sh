#!/usr/bin/env bash

set -euo pipefail

if [[ $# -ne 3 || -z $1 || -z $2 || -z $3 ]]; then
  echo "::error::usage: extract-release-notes.sh <version> <changelog> <output>" >&2
  exit 2
fi

version=$1
changelog=$2
output=$3

if [[ ! -f "$changelog" ]]; then
  echo "::error::changelog '$changelog' does not exist" >&2
  exit 1
fi

output_dir=$(dirname "$output")
if [[ ! -d "$output_dir" ]]; then
  echo "::error::output directory '$output_dir' does not exist" >&2
  exit 1
fi

temporary=$(mktemp "${output}.tmp.XXXXXX")
cleanup() {
  rm -f "$temporary"
}
trap cleanup EXIT

if awk -v version="$version" '
  BEGIN { heading = "## " version }
  found == 0 && ($0 == heading || index($0, heading " - ") == 1) { found = 1; next }
  found == 1 && /^## / { exit }
  found == 1 { print }
  END { if (found == 0) exit 2 }
' "$changelog" >"$temporary"; then
  :
else
  status=$?
  if [[ $status -eq 2 ]]; then
    echo "::error::missing CHANGELOG.md section for $version" >&2
  else
    echo "::error::failed to extract CHANGELOG.md section for $version" >&2
  fi
  exit 1
fi

if ! grep -q '[^[:space:]]' "$temporary"; then
  echo "::error::CHANGELOG.md section for $version is empty" >&2
  exit 1
fi

mv "$temporary" "$output"
trap - EXIT
