#!/bin/sh
# Smoke-test a built treemd binary: it starts, reports its version, and lists
# the headings of a small document exactly.
#
# Usage: scripts/smoke-test.sh path/to/treemd
#
# Plain POSIX sh, so it also runs in an Alpine container, which is where a
# musl build has to work (#42), and under Git Bash on Windows.
set -eu

bin=$1
dir=$(mktemp -d "${TMPDIR:-/tmp}/treemd-smoke.XXXXXX")
trap 'rm -rf "$dir"' EXIT

version=$("$bin" --version)
echo "$version"
case $version in
    "treemd "*) ;;
    *)
        echo "smoke test: unexpected --version output: $version" >&2
        exit 1
        ;;
esac

printf '# Title\n\nText.\n\n## Section\n\n### Detail\n' > "$dir/doc.md"
printf '# Title\n## Section\n### Detail\n' > "$dir/expected"
"$bin" -l "$dir/doc.md" < /dev/null > "$dir/actual"
if ! cmp -s "$dir/expected" "$dir/actual"; then
    echo "smoke test: -l listed the wrong headings" >&2
    diff "$dir/expected" "$dir/actual" >&2 || true
    exit 1
fi

echo "smoke test passed: $bin"
