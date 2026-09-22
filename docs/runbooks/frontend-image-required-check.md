# Runbook — the frontend image build must be a required check

**Status:** the workflow side is done (T7). The branch-protection side needs ONE
command from a repo admin; a workflow's `GITHUB_TOKEN` cannot edit branch
protection, so it cannot be automated from inside CI without an admin token.

## What went wrong (F-P8)

`docker-frontend.yml` failed **20 of its 20 most recent runs** — newest
2026-09-08, last success **2026-06-16**. For those three months:

- `docker-build.yml` (backend) kept succeeding and its `trigger-deploy` job kept
  POSTing `{"tag":"dev"}` to `https://onyx.rlt.sk/api/deploy`;
- the deployer pulls `ppt-web:dev`, `ppt-reality-web:dev`, `ppt-admin-web:dev`
  by that **mutable** tag, so it kept getting the last image that built — a
  **June** frontend — and pairing it with a **September** backend and API
  schema;
- nothing anywhere turned red. `docker-frontend.yml` ran only on `push`, so it
  produced no status on any pull request, and its own `trigger-deploy` job is
  gated `needs: build`, so a failed build simply produced *no deploy* rather
  than a failed one.

A required status check is the only thing that converts "the frontend image
does not build" into "this PR cannot merge".

## The one command an admin must run

Additive — it appends to the existing required-check list and touches nothing
else on the rule (the `contexts` sub-resource is add-only, unlike a `PATCH` of
`required_status_checks`, which replaces the whole list):

```bash
gh api -X POST \
  repos/martin-janci/property-management/branches/dev/protection/required_status_checks/contexts \
  -f 'contexts[]=frontend-images'
```

Verify:

```bash
gh api repos/martin-janci/property-management/branches/dev/protection/required_status_checks \
  --jq '.contexts'
# must now contain "frontend-images"
```

Requires a token with the `administration:write` repository permission — the
default `gh` login of a repo admin has it; `secrets.GITHUB_TOKEN` does not.

### Or let the existing workflow do it

`frontend-images` has been added to `branch-protection-setup.yml`'s
`NEW_CHECKS` list, so the repo's normal mechanism covers it:

```bash
gh secret set GH_ADMIN_TOKEN --body "<pat with administration:write>"   # once
gh workflow run branch-protection-setup.yml --ref dev -f dry_run=true   # preview
gh workflow run branch-protection-setup.yml --ref dev                   # apply
```

