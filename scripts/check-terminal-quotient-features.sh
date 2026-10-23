#!/usr/bin/env bash
# The `terminal-quotient` feature changes the proof-stream identifier in
# akita-transcript and the grammar its producers and consumers speak. Every
# Akita crate that prices, sends or checks the quotient asserts at compile time
# that its own feature agrees with the identifier, so a partial feature graph
# must fail to compile, and the supported entry points must compile.
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
reject -p akita-params --features akita-transcript/terminal-quotient
reject -p akita-types --features akita-params/terminal-quotient
reject -p akita-pcs --features akita-transcript/terminal-quotient
reject -p akita-pcs --features akita-verifier/terminal-quotient
reject -p akita-pcs --features akita-prover/terminal-quotient
echo "terminal-quotient feature graphs are coherent."
