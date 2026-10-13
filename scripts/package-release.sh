#!/usr/bin/env bash
# Package release binaries into reproducible archives, with checksums.
#
# Usage: SOURCE_DATE_EPOCH=<seconds> scripts/package-release.sh BUILDS OUT
#
# BUILDS holds one directory per target triple, named after it, containing
# that target's built `treemd` or `treemd.exe`: the layout download-artifact
# gives one artifact per build job. OUT receives, per target:
#
#   treemd-<target>.tar.gz      treemd-<target>, treemd.1 and LICENSE
#   treemd-<target>.exe.zip     treemd-<target>.exe and LICENSE (Windows)
#   treemd-<target>[.exe].sha256  hash of the binary inside the archive
#
# and SHA256SUMS, the hashes of the archives themselves.
#
# Every timestamp is SOURCE_DATE_EPOCH, owners are 0, and file order and
# modes are fixed, so the same binaries give byte-identical archives with the
# same versions of tar, gzip and zip.
# Needs GNU tar, GNU coreutils, gzip and Info-ZIP zip, as on ubuntu runners.
set -euo pipefail

if [ $# -ne 2 ]; then
    echo "usage: SOURCE_DATE_EPOCH=<seconds> $0 BUILDS OUT" >&2
    exit 2
fi
: "${SOURCE_DATE_EPOCH:?set SOURCE_DATE_EPOCH to the release commit time}"

builds=$(cd "$1" && pwd)
mkdir -p "$2"
out=$(cd "$2" && pwd)
root=$(cd "$(dirname "$0")/.." && pwd)

# zip records local time, and the archive member order below relies on a
# byte-wise sort.
export TZ=UTC LC_ALL=C

if ! tar --version 2>/dev/null | grep -q 'GNU tar'; then
    echo "GNU tar is required" >&2
    exit 1
fi

stage=$(mktemp -d "${TMPDIR:-/tmp}/treemd-package.XXXXXX")
trap 'rm -rf "$stage"' EXIT

# The man page is generated from the CLI definition, so it is the same for
# every target. Take it from the one build this Linux x86_64 host can run.
host_bin="$stage/treemd-host"
cp "$builds/x86_64-unknown-linux-gnu/treemd" "$host_bin"
chmod 0755 "$host_bin"
"$host_bin" --man-page > "$stage/treemd.1"
if ! grep -q '^\.TH treemd 1' "$stage/treemd.1"; then
    echo "treemd --man-page did not produce a man page" >&2
    exit 1
fi

archives=()
for dir in "$builds"/*/; do
    target=$(basename "$dir")
    work="$stage/$target"
    mkdir "$work"
    install -m 0644 "$root/LICENSE" "$work/LICENSE"

    if [ -f "$dir/treemd.exe" ]; then
        bin="treemd-$target.exe"
        archive="$bin.zip"
        install -m 0755 "$dir/treemd.exe" "$work/$bin"
        members=(LICENSE "$bin")
        touch -h -d "@$SOURCE_DATE_EPOCH" "${members[@]/#/$work/}"
        # -X drops uid/gid and extended timestamps, -D directory entries.
        (cd "$work" && zip -X -D -q -9 "$out/$archive" "${members[@]}")
    elif [ -f "$dir/treemd" ]; then
        bin="treemd-$target"
        archive="$bin.tar.gz"
        install -m 0755 "$dir/treemd" "$work/$bin"
        install -m 0644 "$stage/treemd.1" "$work/treemd.1"
        members=(LICENSE "$bin" treemd.1)
        (cd "$work" && tar --format=gnu --sort=name \
            --owner=0 --group=0 --numeric-owner \
            --mtime="@$SOURCE_DATE_EPOCH" \
            -cf - "${members[@]}") | gzip -9 -n > "$out/$archive"
    else
        echo "no treemd binary in $dir" >&2
        exit 1
    fi

    (cd "$work" && sha256sum -- "$bin") > "$out/$bin.sha256"
    archives+=("$archive")
done

if [ ${#archives[@]} -eq 0 ]; then
    echo "no builds found in $builds" >&2
    exit 1
fi
(cd "$out" && sha256sum -- "${archives[@]}") > "$out/SHA256SUMS"

echo "Packaged ${#archives[@]} targets into $out:"
cat "$out/SHA256SUMS"
