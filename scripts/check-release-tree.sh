#!/usr/bin/env sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"
failed=0
for forbidden in .beads .claude .mcp.json .release-please-manifest.json secretspec.toml; do
    if [ -e "$forbidden" ]; then
        printf 'private path present: %s\n' "$forbidden" >&2
        failed=1
    fi
done
if rg -n --hidden --glob '!scripts/check-release-tree.sh' --glob '!.git/**' --glob '!target/**' --glob '!book/build/**' '(\/work\/|\/Users\/|\/home\/|dotfiles|\.beads|\.claude)' .; then
    printf 'private path reference found\n' >&2
    failed=1
fi
exit "$failed"