That path is idempotent and rebuilds the payload from the *current* protection
object, so it cannot silently weaken another setting (#771/#923).

## Why `frontend-images` is the right context to require

`docker-frontend.yml` now runs on **every** pull request with **no `paths:`
filter**, and ends in an always-reporting aggregator job whose id —
`frontend-images` — is the status context. It is the sole producer of that
context, the same single-producer shape `frontend.yml` and `backend.yml` use
(#1672):

| PR shape | `changes.frontend` | image build | `frontend-images` |
|---|---|---|---|
| touches `frontend/**`, `docker/frontend/**`, `docker/nginx/**` or the workflows | `true` | runs | red unless the manifest gate **and** all three image builds succeeded |
| pure backend / docs | `false` | skipped | **green as a no-op** |

So requiring it cannot wedge a PR that does not touch the frontend — the
failure mode that makes people un-require a check.

Do **not** require the matrix jobs (`build / build (ppt-web)` …): their context
names change with the matrix, and a required context that stops being produced
blocks every PR.

`trigger-deploy` is deliberately *not* a dependency of the aggregator. It is
`continue-on-error` by design (issue #950: an OIDC 403 while onyx's `auth.yaml`
is being edited must not fail an image build that already pushed), and an infra
hiccup must not make a PR red.

## What was actually wrong with the image build

Fixed in `docker/frontend/{ppt-web,admin-web,reality-web}.Dockerfile`:

1. **Base image was `node:20-alpine` while the workspace toolchain had moved
   past Node 20.** `frontend/package.json` declares
   `engines.node: ^20.19.0 || >=22.12.0`, but `frontend/pnpm-lock.yaml` now
   pins packages that exclude 20 outright — notably
   `rollup-plugin-visualizer@7.1.1` (`engines: {node: '>=22'}`), which
   `frontend/apps/ppt-web/vite.config.ts` imports on line 3 and therefore loads
   on every `vite build`, plus `commander@15.0.0` (`>=22.12.0`). Now
   `node:22-alpine`, which satisfies every `engines` range in the lockfile.
2. **The pnpm pin was inert and disagreed with everything else.** The
   Dockerfiles ran `corepack prepare pnpm@9.15.0 --activate`, but with corepack
   enabled the `packageManager` field of the package.json in the working
   directory always wins — and `frontend/package.json` says `pnpm@8.14.0`. The
   images were silently running 8.14.0 while claiming 9.15.0. Confirmed
   locally: `corepack pnpm --version` in a tree carrying that package.json
   prints `8.14.0`. Replaced with `corepack enable && corepack install`, which
   reads the pin — one pnpm version across `frontend.yml`
   (`pnpm/action-setup` `version: 8`), the lockfile (`lockfileVersion: '6.0'`,
   a pnpm-8 format) and every image.
3. **`pnpm install` had no `--frozen-lockfile`.** Every spec in the workspace is
   `^`-ranged, so the image re-resolved dependencies on each build: the bundle
   CI tested and the bundle that shipped could contain different versions, and a
   lockfile that no longer matched the workspace failed *silently* instead of
   loudly. Now `pnpm install --frozen-lockfile`.
4. **The hand-written workspace-manifest `COPY` list could rot silently.**
   `scripts/check-frontend-docker-manifests.sh` recomputes the transitive
   closure of each app's `workspace:*` dependencies from the real
   `package.json` files and fails when a Dockerfile's list no longer covers it
   (also catching a COPY whose destination does not match the package's
   workspace path, and a COPY of a path that does not exist). It runs as the
   `manifests` job of `docker-frontend.yml` on every event — seconds, no Docker,
   no install.

### Still to confirm on the first real run

The GitHub API was rate-limited while this landed, so the CI logs for the 20
failed runs could not be read, and the clone is shallow (1506 commits, history
does not reach 2026-06-16), so the breaking commit could not be bisected. The
four defects above are the ones the files prove. If the first
`docker-frontend.yml` run after this change is still red, capture the failing
step and check, in this order:

1. the `deps` stage `pnpm install --frozen-lockfile` — if it reports
   `ERR_PNPM_OUTDATED_LOCKFILE`, `frontend/pnpm-lock.yaml` is genuinely out of
   step with the manifests and needs regenerating on `dev` (this was **not** the
   case locally: the exact manifest set each Dockerfile copies installs clean
   against the committed lockfile and leaves it byte-identical);
2. the `builder` stage `pnpm --filter … build` — a musl/native-binding failure
   (`@tailwindcss/oxide`, `lightningcss`, `@rolldown/binding`, `@next/swc`)
   would point at alpine rather than at Node. All `*-musl` variants are present
   in the lockfile, so this should not fire;
3. `frontend.yml` itself, which still pins `node-version: '20'` in all five of
   its jobs and so violates the same `engines` constraint as the old image did.
   Aligning it to 22 is a follow-up, deliberately out of T7's scope.

## Mixed-revision deploys are now refused

Separately from CI, the deployer no longer accepts a release whose services
disagree about which commit they were built from — the situation that shipped
for three months here. `BlueGreenDeployer::deploy` reads
`org.opencontainers.image.revision` (stamped into every image by T6) off each
pulled image and refuses with `400` before anything is torn down:

```
mixed-revision deploy refused: the service images do not share one
org.opencontainers.image.revision (jun2026: admin-web, ppt-web, reality-web |
sep2026: api-server, reality-server). Every service must come from the same
commit; rebuild the lagging image(s) and retry.
```

Policy, with the reasoning and unit tests in
`backend/servers/deploy-server/src/domain/release.rs`:

| images | outcome |
|---|---|
| all share one revision | deploy proceeds, commit logged |
| revisions disagree | **refused (400)**, every divergent service named |
| some labelled, some not | **refused (400)** — an unlabelled image cannot be shown to be the same build |
| none labelled | allowed, logged at `warn` — every `Release` row written before T6 looks like this, and `pmctl rollback` builds its spec from those rows |

Once every image in GHCR has been rebuilt post-T6, the "none labelled" branch
should be tightened to a refusal as well; it exists only to keep the rollback
path to pre-T6 releases alive.
