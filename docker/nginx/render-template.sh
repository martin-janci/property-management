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

export BG_TARGET BG_COLOR LAYOUT_PREVIEW_FRAME_ANCESTORS

# Only these vars are substituted. nginx's own `$host`, `$remote_addr`
# etc. must NOT be expanded — they're nginx variables, evaluated per request.
envsubst '${BG_TARGET} ${BG_COLOR} ${LAYOUT_PREVIEW_FRAME_ANCESTORS}' \
    < /etc/nginx/conf.d/default.conf.template \
    > /etc/nginx/conf.d/default.conf

echo "rendered ppt-web nginx config: BG_TARGET=${BG_TARGET} BG_COLOR=${BG_COLOR}"
