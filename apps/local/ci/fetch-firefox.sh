#!/bin/sh
# For the workflows: downloads the Firefox ESR of firefox.lock from Mozilla, and checks it against the checksum in the lock and
# against Mozilla's own SHA256SUMS of that release, both.
#
#   sh apps/local/ci/fetch-firefox.sh OUT_FILE
set -eu

out=${1:?the file to write}
here=$(cd "$(dirname "$0")" && pwd)
# shellcheck disable=SC1091
. "$here/firefox.lock"

base="$FIREFOX_BASE/$FIREFOX_VERSION"
work=$(mktemp -d "${TMPDIR:-/tmp}/rnl-fetch.XXXXXX")
trap 'rm -rf "$work"' EXIT

curl -fL --proto '=https' --tlsv1.2 --retry 3 -o "$out" "$base/$FIREFOX_PATH"
curl -fL --proto '=https' --tlsv1.2 --retry 3 -o "$work/SHA256SUMS" "$base/SHA256SUMS"

sum=$(sha256sum "$out" | cut -d' ' -f1)
listed=$(grep " $FIREFOX_PATH\$" "$work/SHA256SUMS" | cut -d' ' -f1)
echo "file:    $sum"
echo "lock:    $FIREFOX_SHA256"
echo "Mozilla: $listed"
[ "$sum" = "$FIREFOX_SHA256" ] || { echo "fetch-firefox.sh: the file is not the one in firefox.lock" >&2; exit 1; }
[ "$sum" = "$listed" ] || { echo "fetch-firefox.sh: the file is not the one Mozilla's SHA256SUMS lists" >&2; exit 1; }
ls -l "$out"
