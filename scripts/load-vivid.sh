#!/usr/bin/env bash
# SPDX-FileCopyrightText: Copyright 2026 Au-Zone Technologies
# SPDX-License-Identifier: Apache-2.0
#
# Load the kernel's virtual V4L2 drivers for tests/vivid.rs on a hosted Ubuntu
# runner: vivid (one single-planar and one multi-planar capture node) and
# vim2m (memory-to-memory). The modules ship in linux-modules-extra for the
# running kernel. When the archive has no such package (it has not caught up
# with the runner image) this warns and exits 0, and the device tests skip; a
# failure to install a package that exists is an error.
#
# Writes available=true|false to $GITHUB_OUTPUT when it is set.

set -euo pipefail

report() {
    if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
        echo "available=$1" >> "$GITHUB_OUTPUT"
    fi
}

uname -r
sudo apt-get update -q
pkg="linux-modules-extra-$(uname -r)"
if ! apt-cache show "$pkg" > /dev/null 2>&1; then
    echo "::warning::$pkg is not in the archive; vivid tests skipped"
    report false
    exit 0
fi
sudo apt-get install -y -q "$pkg"
sudo modprobe vivid n_devs=2 node_types=0x1,0x1 multiplanar=1,2
sudo modprobe vim2m
# udev creates and permissions the nodes after modprobe returns; wait for it,
# or it resets the modes changed below.
sudo udevadm settle
sudo chmod a+rw /dev/video* /dev/dma_heap/system
ls -l /dev/video* /dev/dma_heap/system
for d in /sys/class/video4linux/video*; do
    echo "$(basename "$d"): $(cat "$d/name")"
done
report true
