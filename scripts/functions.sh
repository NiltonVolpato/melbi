# This file must be sourced, not executed.

melbi() {
    RUST_LOG=debug cargo run --quiet --package melbi-cli -- "$@"
}

# Prevent running as a script; only allow sourcing.
if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    echo "This file must be sourced, not executed." >&2
    return 1 2>/dev/null
    exit 1
else
    echo "Functions sourced successfully."
fi
