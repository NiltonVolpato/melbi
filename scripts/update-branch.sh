#!/usr/bin/env bash
set -euo pipefail

REMOTE_URL="https://github.com/NiltonVolpato/melbi"
BASE_BRANCH="main"
CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD)

if [[ "$CURRENT_BRANCH" == "$BASE_BRANCH" ]]; then
  echo "Already on $BASE_BRANCH; nothing to update." >&2
  exit 0
fi

echo "Fetching $BASE_BRANCH from $REMOTE_URL..." >&2
if ! git fetch "$REMOTE_URL" "$BASE_BRANCH"; then
  cat <<'MSG' >&2
Unable to contact the upstream repository. If you're working in the sandboxed
CI environment you may need to run this script from a machine with GitHub
access, then copy the updated tree back into the workspace.
MSG
  exit 1
fi

echo "Rebasing $CURRENT_BRANCH onto fetched $BASE_BRANCH" >&2
git rebase FETCH_HEAD
