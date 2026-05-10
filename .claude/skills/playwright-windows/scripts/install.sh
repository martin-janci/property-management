#!/usr/bin/env bash
# Symlink pw-win and playwright-cli-win into ~/bin/ so they are on PATH.
# Idempotent. Safe to re-run.

set -euo pipefail

here="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
target="${HOME}/bin"

mkdir -p "$target"

for name in pw-win playwright-cli-win; do
  src="${here}/${name}"
  dst="${target}/${name}"
  chmod +x "$src"
  if [[ -L "$dst" || -e "$dst" ]]; then
    if [[ "$(readlink -f "$dst" 2>/dev/null || true)" == "$src" ]]; then
      echo "ok  : ${dst} -> ${src}"
      continue
    fi
    echo "warn: ${dst} exists and points elsewhere; replacing" >&2
    rm -f "$dst"
  fi
  ln -s "$src" "$dst"
  echo "link: ${dst} -> ${src}"
done

case ":${PATH}:" in
  *":${target}:"*) ;;
  *) echo "note: ${target} is not on \$PATH. Add this to your shell rc:"
     echo "      export PATH=\"\${HOME}/bin:\${PATH}\""
     ;;
esac
