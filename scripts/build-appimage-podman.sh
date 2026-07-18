#!/usr/bin/env bash
# Build the Tauri AppImage inside Ubuntu 24.04 with Podman.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TOOLS_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
PROJECT_DIR="$(cd "${TOOLS_DIR}/.." && pwd)"
IMAGE_NAME="${IRIEBOOK_APPIMAGE_IMAGE:-localhost/iriebook-appimage-ubuntu24:latest}"
CONTAINERFILE="${SCRIPT_DIR}/Containerfile.appimage-ubuntu24"
REBUILD=false
PREPARE_UPDATE=true

usage() {
    cat <<'USAGE'
Usage: tools/scripts/build-appimage-podman.sh [OPTIONS]

Builds the Linux AppImage in an Ubuntu 24.04 Podman container so the artifact
targets Kubuntu/Ubuntu 24.04-era system libraries instead of Arch/CachyOS ones.

Options:
  --rebuild              Rebuild the Podman image before running the build.
  --no-prepare-update    Skip ./prepare-update.sh after the Tauri build.
  -h, --help             Show this help.

Environment:
  IRIEBOOK_APPIMAGE_IMAGE                 Podman image tag to use.
  TAURI_SIGNING_PRIVATE_KEY               Defaults to <project>/iriebook_update.key.
  TAURI_SIGNING_PRIVATE_KEY_PASSWORD      Required for signed updater artifacts.
USAGE
}

log() {
    printf '[irie-appimage] %s\n' "$1"
}

die() {
    printf '[irie-appimage] ERROR: %s\n' "$1" >&2
    exit 1
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --rebuild)
            REBUILD=true
            shift
            ;;
        --no-prepare-update)
            PREPARE_UPDATE=false
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            die "Unknown option: $1"
            ;;
    esac
done

command -v podman >/dev/null 2>&1 || die 'podman is not installed or not on PATH.'
[[ -f "${CONTAINERFILE}" ]] || die "Missing Containerfile: ${CONTAINERFILE}"

if [[ "${REBUILD}" == true ]] || ! podman image exists "${IMAGE_NAME}"; then
    log "Building ${IMAGE_NAME}."
    podman build -t "${IMAGE_NAME}" -f "${CONTAINERFILE}" "${TOOLS_DIR}"
else
    log "Using existing ${IMAGE_NAME}. Pass --rebuild to refresh it."
fi

SIGNING_KEY="$(realpath "${TAURI_SIGNING_PRIVATE_KEY:-${PROJECT_DIR}/iriebook_update.key}")"

[[ -f "${SIGNING_KEY}" ]] || die "Signing key not found: ${SIGNING_KEY}"
[[ -n "${TAURI_SIGNING_PRIVATE_KEY_PASSWORD:-}" ]] || die 'TAURI_SIGNING_PRIVATE_KEY_PASSWORD must be set in the environment.'

INNER_SIGNING_KEY='/run/secrets/iriebook_update.key'
INNER_COMMAND='cd tools/iriebook-tauri-ui && npm ci && npm run tauri build'
CARGO_CACHE="${TOOLS_DIR}/.podman-cache/cargo"
NPM_CACHE="${TOOLS_DIR}/.podman-cache/npm"
HOME_CACHE="${TOOLS_DIR}/.podman-cache/home"

mkdir -p "${CARGO_CACHE}" "${NPM_CACHE}" "${HOME_CACHE}"

if [[ "${PREPARE_UPDATE}" == true ]]; then
    INNER_COMMAND+=' && cd ../.. && ./prepare-update.sh'
fi

log "Building AppImage under Ubuntu 24.04 userland."
podman run --rm \
    --userns=keep-id \
    --security-opt label=disable \
    -e "CARGO_HOME=${CARGO_CACHE}" \
    -e "HOME=${HOME_CACHE}" \
    -e "NPM_CONFIG_CACHE=${NPM_CACHE}" \
    -e "RUSTUP_HOME=/usr/local/rustup" \
    -e "TAURI_SIGNING_PRIVATE_KEY=${INNER_SIGNING_KEY}" \
    -e TAURI_SIGNING_PRIVATE_KEY_PASSWORD \
    -v "${SIGNING_KEY}:${INNER_SIGNING_KEY}:ro" \
    -v "${PROJECT_DIR}:${PROJECT_DIR}" \
    -w "${PROJECT_DIR}" \
    "${IMAGE_NAME}" \
    bash -lc "${INNER_COMMAND}"

log 'AppImage build finished.'
