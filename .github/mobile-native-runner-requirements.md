# Mobile-Native CI Cloud-Runner Requirements

Runner spec for the Reality Portal Kotlin Multiplatform app (`mobile-native/`).
Written to unblock the mobile-native backlog by making the cloud-runner
requirement explicit and confirmable. This document **does not provision**
any runner — standing up or enabling runners is an operator action (see
"Operator confirmation checklist" below).

## Current runner topology (confirmed from `.github/workflows/mobile-native.yml`)

| Job             | `runs-on`      | Work                                                     | Needs                       |
| --------------- | -------------- | ------------------------------------------------------- | --------------------------- |
| `lint`          | `ubuntu-latest`| `./gradlew spotlessCheck`                                | JDK 17 only                 |
| `build-android` | `ubuntu-latest`| `:shared:build`, `:androidApp:assembleDebug`            | JDK 17 + Android SDK (CLI)  |
| `build-ios`     | `macos-latest` | `:shared:linkDebugFrameworkIosSimulatorArm64`           | **macOS** (Xcode toolchain) |

Both GitHub-hosted runner classes used here (`ubuntu-latest`, `macos-latest`)
are already wired into the workflow and are sufficient for the **build + lint**
work the backlog depends on. The Kotlin/Compose shared + Android slices run on
the free `ubuntu-latest` tier; the iOS framework link is the only job that
requires the billable macOS tier.

## Gap analysis (why the backlog can read as "blocked on a runner")

1. **No unit-test job.** `mobile-native.yml` runs `spotlessCheck` + builds but
   never runs `./gradlew test` (or `:shared:allTests`). The CI gates table in
   `CLAUDE.md` claims this workflow verifies "Gradle build + unit tests" — the
   test half is not yet wired. Adding it is a pure `ubuntu-latest` job (no new
   runner class).
2. **No instrumented / emulator job.** Android connected tests
   (`connectedDebugAndroidTest`) need a KVM-enabled runner plus an emulator
   (e.g. `reactivecircus/android-emulator-runner`). GitHub-hosted
   `ubuntu-latest` supports nested virtualization for this, so it is a workflow
   addition, **not** a new runner provision.
3. **iOS beyond framework-link** (simulator unit/UI tests) requires the macOS
   tier, which is already available via `macos-latest` but is **billable** — the
   only item that can be gated by org billing rather than by repo config.

None of 1–3 require a *new* or *self-hosted* runner to be stood up. They require
(a) workflow jobs to be authored, and (b) the macOS tier to be confirmed enabled
for the org's Actions billing.

## Operator confirmation checklist (cannot be done from a PR)

- [ ] Confirm GitHub Actions **macOS minutes are enabled** for the org/repo
      billing plan (private repos meter macOS at a higher multiplier; a disabled
      plan is what would actually block `build-ios`).
- [ ] Confirm the repo's Actions settings permit `ubuntu-latest` **nested
      virtualization** (default-on for GitHub-hosted; blocked only by org policy)
      if emulator/instrumented tests are added per gap #2.
- [ ] Decide self-hosted vs. GitHub-hosted for macOS: if macOS minutes are not
      budgeted, register a self-hosted macOS runner with label `macos-selfhosted`
      and update `build-ios`'s `runs-on`. (Infra/credentials task — out of scope
      for in-repo changes.)

## Conclusion

For the mobile-native backlog, the cloud-runner requirement is **confirmed, not
missing**: `ubuntu-latest` + `macos-latest` already cover lint, Android build,
and the iOS framework link. What remains before the backlog is unblocked is
operator confirmation of macOS Actions billing (checklist above) and authoring
the missing test/emulator jobs (gaps #1–#2) — both tracked here so the backlog
items can proceed against a known-good runner topology.
