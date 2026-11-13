#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_PATH="${ROOT_DIR}/openapi/openapi-v3.json"

mkdir -p "${ROOT_DIR}/openapi"

curl -fsSL https://patch-api.conalog.com/openapi.json \
  | jq '
      .paths |= with_entries(
        select(.key | startswith("/api/v3/") or startswith("/api/v4/"))
      )
      | .info.title = "patch-client (v3+v4)"
    ' > "${OUT_PATH}"

echo "saved: ${OUT_PATH}"
echo "v3 path count: $(jq '.paths | keys | map(select(startswith("/api/v3/"))) | length' "${OUT_PATH}")"
echo "v4 path count: $(jq '.paths | keys | map(select(startswith("/api/v4/"))) | length' "${OUT_PATH}")"
echo "operation count: $(jq '[.paths[] | keys[] | select(. != "parameters")] | length' "${OUT_PATH}")"
