#!/bin/sh
# Makes the packages of the local program (apps/local) for Linux x86_64 from the static program:
#
#   sh apps/local/ci/package.sh PROGRAM VERSION OUT_DIR [FIREFOX_TARBALL]
#
#   rnl-viewer-local-VERSION-linux-x86_64.tar.gz                the program, its README, the licence, and get-firefox.sh with
#                                                               firefox.lock, which puts Mozilla's Firefox ESR beside it
#   rnl-viewer-local-VERSION-linux-x86_64-with-firefox.tar.gz   the same with Mozilla's official Firefox ESR, unmodified,
#                                                               in the folder `firefox`, where the program looks first
#
# The second only if the Firefox tarball is given (already checked against firefox.lock). Firefox is unpacked as it is: nothing
# is added to it, taken from it or changed in it (Mozilla's distribution policy for unaltered software); what is added is
# beside it, in FIREFOX-NOTICE.txt. SHA256SUMS.txt lists the files made.
set -eu

program=${1:?the program}
version=${2:?the version}
out=${3:?the output folder}
firefox=${4:-}
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../../.." && pwd)
# shellcheck disable=SC1091
. "$here/firefox.lock"

name="rnl-viewer-local-$version-linux-x86_64"
work=$(mktemp -d "${TMPDIR:-/tmp}/rnl-package.XXXXXX")
trap 'rm -rf "$work"' EXIT
mkdir -p "$out"
out=$(cd "$out" && pwd)

lay() { # DIRECTORY: what both packages hold
  mkdir -p "$1"
  cp "$program" "$1/rnl-viewer-local"
  chmod 755 "$1/rnl-viewer-local"
  cp "$root/apps/local/README.md" "$1/README.md"
  cp "$root/LICENSE" "$1/LICENSE"
  # (the licences of the fonts and files that the page carries)
  cp -r "$root/LICENSES" "$1/LICENSES"
  cp "$here/get-firefox.sh" "$here/firefox.lock" "$1/"
  chmod 755 "$1/get-firefox.sh"
}

pack() { # DIRECTORY: a tar.gz with the same bytes whenever it is made from the same files
  tar -C "$work" --sort=name --owner=0 --group=0 --numeric-owner --mtime='2000-01-01 00:00:00 UTC' -cf - "$1" | gzip -9n > "$out/$1.tar.gz"
}

lay "$work/$name"
pack "$name"

if [ -n "$firefox" ]; then
  with="$name-with-firefox"
  lay "$work/$with"
  tar -xJf "$firefox" -C "$work/$with"
  [ -x "$work/$with/firefox/firefox" ] || { echo "package.sh: no firefox/firefox in $firefox" >&2; exit 1; }
  cat > "$work/$with/FIREFOX-NOTICE.txt" <<NOTICE
The folder "firefox" is Mozilla Firefox ESR $FIREFOX_VERSION for Linux x86_64, as Mozilla publishes it, unmodified:

  $FIREFOX_BASE/$FIREFOX_VERSION/$FIREFOX_PATH
  sha256 $FIREFOX_SHA256

It is Mozilla's software, under its own licenses (the Mozilla Public License 2.0 and others: see about:license in the
browser, and the files in the folder), and the names and logos Firefox and Mozilla are Mozilla's trademarks. Its source code is at
$FIREFOX_BASE/$FIREFOX_VERSION/source/ . It is here so that the program (RNL Viewer, local) has a browser that is new enough on a
computer whose own is not, and does not need to be installed: the program starts it in a profile of its own, and never touches
the user's own Firefox or its profile. Nothing of it is changed. To use the program with another Firefox, remove the folder.
NOTICE
  pack "$with"
fi

(cd "$out" && sha256sum "$name.tar.gz" ${firefox:+"$name-with-firefox.tar.gz"} > SHA256SUMS.txt)
ls -l "$out"
cat "$out/SHA256SUMS.txt"
