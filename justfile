# justfile for treemd - Markdown tree viewer and navigator

# Default recipe - show available commands
default:
    @just --list

# Build the project in debug mode
build:
    cargo build

# Build the project in release mode with optimizations
release:
    cargo build --release

# Run the project with a test file
run FILE="README.md":
    cargo run -- {{FILE}}

# Run the release build with a test file
run-release FILE="README.md":
    ./target/release/treemd {{FILE}}

# Run with the link following test file
run-links:
    cargo run -- tests/docs/test_links.md

# Run all tests
test:
    cargo test --all-targets --all-features --locked

# Run tests without optional/default features
test-minimal:
    cargo test --all-targets --no-default-features --locked

# Run tests with output shown
test-verbose:
    cargo test -- --nocapture

# Check code without building
check:
    cargo check --all-targets --all-features --locked

# Run clippy for linting
lint:
    cargo clippy --all-targets --all-features --locked -- -D warnings

# Format code with rustfmt
fmt:
    cargo fmt

# Check if code is formatted
fmt-check:
    cargo fmt -- --check

# Clean build artifacts
clean:
    cargo clean

# Install the binary to ~/.cargo/bin
install:
    @echo "Installing treemd..."
    cargo install --path . --force
    @echo "✅ Installation complete! Run 'treemd --help' to get started."

# Uninstall the binary
uninstall:
    cargo uninstall treemd

# Update dependencies
update:
    cargo update

# Audit dependencies. The ignored quick-xml advisories are build-time-only in
# wayland-scanner, which parses bundled protocol XML rather than user input.
audit:
    cargo audit --ignore RUSTSEC-2026-0194 --ignore RUSTSEC-2026-0195

# Show outdated dependencies
outdated:
    cargo outdated

# Full CI check: format, lint, test, build
ci: fmt-check lint test test-minimal audit release
    @echo "✅ All CI checks passed!"

# Quick test of link following feature
test-links: install
    @echo "Testing link following feature..."
    @echo "1. Opening tests/docs/test_links.md"
    @echo "2. Press 'f' to enter link mode"
    @echo "3. Press 'Tab' to cycle links"
    @echo "4. Press 'Enter' to follow"
    @echo "5. Press 'b' to go back"
    @echo "6. Press '?' to see help"
    @echo ""
    treemd tests/docs/test_links.md

# Watch and rebuild on file changes (requires cargo-watch)
watch:
    cargo watch -x check -x test -x run

# Generate and open documentation
doc:
    cargo doc --open

# Show project statistics
stats:
    @echo "Lines of code:"
    @find src -name "*.rs" -exec wc -l {} + | tail -1
    @echo "\nDependencies:"
    @cargo tree --depth 1

# Create a new release build and show binary size
release-info: release
    @echo "Release binary:"
    @ls -lh target/release/treemd | awk '{print $5, $9}'
    @echo "\nStripped binary size:"
    @strip target/release/treemd
    @ls -lh target/release/treemd | awk '{print $5, $9}'

# Check that a release is safe to tag. A pushed tag is immutable: the repo has a
# ruleset on refs/tags/v* that blocks deletion and updates with no bypass, so
# everything below has to be right before the tag goes out, not after.
release-preflight VERSION:
    #!/usr/bin/env bash
    set -uo pipefail
    version="{{VERSION}}"
    version="${version#v}"
    fail=0

    if [ -n "$(git status --porcelain)" ]; then
        echo "working tree is dirty"
        fail=1
    fi

    branch=$(git rev-parse --abbrev-ref HEAD)
    if [ "$branch" != "main" ]; then
        echo "on branch '$branch', expected main"
        fail=1
    fi

    manifest=$(cargo metadata --no-deps --format-version 1 \
        | jq -r '.packages[] | select(.name == "treemd") | .version')
    if [ "$version" != "$manifest" ]; then
        echo "Cargo.toml is $manifest, expected $version"
        fail=1
    fi

    if ! grep -q "^## \[$version\]" CHANGELOG.md; then
        echo "CHANGELOG.md has no '## [$version]' section"
        fail=1
    fi

    if git ls-remote --exit-code --tags origin "v$version" >/dev/null 2>&1; then
        echo "v$version is already pushed, and tags are immutable. Cut the next patch instead."
        fail=1
    fi

    if [ "$fail" -ne 0 ]; then
        echo
        echo "preflight failed, do not tag"
        exit 1
    fi
    echo "ready to tag v$version"

# Cut and push a release tag, but only if the preflight passes.
release-tag VERSION: (release-preflight VERSION)
    git tag -a "v{{VERSION}}" -m "Release v{{VERSION}}"
    git push origin "v{{VERSION}}"
