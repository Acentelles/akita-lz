#!/usr/bin/env bash
# The `terminal-quotient` feature compiles the terminal quotient certificate in;
# `akita_config::TerminalQuotientConfig` selects it at runtime, and the selection
# alone chooses the proof-stream identifier and the grammar. Every Akita crate
# that prices, sends or checks the quotient asserts at compile time that its
# own feature agrees with akita-transcript's, so a partial feature graph must
# fail to compile, and the supported entry points must compile. No feature or
# hook may override the identifier independently of the selection.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

message='terminal-quotient must be enabled on every Akita crate in the build'
log="$(mktemp "${TMPDIR:-/tmp}/terminal-quotient-features.XXXXXX")"
trap 'rm -f "$log"' EXIT

accept() {
    echo "accept: cargo check $*"
    cargo check --quiet "$@"
}

reject() {
    echo "reject: cargo check $*"
    if cargo check --quiet "$@" >"$log" 2>&1; then
        echo "error: a partial terminal-quotient feature graph compiled: $*" >&2
        exit 1
    fi
    if ! grep -qF "$message" "$log"; then
        cat "$log" >&2
        echo "error: the partial graph failed for a reason other than the feature check: $*" >&2
        exit 1
    fi
}

accept -p akita-pcs --features terminal-quotient
accept -p akita-verifier --features terminal-quotient
accept -p akita-prover --features terminal-quotient
accept -p akita-config --features terminal-quotient
reject -p akita-params --features akita-transcript/terminal-quotient
reject -p akita-types --features akita-params/terminal-quotient
reject -p akita-config --features akita-transcript/terminal-quotient
reject -p akita-pcs --features akita-transcript/terminal-quotient
reject -p akita-pcs --features akita-verifier/terminal-quotient
reject -p akita-pcs --features akita-prover/terminal-quotient

# No identifier override: akita-transcript exposes no test-support feature, and
# no source in the workspace defines a thread-local version override.
echo "reject: cargo check -p akita-transcript --features test-support"
if cargo check --quiet -p akita-transcript --features test-support >"$log" 2>&1; then
    echo "error: akita-transcript accepts a test-support feature" >&2
    exit 1
fi
if ! grep -qF "test-support" "$log"; then
    cat "$log" >&2
    echo "error: the test-support check failed for another reason" >&2
    exit 1
fi
if grep -rnE "with_proof_stream_version|VERSION_OVERRIDE" crates; then
    echo "error: a proof-stream version override exists" >&2
    exit 1
fi
echo "terminal-quotient feature graphs are coherent."
