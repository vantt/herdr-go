#!/usr/bin/env bash
# Computes SHA-256 checksums for packaged release archives (.tar.gz, .zip)
# in a directory and prints them to stdout in sha256sum -c compatible
# format. The caller redirects the output to a file of its own choosing --
# this script never writes inside the scanned directory itself, since a
# self-included output file would corrupt the checksum set (D8, D10 of
# docs/history/self-update-merge-config/CONTEXT.md).
set -euo pipefail

if [ $# -ne 1 ]; then
  echo "usage: $0 <dist-dir>" >&2
  exit 1
fi

dir="$1"

if [ ! -d "$dir" ]; then
  echo "not a directory: $dir" >&2
  exit 1
fi

if command -v sha256sum >/dev/null 2>&1; then
  hash_cmd=(sha256sum)
elif command -v shasum >/dev/null 2>&1; then
  hash_cmd=(shasum -a 256)
else
  echo "neither sha256sum nor shasum found on PATH" >&2
  exit 1
fi

for f in "$dir"/*.tar.gz "$dir"/*.zip; do
  [ -f "$f" ] || continue
  hash="$("${hash_cmd[@]}" "$f" | awk '{print $1}')"
  printf '%s  %s\n' "$hash" "$(basename "$f")"
done
