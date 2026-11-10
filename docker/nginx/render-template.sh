#!/bin/sh
# Render the nginx config template at container startup so the /api proxy
# upstream resolves to ${BG_TARGET}-api-${BG_COLOR}, matching the blue/green
# topology the deploy-server brings up.
#
# Falls back to "_unset_" markers when the env vars aren't set (e.g. someone
# runs the image outside the deploy-server flow). nginx will fail to resolve
# `_unset_-api-_unset_` and return 502 on /api requests — preferable to
# silently routing /api to the SPA fallback.

set -eu

: "${BG_TARGET:=_unset_}"
: "${BG_COLOR:=_unset_}"
# Space-separated origins allowed to frame ppt-web when ?layoutPreview=1 is
# present (mirrors reality-web's env). Safe empty default = carve-out disabled.
: "${LAYOUT_PREVIEW_FRAME_ANCESTORS:=}"
# Version identity for the `/version` endpoint. The Dockerfile sets these from
# the APP_VERSION / GIT_SHA / BUILD_DATE build args, so the defaults below only
# ever apply to an image built outside the release pipeline — in which case
# /version answers "unknown" rather than serving an unsubstituted `${...}`.
: "${APP_VERSION:=unknown}"
: "${GIT_SHA:=unknown}"
: "${BUILD_DATE:=unknown}"

export BG_TARGET BG_COLOR LAYOUT_PREVIEW_FRAME_ANCESTORS
export APP_VERSION GIT_SHA BUILD_DATE

# Only these vars are substituted. nginx's own `$host`, `$remote_addr`
# etc. must NOT be expanded — they're nginx variables, evaluated per request.
envsubst '${BG_TARGET} ${BG_COLOR} ${LAYOUT_PREVIEW_FRAME_ANCESTORS} ${APP_VERSION} ${GIT_SHA} ${BUILD_DATE}' \
    < /etc/nginx/conf.d/default.conf.template \
    > /etc/nginx/conf.d/default.conf

echo "rendered nginx config: BG_TARGET=${BG_TARGET} BG_COLOR=${BG_COLOR} APP_VERSION=${APP_VERSION} GIT_SHA=${GIT_SHA}"
