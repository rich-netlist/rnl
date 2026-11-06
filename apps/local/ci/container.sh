#!/bin/sh
# Run inside a container of an older enterprise Linux (centos:7, rockylinux:8, rockylinux:9), with
#   /pkg   the package with Firefox, unpacked: rnl-viewer-local and firefox/ beside it
#   /work  a netlist to open (top.sp)
# It prints what the system is and has, checks that the program is static and runs, installs the libraries that the official
# Firefox needs (and that a desktop has), and starts the program on the netlist as a user would, with no browser named: its
# launcher finds the Firefox beside it and starts it, headless, with Marionette on port 2828, for apps/local/ci/drive.mjs to look at
# from outside (host networking). It stays in the foreground until the container is stopped.
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

# (what Firefox needs of a desktop, which the program's README says: a stock minimal Rocky 8 has none of it, libX11-xcb among them)
$PM install -y gtk3 dbus-glib libXt libXtst libX11-xcb alsa-lib libXcomposite libXdamage libXrandr libXi libXcursor xz bzip2 which \
  dejavu-sans-fonts >/tmp/install.log 2>&1 || { tail -30 /tmp/install.log; exit 1; }

echo "== the static program"
cd /pkg
./rnl-viewer-local --version
# (ldd succeeds on a static program that is position independent, and says what it says: that the program is static is the
# build job's check, with file; this is for the record)
ldd ./rnl-viewer-local 2>&1 | head -3 | sed 's/^/ldd: /' || true

echo "== the Firefox of the package"
./firefox/firefox --version

echo "== the program, with its launcher"
mkdir -p /tmp/home
# (a container has no user namespaces for the content sandbox; Marionette is how the driver looks at the page)
export HOME=/tmp/home MOZ_HEADLESS=1 MOZ_MARIONETTE=1 MOZ_DISABLE_CONTENT_SANDBOX=1
unset XDG_RUNTIME_DIR XDG_CACHE_HOME XDG_CONFIG_HOME
exec ./rnl-viewer-local /work/top.sp
