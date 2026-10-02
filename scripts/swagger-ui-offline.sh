#!/usr/bin/env bash
#
# swagger-ui-offline.sh — make `utoipa-swagger-ui` buildable when github.com is
# egress-blocked (issue #2966).
#
# The crate's build script downloads
#   https://github.com/swagger-api/swagger-ui/archive/refs/tags/v<ver>.zip
# and 403s behind the cloud-runner proxy, so every cargo command that compiles
# api-server / reality-server fails before clippy or tests even start. The
# same Swagger-UI `dist/` files ship on npm as `swagger-ui-dist`, and the npm
# registry is reachable. This script repackages that tarball into the zip
# layout the build script expects (`<top>/dist/...`) and prints a
# `file://` URL for SWAGGER_UI_DOWNLOAD_URL.
#
# Output (stdout): the file:// URL, or NOTHING when the default GitHub URL is
# reachable (caller then keeps the crate's default) or when the fallback could
# not be prepared. Diagnostics go to stderr. Always exits 0 so callers can use
# `URL="$(scripts/swagger-ui-offline.sh)"` unconditionally.
#
# Usage:
#   export SWAGGER_UI_DOWNLOAD_URL="$(scripts/swagger-ui-offline.sh)"
#   [ -n "$SWAGGER_UI_DOWNLOAD_URL" ] || unset SWAGGER_UI_DOWNLOAD_URL

set -uo pipefail

DEFAULT_VER="5.17.14"
# Prefer the version pinned by the utoipa-swagger-ui build script in the local
# cargo registry, so the offline bundle matches what CI downloads.
VER="$(grep -hoE 'swagger-ui/archive/refs/tags/v[0-9]+\.[0-9]+\.[0-9]+\.zip' \
  "${CARGO_HOME:-$HOME/.cargo}"/registry/src/*/utoipa-swagger-ui-*/build.rs 2>/dev/null \
  | sort -V | tail -1 | sed -E 's/.*\/v([0-9.]+)\.zip/\1/')"
VER="${VER:-$DEFAULT_VER}"

GH_URL="https://github.com/swagger-api/swagger-ui/archive/refs/tags/v${VER}.zip"
if curl -fsSIL --max-time 8 -o /dev/null "$GH_URL" 2>/dev/null; then
  exit 0  # GitHub reachable: keep the crate's default download
fi

CACHE_DIR="${XDG_CACHE_HOME:-$HOME/.cache}/ppt"
ZIP="$CACHE_DIR/swagger-ui-${VER}.zip"
if [ -s "$ZIP" ]; then
  echo "file://$ZIP"
  exit 0
fi

mkdir -p "$CACHE_DIR" || exit 0
TGZ="$(mktemp "$CACHE_DIR/swagger-ui-dist.XXXXXX.tgz")" || exit 0
trap 'rm -f "$TGZ"' EXIT

NPM_URL="https://registry.npmjs.org/swagger-ui-dist/-/swagger-ui-dist-${VER}.tgz"
if ! curl -fsSL --max-time 60 -o "$TGZ" "$NPM_URL"; then
  echo "swagger-ui-offline: github.com and $NPM_URL both unreachable; leaving SWAGGER_UI_DOWNLOAD_URL unset" >&2
  exit 0
fi

# npm tarball entries are `package/<file>`; the build script wants
# `<top>/dist/<file>` (it keeps only entries whose 2nd path component is `dist`).
if ! python3 - "$TGZ" "$ZIP.tmp" "swagger-ui-${VER}" <<'PY'
import sys, tarfile, zipfile
src, dst, top = sys.argv[1:4]
with tarfile.open(src, "r:gz") as tf, zipfile.ZipFile(dst, "w", zipfile.ZIP_DEFLATED) as zf:
    zf.writestr(f"{top}/", "")
    zf.writestr(f"{top}/dist/", "")
    for m in tf.getmembers():
        if not m.isfile() or not m.name.startswith("package/"):
            continue
        zf.writestr(f"{top}/dist/{m.name[len('package/'):]}", tf.extractfile(m).read())
PY
then
  rm -f "$ZIP.tmp"
  echo "swagger-ui-offline: repackaging failed; leaving SWAGGER_UI_DOWNLOAD_URL unset" >&2
  exit 0
fi
mv "$ZIP.tmp" "$ZIP"
echo "swagger-ui-offline: github.com blocked; using npm swagger-ui-dist@${VER} -> $ZIP" >&2
echo "file://$ZIP"
