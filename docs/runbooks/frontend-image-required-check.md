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

`trigger-deploy` is deliberately *not* a dependency of the aggregator, so an
infra hiccup on a push cannot make a PR red. Note that the job is no longer
blanket-`continue-on-error` (that also hid a deploy the deployer *refused*):
only the OIDC step is tolerated, and the POST step warns on a transport error,
a 5xx or a 401/403 — issue #950's case — while failing the job on any other
4xx.

## What was wrong with the image build

**Read this heading literally: these are defects the files prove, not a
diagnosed root cause.** The CI logs of the 20 failed runs have still not been
read (the GitHub API was rate-limited while this landed) and the clone is
shallow (1506 commits, history does not reach 2026-06-16), so the breaking
commit was never bisected. Defects 2–4 are each sufficient to produce a broken
or non-reproducible image; which one actually reddened those runs is unproven.
Defect 1 is a correctness fix, **not** a cause — see its own caveat.

Fixed in `docker/frontend/{ppt-web,admin-web,reality-web}.Dockerfile`:

1. **Base image was `node:20-alpine` while the declared `engines` ranges had
   moved past Node 20.** `frontend/package.json` declares
   `engines.node: ^20.19.0 || >=22.12.0`, and `frontend/pnpm-lock.yaml` pins
   packages that exclude 20 outright — notably
   `rollup-plugin-visualizer@7.1.1` (`engines: {node: '>=22'}`, confirmed at
   `frontend/pnpm-lock.yaml:9924`), which
   `frontend/apps/ppt-web/vite.config.ts` imports on line 3 and therefore loads
   on every `vite build`, plus `commander@15.0.0` (`>=22.12.0`). Now
   `node:22-alpine`, which satisfies every `engines` range in the lockfile.

   **This was almost certainly NOT the cause of the red runs**, and the earlier
   version of this runbook overstated it. No `.npmrc` in this repo sets
   `engine-strict`, so pnpm only *warns* on a violated range; and
   `frontend.yml:171-176` builds ppt-web on `node-version: '20'` with pnpm 8
   against this exact lockfile, as a check that is already required on `dev`.
   Node 20 demonstrably builds ppt-web in CI. The change is still right — the
   image should run the Node the workspace asks for — but it buys correctness,
   not a fix.
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

## Mixed-revision deploys

Separately from CI, the deployer now compares the commit every service image
was built from — the thing that went unnoticed for three months here.
`BlueGreenDeployer::deploy` reads `org.opencontainers.image.revision` (stamped
into every image by T6) off each pulled image before anything is torn down:

```
mixed-revision deploy refused: the service images do not share one
org.opencontainers.image.revision (jun2026: admin-web, ppt-web, reality-web |
sep2026: api-server, reality-server). Every service must come from the same
commit; rebuild the lagging image(s) and retry.
```

What a disagreement COSTS depends on how the deploy was addressed
(`BlueGreenSpec::require_single_revision`). A **version**-addressed deploy can
promise one commit; a **mutable-tag** deploy cannot, so there the same finding
is a warning:

| deploy path | addressed by | on a mixed revision |
|---|---|---|
| `POST /api/promote` (a candidate registered by `release.yml` from a `v*` tag) | immutable version | **refused (400)**, every divergent service named |
| `POST /api/deploy` (branch auto-deploy, `:dev` / `:main`) | mutable branch tag | `warn!`, deploy proceeds |
| auto-rollback after a failed health grace, and `pmctl rollback` | a previously recorded release | `warn!`, rollback proceeds |

Refusing on the last two would be worse than the bug it catches: the branch tag
is rewritten by two separate workflow runs (`docker-build.yml` and
`docker-frontend.yml`), so between them the refs legitimately name different
commits, and a rollback that refuses leaves the bad colour live with
"AUTO-ROLLBACK FAILED — system in indeterminate state".

The divergence itself is closed **at the source**: both image workflows lost the
`paths:` filter on their branch `push:` trigger, so one push to `main`/`dev`
rebuilds all six images at one commit instead of refreshing only the half that
changed. The warn-path then covers the minutes between the two runs finishing.

And a refused deploy is no longer silent: the `trigger-deploy` jobs lost their
job-level `continue-on-error`, and their `POST /api/deploy` step fails the job
on a 4xx from the deployer while still tolerating a transport error, a 5xx or an
OIDC 401/403 (issue #950's case).

Per-verdict reasoning and unit tests are in
`backend/servers/deploy-server/src/domain/release.rs`; a release where NO image
carries the label is allowed and logged at `warn` on every path, because every
`Release` row written before T6 looks like that and `pmctl rollback` builds its
spec from those rows. Once every image in GHCR has been rebuilt post-T6, that
branch should be tightened to a refusal on the version-addressed path.
