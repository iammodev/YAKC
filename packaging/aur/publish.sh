#!/usr/bin/env bash
# Publishes the YAKC package to the AUR.
#
# One-time prerequisites:
#   1. Create an AUR account: https://aur.archlinux.org/register
#   2. Add your SSH *public* key under My Account → SSH Public Key
#      (e.g. the contents of ~/.ssh/id_ed25519.pub).
#
# Then run this script from the repo root or from packaging/aur/.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

echo "Cloning the AUR repo (ssh://aur@aur.archlinux.org/yakc.git)…"
git clone ssh://aur@aur.archlinux.org/yakc.git "$work/yakc"

cp "$here/PKGBUILD" "$here/.SRCINFO" "$work/yakc/"

cd "$work/yakc"
git add PKGBUILD .SRCINFO
git commit -m "Update to 2.0.0"
git push

echo "Done. Package is live: https://aur.archlinux.org/packages/yakc"
