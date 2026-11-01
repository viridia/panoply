#!/usr/bin/env bash
# Runs the verification recipe (AGENTS.md's "Verify" section) as one command, filtering routine
# output and only widening it for a step that actually failed.
#
# A warning counts as a failure here even though cargo's own exit code ignores it, because the tree
# is warning-free.
#
# Usage: scripts/verify.sh
#
# Runtime checks (the plans) are not part of it: they launch the game and take the focus.

set -uo pipefail
cd "$(dirname "$0")/.."

if [[ $# -gt 0 ]]; then
    echo "usage: $0" >&2
    exit 2
fi

failed=0
declare -a fail_names=()

# Runs one step, capturing combined output. Passes if the command exits 0 *and* prints no line
# starting with "warning"; on failure, prints the full output, since a filtered failure is useless
# for debugging.
run_step() {
    local name="$1"
    shift
    echo "== ${name} =="
    local output status warnings
    output=$("$@" 2>&1)
    status=$?
    warnings=$(printf '%s\n' "${output}" | grep -c '^warning' || true)

    if [[ ${status} -ne 0 ]]; then
        printf '%s\n' "${output}"
        echo "FAILED (exit ${status}): ${name}"
        failed=1
        fail_names+=("${name}")
    elif [[ ${warnings} -gt 0 ]]; then
        printf '%s\n' "${output}" | grep '^warning'
        echo "FAILED (warnings): ${name}"
        failed=1
        fail_names+=("${name} (warnings)")
    else
        echo "pass: ${name}"
    fi
    echo
}

# First because they cost milliseconds.
run_step "scripts/xref.py" python3 scripts/xref.py --quiet
run_step "scripts/devfmt.py --check --diff" python3 scripts/devfmt.py --check --diff
run_step "cargo clippy --workspace" cargo clippy --workspace

echo "=================================="
if [[ ${failed} -eq 0 ]]; then
    echo "All checks passed."
else
    echo "FAILED:"
    printf '  - %s\n' "${fail_names[@]}"
fi
exit ${failed}
