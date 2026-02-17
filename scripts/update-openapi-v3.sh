#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_PATH="${ROOT_DIR}/openapi/openapi-v3.json"

mkdir -p "${ROOT_DIR}/openapi"

curl -fsSL https://patch-api.conalog.com/openapi.json \
  | jq '
      .paths |= with_entries(select(.key | startswith("/api/v3/")))
      | .info.title = "patch-client (v3)"
    ' > "${OUT_PATH}"

echo "saved: ${OUT_PATH}"
echo "v3 path count: $(jq '.paths | length' "${OUT_PATH}")"
