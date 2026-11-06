#!/bin/sh
# Run inside a container of an older enterprise Linux (centos:7, rockylinux:8, rockylinux:9), with /probe holding what the
# job downloaded: the static `rnl`, the sample files and the official Firefox tarball. It prints what the system is and
# has, checks that the static program runs, installs Firefox's runtime libraries, and starts the official Firefox
# headless with Marionette on port 2828, for scripts/linux-el-probe/drive.mjs to drive from outside (host networking).
set -eu

. /etc/os-release
echo "== $PRETTY_NAME"
echo "kernel of the host (a container shares it): $(uname -r)"
ldd --version 2>&1 | head -1

case "$ID" in
  centos)
    # CentOS 7 is out of the mirrors: its packages are on the vault
    sed -i -e 's|^mirrorlist=|#mirrorlist=|g' -e 's|^#baseurl=http://mirror.centos.org|baseurl=http://vault.centos.org|g' /etc/yum.repos.d/CentOS-*.repo
    PM="yum"
    ;;
  *)
    PM="dnf"
    ;;
esac

$PM install -y gtk3 dbus-glib libXt libXtst alsa-lib libXcomposite libXdamage libXrandr libXi libXcursor xz bzip2 which \
  dejavu-sans-fonts >/tmp/install.log 2>&1 || { tail -30 /tmp/install.log; exit 1; }

# the Firefox the system itself offers, for the record
if $PM install -y firefox >/tmp/firefox-system.log 2>&1; then
  echo "system Firefox: $(firefox --version 2>&1 | tail -1)"
else
  echo "system Firefox: none installable ($(tail -1 /tmp/firefox-system.log))"
fi

echo "== the static program"
/probe/rnl --version
if ldd /probe/rnl >/tmp/ldd.log 2>&1; then
  echo "ldd says: dynamic"; cat /tmp/ldd.log
else
  echo "ldd says: $(cat /tmp/ldd.log)"
fi
/probe/rnl check /probe/samples/cases/04-hierarchy.sp | tail -3

echo "== the official Firefox"
mkdir -p /opt
tar -xf /probe/firefox.tar.xz -C /opt
FF=/opt/firefox/firefox
"$FF" --version
mkdir -p /tmp/home /tmp/profile
export HOME=/tmp/home MOZ_HEADLESS=1 MOZ_DISABLE_CONTENT_SANDBOX=1
exec "$FF" --headless --marionette --no-remote --profile /tmp/profile about:blank
