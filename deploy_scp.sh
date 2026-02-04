#!/bin/sh
set -eu

# Usage: update the paths below, then run:
#   ./deploy_scp.sh [f|b|both]

FRONTEND_DIR="dist"
BACKEND_BIN="maida_control_server/target/x86_64-unknown-linux-gnu/release/maida_control_server"

REMOTE_HOST="cm"
REMOTE_FRONTEND_DIR="/opt/maida_control/front"
REMOTE_BACKEND_DIR="/opt/maida_control/server"

TARGET="${1:-both}"

case "${TARGET}" in
  f|b|both) ;;
  *)
    echo "Usage: $0 [f|b|both]" >&2
    exit 1
    ;;
esac

if [ "${TARGET}" = "f" ] || [ "${TARGET}" = "both" ]; then
  npm run build
  ssh "${REMOTE_HOST}" "mkdir -p '${REMOTE_FRONTEND_DIR}'"
  scp -r "${FRONTEND_DIR}/." "${REMOTE_HOST}:${REMOTE_FRONTEND_DIR}/"
fi

if [ "${TARGET}" = "b" ] || [ "${TARGET}" = "both" ]; then
  (cd maida_control_server && cargo build --release --target x86_64-unknown-linux-gnu)
  ssh "${REMOTE_HOST}" "mkdir -p '${REMOTE_BACKEND_DIR}'"
   ssh "${REMOTE_HOST}" "service maidacontrol stop"
   scp "${BACKEND_BIN}" "${REMOTE_HOST}:${REMOTE_BACKEND_DIR}/"
  ssh "${REMOTE_HOST}" "service maidacontrol start"
fi
