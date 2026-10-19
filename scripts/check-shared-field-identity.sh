#!/usr/bin/env bash
set -euo pipefail

# The integrated dependency graph must contain no akita-field, exactly one
# jolt-field, jolt-poly, jolt-transcript, and jolt-sumcheck package identity,
# and one shared Jolt Git commit.
#
# Structural check over `cargo metadata` package IDs: immune to `cargo tree`
# rendering (CARGO_TERM_COLOR=always colorizes the `(*)` dedup marker, which
# broke the previous text parse). The worst-case color environment is forced
# below as a permanent regression guard.
export CARGO_TERM_COLOR=always

check_workspace() {
  local label="$1"
  local manifest="$2"
  local metadata
  local akita_identities
  local crate
  local identities
  local count
  local source
  local field_source=""

  metadata="$(cargo metadata --format-version 1 --locked --manifest-path "$manifest")"

  akita_identities="$(jq -r '.packages[] | select(.name == "akita-field") | .id' <<<"$metadata" | sort -u)"
  if [[ -n "$akita_identities" ]]; then
    echo "error: $label dependency graph still contains akita-field" >&2
    printf '%s\n' "$akita_identities" >&2
    exit 1
  fi

  for crate in jolt-field jolt-poly jolt-transcript jolt-sumcheck; do
    identities="$(jq -r --arg name "$crate" '.packages[] | select(.name == $name) | .id' <<<"$metadata" | sort -u)"
    count="$(grep -c . <<<"$identities" || true)"
    if [[ "$count" -ne 1 ]]; then
      echo "error: expected exactly one $crate package identity in $label, found $count" >&2
      printf '%s\n' "$identities" >&2
      exit 1
    fi
    source="$(jq -r --arg name "$crate" '.packages[] | select(.name == $name) | .source' <<<"$metadata" | sort -u)"
    if [[ -z "$field_source" ]]; then
      field_source="$source"
    fi
    if [[ "$source" != "$field_source" || "$source" != git+https://github.com/a16z/jolt* ]]; then
      echo "error: $crate does not use the same Jolt Git source as jolt-field in $label" >&2
      printf '%s\n' "$identities" >&2
      exit 1
    fi
    printf 'shared %s identity (%s): %s\n' "$crate" "$label" "$identities"
  done
}

check_workspace root Cargo.toml
check_workspace fuzz fuzz/Cargo.toml
check_workspace recursion-profile profile/akita-recursion/Cargo.toml
