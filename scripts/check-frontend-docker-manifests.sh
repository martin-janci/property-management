#!/usr/bin/env bash
# Assert that each frontend Dockerfile's hand-written workspace-manifest COPY
# list still covers everything the app it builds actually needs.
#
# WHY THIS EXISTS (T7 / F-P8)
# ---------------------------
# docker/frontend/*.Dockerfile install dependencies in a `deps` stage that
# copies workspace `package.json` files ONE BY ONE, so the layer caches on
# manifests rather than on the whole source tree. pnpm resolves a
# `workspace:*` dependency by finding the depended-on package's manifest in
# the workspace; if it is missing, the build dies with
#
#     ERR_PNPM_... in the dependencies field, no project of name "@ppt/x" found
#
# The list is therefore load-bearing AND hand-maintained: add a
# `workspace:*` dep to any package in the chain and three Dockerfiles silently
# become wrong. `docker-frontend.yml` failed 20 of its last 20 runs while
# nothing blocked a merge, which is exactly the shape of failure a hand-list
# produces.
#
# This script recomputes the answer from the real manifests instead of
# trusting the comment above each list:
#
#   * it reads `pnpm --filter <pkg> build` out of each Dockerfile to learn
#     which app that image builds,
#   * it walks the `workspace:` dependency graph transitively from that app,
#   * it collects the manifest COPYs of the `deps` stage ONLY — the stage that
#     runs `pnpm install`. A manifest copied in a later stage cannot help that
#     install, so counting it would make this gate pass on a Dockerfile that
#     still fails to build,
#   * it fails if a required manifest is not COPY'd, if a COPY'd path does not
#     exist, or if a COPY lands a manifest in a directory that does not match
#     its workspace path (which would make pnpm see the wrong package name).
#
# Extra manifests are reported but do NOT fail: copying more than the closure
# is harmless (it only adds importers pnpm already has in the lockfile), and
# the three Dockerfiles historically share one list.
#
# frontend/apps/mobile is deliberately absent from every list: `.dockerignore`
# excludes `frontend/apps/mobile/` from the build context entirely, so a COPY
# of it could not succeed. It is listed in NOT_IN_CONTEXT below so that
# "missing" and "deliberately excluded" stay distinguishable.
#
# Usage: ./scripts/check-frontend-docker-manifests.sh
# Exits 0 when every list is complete, 1 otherwise. No network, no install.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

python3 - "$REPO_ROOT" <<'PY'
import json
import os
import re
import sys

root = sys.argv[1]
frontend = os.path.join(root, "frontend")

# Workspace packages whose manifest cannot be COPY'd because .dockerignore
# keeps them out of the build context. Keep in sync with .dockerignore.
NOT_IN_CONTEXT = {"frontend/apps/mobile/package.json"}

DOCKERFILES = [
    "docker/frontend/ppt-web.Dockerfile",
    "docker/frontend/admin-web.Dockerfile",
    "docker/frontend/reality-web.Dockerfile",
]

# ---------------------------------------------------------------------------
# 1. The real workspace: name -> manifest path, and the workspace dep graph.
# ---------------------------------------------------------------------------
manifests = {}  # package name -> repo-relative manifest path
deps = {}  # package name -> set of workspace dep names
for sub in ("packages", "apps"):
    base = os.path.join(frontend, sub)
    if not os.path.isdir(base):
        continue
    for entry in sorted(os.listdir(base)):
        path = os.path.join(base, entry, "package.json")
        if not os.path.isfile(path):
            continue
        with open(path, encoding="utf-8") as fh:
            pkg = json.load(fh)
        name = pkg.get("name")
        if not name:
            print(f"::error::{path} has no \"name\" field")
            sys.exit(1)
        rel = os.path.relpath(path, root)
        manifests[name] = rel
        ws = set()
        for field in (
            "dependencies",
            "devDependencies",
            "peerDependencies",
            "optionalDependencies",
        ):
            for dep, spec in (pkg.get(field) or {}).items():
                if isinstance(spec, str) and spec.startswith("workspace:"):
                    ws.add(dep)
        deps[name] = ws

failures = []

# ---------------------------------------------------------------------------
# 2. Per Dockerfile: which app does it build, and what does it copy?
# ---------------------------------------------------------------------------
copy_re = re.compile(r"^COPY\s+(?!--)(.*)$")
filter_re = re.compile(r"pnpm\s+--filter\s+(\S+)\s+build")
# `FROM <image> AS <stage>` — used to bound the COPY scan to the deps stage.
from_re = re.compile(r"^FROM\s+\S+(?:\s+AS\s+(\S+))?\s*$", re.IGNORECASE)
# The stage that runs `pnpm install`, i.e. the one whose manifest list decides
# whether the install can resolve the workspace at all.
DEPS_STAGE = "deps"

