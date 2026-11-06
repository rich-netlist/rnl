#!/bin/sh
# Puts Mozilla's official Firefox ESR, unmodified, beside rnl-viewer-local, where the program looks for it first: a folder
# `firefox` next to the program. For a computer whose own Firefox is older than the page needs (Firefox 115 ESR or newer), with
# no root: nothing is installed, and the user's own Firefox and its profile are never touched.
#
#   sh get-firefox.sh                 download the Firefox ESR this version of the program was tested with, check it, unpack it
#   sh get-firefox.sh --from FILE     the same from a file already downloaded (firefox-VERSION.tar.xz): for a computer with
#                                     no network, which has it copied from another
#   sh get-firefox.sh --version V     another ESR of Mozilla's (such as a newer one): checked against Mozilla's own SHA256SUMS
#
# The file is checked against the checksum of Mozilla's SHA256SUMS of that release, which is also kept in firefox.lock for the
# version the program was tested with. The Firefox is Mozilla's, under its own licenses (in the folder), and is not changed.
set -eu

here=$(cd "$(dirname "$0")" && pwd)
# shellcheck disable=SC1091
. "$here/firefox.lock"

version=$FIREFOX_VERSION
from=
while [ $# -gt 0 ]; do
  case "$1" in
    --from) from=${2:?--from needs a file}; shift 2 ;;
    --version) version=${2:?--version needs a version, such as 153.6.0esr}; shift 2 ;;
    -h|--help) sed -n '2,13p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "get-firefox.sh: $1 is not an option" >&2; exit 2 ;;
  esac
done

if [ -e "$here/firefox" ]; then
  echo "get-firefox.sh: $here/firefox is there already; move it away to get another" >&2
  exit 1
fi
command -v xz >/dev/null 2>&1 || { echo "get-firefox.sh: xz is needed to unpack Firefox (yum install xz, or dnf install xz)" >&2; exit 1; }
command -v sha256sum >/dev/null 2>&1 || { echo "get-firefox.sh: sha256sum is needed" >&2; exit 1; }

path="linux-x86_64/en-US/firefox-$version.tar.xz"
base="$FIREFOX_BASE/$version"
work=$(mktemp -d "${TMPDIR:-/tmp}/rnl-firefox.XXXXXX")
trap 'rm -rf "$work"' EXIT

fetch() { # URL FILE
  if command -v curl >/dev/null 2>&1; then curl -fL --proto '=https' --tlsv1.2 -o "$2" "$1"
  elif command -v wget >/dev/null 2>&1; then wget --secure-protocol=TLSv1_2 -O "$2" "$1"
  else echo "get-firefox.sh: curl or wget is needed to download (or copy the file from another computer, and use --from)" >&2; return 1; fi
}

archive=$work/firefox.tar.xz
if [ -n "$from" ]; then
  cp "$from" "$archive"
else
  echo "Downloading $base/$path"
  fetch "$base/$path" "$archive"
fi
sum=$(sha256sum "$archive" | cut -d' ' -f1)

if [ "$version" = "$FIREFOX_VERSION" ]; then
  [ "$sum" = "$FIREFOX_SHA256" ] || { echo "get-firefox.sh: the file is not the Firefox $version that was tested: sha256 $sum, not $FIREFOX_SHA256" >&2; exit 1; }
else
  # (a version that was not tested: Mozilla's own list of the release is what it is checked against)
  if [ -n "$from" ]; then
    echo "get-firefox.sh: with --from, only the version in firefox.lock ($FIREFOX_VERSION) can be checked" >&2
    exit 1
  fi
  fetch "$base/SHA256SUMS" "$work/SHA256SUMS"
  listed=$(grep " $path\$" "$work/SHA256SUMS" | cut -d' ' -f1)
  [ -n "$listed" ] && [ "$sum" = "$listed" ] || { echo "get-firefox.sh: the file is not what Mozilla's SHA256SUMS lists for $path" >&2; exit 1; }
fi
echo "sha256 $sum: as listed"

mkdir "$work/unpacked"
tar -xJf "$archive" -C "$work/unpacked"
[ -x "$work/unpacked/firefox/firefox" ] || { echo "get-firefox.sh: no firefox/firefox in the file" >&2; exit 1; }
mv "$work/unpacked/firefox" "$here/firefox"
"$here/firefox/firefox" --version || true
echo "Firefox is in $here/firefox: rnl-viewer-local uses it from now on."
