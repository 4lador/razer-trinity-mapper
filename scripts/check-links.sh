#!/usr/bin/env bash
# Verifies that documentation links referenced in README.md are reachable.
# Some sites block bots (403) — those are accepted with a warning.
# This script requires network access: skip locally with `#[ignore]`
# or run only in CI.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readme="$root/README.md"

[ -f "$readme" ] || { echo "LINK-KO README.md not found" >&2; exit 1; }

# Extract all http(s) links from the README (excluding badges/images)
links=$(grep -oE 'https://[^ )>]+' "$readme" | grep -v 'img.shields.io' | sort -u)

if [ -z "$links" ]; then
    echo "LINK-OK no external links found"
    exit 0
fi

user_agent="Mozilla/5.0 (X11; Linux x86_64) razer-trinity-mapper-link-check"
timeout=10
failures=0

while IFS= read -r url; do
    code=$(curl -s -o /dev/null -w "%{http_code}" -L \
        -A "$user_agent" --max-time "$timeout" "$url" 2>/dev/null || echo "000")

    case "$code" in
        200|301|302)
            ;;
        403)
            printf 'LINK-WARN %s (403 — bot blocking, likely fine in browser)\n' "$url"
            ;;
        *)
            printf 'LINK-KO %s (got %s)\n' "$url" "$code" >&2
            failures=$((failures + 1))
            ;;
    esac
done <<< "$links"

if [ "$failures" -gt 0 ]; then
    printf 'LINK-KO %d broken link(s)\n' "$failures" >&2
    exit 1
fi

echo "LINK-OK all documentation links reachable"
