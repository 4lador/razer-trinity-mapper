#!/usr/bin/env bash
# Verifies the hexagonal dependency rule.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
    printf 'ARCH-KO %s\n' "$*" >&2
    exit 1
}

# Dependency names declared in the [dependencies] section of a Cargo.toml.
deps_of() {
    awk '
        /^\[dependencies\]/ { in_deps = 1; next }
        /^\[/               { in_deps = 0 }
        in_deps && /^[[:space:]]*[A-Za-z0-9_-]+[[:space:]]*=/ {
            line = $1
            sub(/=.*/, "", line)
            gsub(/[[:space:]]/, "", line)
            print line
        }
    ' "$1"
}

internal_deps_of() {
    deps_of "$1" | grep '^trinity-' || true
}

[ -f "$root/Cargo.toml" ] || fail "missing root Cargo.toml"

# 1. trinity-core: pure domain, zero dependencies.
core="$root/crates/trinity-core/Cargo.toml"
[ -f "$core" ] || fail "missing crates/trinity-core/Cargo.toml"
core_deps="$(deps_of "$core")"
[ -z "$core_deps" ] || fail "trinity-core must have zero dependencies (found: $(echo "$core_deps" | tr '\n' ' '))"

# 2. trinity-app: ports and use cases, only sees trinity-core.
app="$root/crates/trinity-app/Cargo.toml"
if [ -f "$app" ]; then
    while IFS= read -r dep; do
        [ "$dep" = "trinity-core" ] || fail "trinity-app may only depend on trinity-core (found: $dep)"
    done < <(internal_deps_of "$app")
fi

# 3. trinity-infra: adapters, only sees core and app.
infra="$root/crates/trinity-infra/Cargo.toml"
if [ -f "$infra" ]; then
    while IFS= read -r dep; do
        case "$dep" in
            trinity-core | trinity-app) ;;
            *) fail "trinity-infra may only depend on trinity-core/trinity-app (found: $dep)" ;;
        esac
    done < <(internal_deps_of "$infra")
fi

# 4. Composition roots: app required, infra tolerated, never another root.
for crate in trinity-gui trinity-daemon; do
    manifest="$root/crates/$crate/Cargo.toml"
    [ -f "$manifest" ] || continue
    while IFS= read -r dep; do
        case "$dep" in
            trinity-core | trinity-app | trinity-infra) ;;
            *) fail "$crate must not depend on internal crate $dep" ;;
        esac
    done < <(internal_deps_of "$manifest")
done

# 5. The workspace only declares the crates known to the plan.
known="trinity-core trinity-app trinity-infra trinity-gui trinity-daemon"
while IFS= read -r member; do
    name="$(basename "$member")"
    echo "$known" | grep -qw "$name" || fail "unexpected crate in workspace: $name"
done < <(grep -o '"crates/[^"]*"' "$root/Cargo.toml" | tr -d '"')

echo "ARCH-OK hexagonal rule satisfied"
