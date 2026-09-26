// Single source of the `{version, commit, built_at}` body both version routes
// serve (T6). reality-web is the one frontend image without nginx in front of
// it, so its /version surface is a Next route handler rather than an nginx
// `location` — but the JSON must be byte-compatible with what ppt-web and
// admin-web serve from docker/nginx/*.nginx.conf.template, otherwise "curl
// /version on every service" stops being one command.
//
// The values are baked into the image by docker/frontend/reality-web.Dockerfile
// (ARG APP_VERSION / GIT_SHA / BUILD_DATE -> ENV), fed from the release tag and
// github.sha — no git call and no I/O at request time. A build outside that
// pipeline reports "unknown", the same fallback the Rust servers and the nginx
// templates use.

export type VersionPayload = {
  version: string;
  commit: string;
  built_at: string;
};

export function versionPayload(): VersionPayload {
  return {
    version: process.env.APP_VERSION ?? 'unknown',
    commit: process.env.GIT_SHA ?? 'unknown',
    built_at: process.env.BUILD_DATE ?? 'unknown',
  };
}