for dockerfile in DOCKERFILES:
    path = os.path.join(root, dockerfile)
    with open(path, encoding="utf-8") as fh:
        lines = fh.read().splitlines()

    built = filter_re.findall("\n".join(lines))
    if len(built) != 1:
        failures.append(
            f"{dockerfile}: expected exactly one `pnpm --filter <pkg> build`, found {built}"
        )
        continue
    app = built[0]
    if app not in manifests:
        failures.append(
            f"{dockerfile}: builds `{app}`, which is not a workspace package "
            f"(known: {', '.join(sorted(manifests))})"
        )
        continue

    # Only the `deps` stage counts. A manifest COPY'd in the builder or
    # production stage cannot help `pnpm install`, which runs in `deps` — and
    # scanning the whole file would let such a line satisfy a gate whose error
    # message says "deps stage does not copy ...". Stage tracking, not "stop at
    # the second FROM", so the check stays correct if the stage order changes.
    copied = set()
    stage = None
    saw_deps_stage = False
    for line in lines:
        stripped = line.strip()
        fm = from_re.match(stripped)
        if fm:
            stage = (fm.group(1) or "").lower() or None
            if stage == DEPS_STAGE:
                saw_deps_stage = True
            continue
        if stage != DEPS_STAGE:
            continue
        m = copy_re.match(stripped)
        if not m:
            continue
        parts = m.group(1).split()
        if len(parts) < 2:
            continue
        dest = parts[-1]
        for src in parts[:-1]:
            if not src.startswith("frontend/") or not src.endswith("package.json"):
                continue
            copied.add(src)
            # A manifest must land at its workspace-relative path, otherwise
            # pnpm-workspace.yaml's `packages/*` / `apps/*` globs pick it up
            # under the wrong directory name.
            want = os.path.dirname(src[len("frontend/") :])
            got = dest.lstrip("./").rstrip("/")
            if want and got != want:
                failures.append(
                    f"{dockerfile}: COPY {src} -> {dest}, expected destination ./{want}/"
                )

    if not saw_deps_stage:
        failures.append(
            f"{dockerfile}: no `FROM ... AS {DEPS_STAGE}` stage — this check reads the "
            f"workspace-manifest COPY list out of that stage, so it cannot verify this "
            f"Dockerfile. Rename the install stage back to `{DEPS_STAGE}`, or teach this "
            f"script the new name."
        )
        continue

    for src in sorted(copied):
        if not os.path.isfile(os.path.join(root, src)):
            failures.append(f"{dockerfile}: COPY {src} — no such file")

    # ----------------------------------------------------------------------
    # 3. Transitive closure of the built app's workspace deps.
    # ----------------------------------------------------------------------
    closure = set()
    queue = [app]
    while queue:
        name = queue.pop()
        if name in closure:
            continue
        closure.add(name)
        for dep in sorted(deps.get(name, ())):
            if dep not in manifests:
                failures.append(
                    f"{dockerfile}: `{name}` depends on `{dep}` via workspace:, "
                    "but no such workspace package exists"
                )
                continue
            queue.append(dep)

    # The workspace root manifest carries `packageManager`, the pnpm overrides
    # and the workspace globs — without it there is no workspace at all.
    required = {manifests[name] for name in closure} | {"frontend/package.json"}
    missing = sorted(required - copied - NOT_IN_CONTEXT)
    excluded = sorted(required & NOT_IN_CONTEXT)
    extra = sorted(copied - required)

    print(f"{dockerfile}: builds {app}; {len(closure)} workspace packages in closure")
    if extra:
        print(f"  note: copies {len(extra)} manifest(s) outside the closure: {', '.join(extra)}")
    if excluded:
        failures.append(
            f"{dockerfile}: {app} needs {', '.join(excluded)}, which .dockerignore "
            "keeps out of the build context — the image cannot be built as configured"
        )
    if missing:
        failures.append(
            f"{dockerfile}: the `{DEPS_STAGE}` stage does not copy {len(missing)} "
            f"required manifest(s): {', '.join(missing)}"
        )

# ---------------------------------------------------------------------------
if failures:
    print()
    for f in failures:
        print(f"::error::{f}")
    print()
    print(
        "Add the missing `COPY frontend/<path>/package.json ./<path>/` line(s) to the "
        "deps stage of the Dockerfile(s) named above, keeping the destination equal to "
        "the package's workspace-relative directory."
    )
    sys.exit(1)

print()
print("All frontend Dockerfile manifest lists cover their app's workspace closure.")
PY
