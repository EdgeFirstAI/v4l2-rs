#!/usr/bin/env bash
# SPDX-FileCopyrightText: Copyright 2026 Au-Zone Technologies
# SPDX-License-Identifier: Apache-2.0
#
# ABI check for V4L2 code: record the ioctls a command makes, normalised so
# that two runs of the same code compare equal, and diff two recordings.
#
#   scripts/ioctl-trace.sh record <out.trace> -- <command> [args...]
#   scripts/ioctl-trace.sh diff <before.trace> <after.trace>
#
# `record` runs the command under `strace -f -e trace=ioctl` and keeps only
# the ioctl lines, with process IDs, descriptor numbers, pointers, buffer
# timestamps and frame sequence numbers replaced by placeholders. strace
# decodes V4L2 requests by name, so a struct with the wrong size shows up as
# an undecoded `_IOC(...)` request; `record` reports how many there are.
#
# `diff` exits 0 when the two recordings are identical.

set -euo pipefail

usage() {
    sed -n '8,9p' "$0" | sed 's/^#   /usage: /'
    exit 2
}

normalise() {
    sed -E \
        -e 's/^[0-9]+ +//' \
        -e 's/ioctl\([0-9]+, /ioctl(FD, /' \
        -e 's/0x[0-9a-f]{6,}/ADDR/g' \
        -e 's/timestamp=\{tv_sec=[0-9]+, tv_(usec|nsec)=[0-9]+\}/timestamp=T/g' \
        -e 's/(fd|request_fd)=[0-9]+/\1=N/g' \
        -e 's/sequence=[0-9]+/sequence=S/g' \
        "$1" | grep -E '^ioctl\(' | grep -v -e 'resumed>' -e '<unfinished' || true
}

case "${1:-}" in
    record)
        [ $# -ge 4 ] && [ "$3" = "--" ] || usage
        out=$2
        shift 3
        command -v strace > /dev/null || { echo "strace is not installed" >&2; exit 1; }
        raw=$(mktemp "${out}.raw.XXXXXX")
        trap 'rm -f "$raw"' EXIT
        status=0
        strace -f -qq -e trace=ioctl -e signal=none -o "$raw" "$@" || status=$?
        normalise "$raw" > "$out"
        calls=$(wc -l < "$out")
        undecoded=$(grep -c '_IOC(' "$out" || true)
        echo "$out: $calls ioctl calls, $undecoded undecoded requests (command exit $status)"
        exit "$status"
        ;;
    diff)
        [ $# -eq 3 ] || usage
        if diff -u "$2" "$3"; then
            echo "identical: $(wc -l < "$2") ioctl calls"
        else
            exit 1
        fi
        ;;
    *)
        usage
        ;;
esac
