#!/usr/bin/env bash
# Verifies that no accented characters leak into tracked files:
# the codebase is English-only. Locale files and binary assets are exempt.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

pattern='[àâçéèêëîïôùûüœÀÂÇÉÈÊËÎÏÔÙÛÜŒ]'

status=0
while IFS= read -r file; do
    case "$file" in
        crates/trinity-gui/locales/*) continue ;;
        crates/trinity-gui/assets/*) continue ;;
        Cargo.lock) continue ;;
    esac
    if grep -P "$pattern" "$file" 2>/dev/null | grep -qv 'lang-ok'; then
        printf 'LANG-KO %s\n' "$file" >&2
        grep -nP "$pattern" "$file" 2>/dev/null | grep -v 'lang-ok' | head -5 >&2
        status=1
    fi
done < <(git ls-files)

if [ "$status" -ne 0 ]; then
    exit 1
fi

echo "LANG-OK English-only codebase"
