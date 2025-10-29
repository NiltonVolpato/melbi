#!/usr/bin/env bash
# Outputs workspace crate dependencies and features in YAML

VERSION=0.1.0

usage () {
  echo "dependencies [-hV]"
  echo
  echo "Options:"
  echo "  -h|--help      Print this help dialogue and exit"
  echo "  -V|--version   Print the current version and exit"
}

dependencies () {
  for opt in "${@}"; do
    case "${opt}" in
      -h|--help)
        usage
        return 0
        ;;
      -V|--version)
        echo "${VERSION}"
        return 0
        ;;
    esac
  done

  cargo metadata --format-version 1 | jq -r '
    def get_crate_names:
      [.workspace_members[] | split("#")[1] | split("@")[0]
       | select(test("^[a-zA-Z-]+"))];
    def filter_crates:
      .packages[] | select(.source == null);
    def get_external_deps($members):
      [.dependencies[] | select(.name | IN($members[]) | not)
       | .name] | sort;
    def get_workspace_deps($members):
      [.dependencies[] | select(.name | IN($members[])) | .name]
       | sort;
    get_crate_names as $members
    | [filter_crates
       | {name: .name,
          external_dependencies: get_external_deps($members),
          workspace_dependencies: get_workspace_deps($members),
          features: .features}] | map({key: .name, value: .}) | from_entries
  ' | yq -P
}

if [[ ${BASH_SOURCE[0]} != "$0" ]]; then
  export -f dependencies
else
  dependencies "${@}"
  exit 0
fi
