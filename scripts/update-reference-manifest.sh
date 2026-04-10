#!/bin/sh

set -eu

changed_files="$(git diff --cached --name-only --diff-filter=ACMR)"

if [ -z "$changed_files" ]; then
  exit 0
fi

needs_manifest_update=0

for path in $changed_files; do
  case "$path" in
    stdlib/*|tea-compiler/src/reference.rs|tea-compiler/src/stdlib/*|tea-cli/src/main.rs)
      needs_manifest_update=1
      break
      ;;
  esac
done

if [ "$needs_manifest_update" -eq 0 ]; then
  exit 0
fi

echo "Refreshing www/generated/reference-manifest.json"
bun run --cwd www generate:reference
git add www/generated/reference-manifest.json
