#!/bin/bash
# prepare-update.sh
# Automates the Tauri updater workflow by extracting version/signature
# from the newest AppImage build and updating the update manifest.
#
# Required environment variables:
#   BUNDLE_DIR  - Path to the AppImage bundle directory
#   UPDATE_DIR  - Path to the update distribution directory
#   BASE_URL    - Base URL for the update server
#
# Usage:
#   BUNDLE_DIR=/path/to/bundle UPDATE_DIR=/path/to/updates BASE_URL=http://server ./prepare-update.sh

set -euo pipefail

# Validate required environment variables
: "${BUNDLE_DIR:?Environment variable BUNDLE_DIR is required}"
: "${UPDATE_DIR:?Environment variable UPDATE_DIR is required}"
: "${BASE_URL:?Environment variable BASE_URL is required}"

UPDATE_JSON="${UPDATE_DIR}/update.json"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Validate dependencies
check_dependencies() {
    if ! command -v jq &> /dev/null; then
        log_error "jq is required but not installed. Install it with: sudo apt install jq"
        exit 1
    fi
}

# Step 1: Find newest sig file
find_newest_sig() {
    SIG_FILE=$(ls -t "${BUNDLE_DIR}"/*.sig 2>/dev/null | head -1)

    if [[ -z "${SIG_FILE}" ]]; then
        log_error "No .sig file found in ${BUNDLE_DIR}"
        exit 1
    fi

    log_info "Found signature file: ${SIG_FILE}"
}

# Step 2: Extract version from filename
extract_version() {
    SIG_FILENAME=$(basename "${SIG_FILE}")
    VERSION=$(echo "${SIG_FILENAME}" | sed 's/iriebook-tauri-ui_\(.*\)_amd64.AppImage.sig/\1/')

    if [[ -z "${VERSION}" || "${VERSION}" == "${SIG_FILENAME}" ]]; then
        log_error "Failed to extract version from filename: ${SIG_FILENAME}"
        exit 1
    fi

    log_info "Extracted version: ${VERSION}"
}

# Step 3: Read signature content
read_signature() {
    SIGNATURE=$(cat "${SIG_FILE}")

    if [[ -z "${SIGNATURE}" ]]; then
        log_error "Signature file is empty: ${SIG_FILE}"
        exit 1
    fi

    log_info "Read signature (${#SIGNATURE} chars)"
}

# Step 4: Build corresponding AppImage filename and verify it exists
verify_appimage() {
    APPIMAGE_FILENAME="iriebook-tauri-ui_${VERSION}_amd64.AppImage"
    APPIMAGE_FILE="${BUNDLE_DIR}/${APPIMAGE_FILENAME}"

    if [[ ! -f "${APPIMAGE_FILE}" ]]; then
        log_error "AppImage file not found: ${APPIMAGE_FILE}"
        exit 1
    fi

    log_info "Found AppImage: ${APPIMAGE_FILE}"
}

# Step 5: Generate pub_date
generate_pub_date() {
    PUB_DATE=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
    log_info "Generated pub_date: ${PUB_DATE}"
}

# Step 6: Build download URL
build_download_url() {
    DOWNLOAD_URL="${BASE_URL}/${APPIMAGE_FILENAME}"
    log_info "Download URL: ${DOWNLOAD_URL}"
}

# Step 7: Update update.json using jq
update_manifest() {
    if [[ ! -f "${UPDATE_JSON}" ]]; then
        log_error "update.json not found: ${UPDATE_JSON}"
        exit 1
    fi

    jq --arg version "$VERSION" \
       --arg sig "$SIGNATURE" \
       --arg date "$PUB_DATE" \
       --arg url "$DOWNLOAD_URL" \
       '.version = $version | .pub_date = $date | .platforms["linux-x86_64"].signature = $sig | .platforms["linux-x86_64"].url = $url' \
       "${UPDATE_JSON}" > "${UPDATE_JSON}.tmp"

    if [[ $? -ne 0 ]]; then
        log_error "Failed to update JSON with jq"
        rm -f "${UPDATE_JSON}.tmp"
        exit 1
    fi

    mv "${UPDATE_JSON}.tmp" "${UPDATE_JSON}"
    log_info "Updated ${UPDATE_JSON}"
}

# Step 8: Clean irieupdates directory (preserve update.json)
clean_update_dir() {
    log_info "Cleaning ${UPDATE_DIR} (preserving update.json)..."
    find "${UPDATE_DIR}" -type f ! -name "update.json" -delete
}

# Step 9: Copy files to irieupdates
copy_files() {
    cp "${SIG_FILE}" "${UPDATE_DIR}/"
    log_info "Copied signature file to ${UPDATE_DIR}/"

    cp "${APPIMAGE_FILE}" "${UPDATE_DIR}/"
    log_info "Copied AppImage to ${UPDATE_DIR}/"
}

# Summary
print_summary() {
    echo ""
    echo "========================================="
    echo "Update preparation complete!"
    echo "========================================="
    echo "Version:   ${VERSION}"
    echo "Pub Date:  ${PUB_DATE}"
    echo "URL:       ${DOWNLOAD_URL}"
    echo ""
    echo "Files in ${UPDATE_DIR}:"
    ls -la "${UPDATE_DIR}"
    echo ""
    echo "update.json contents:"
    cat "${UPDATE_JSON}"
}

# Main execution
main() {
    log_info "Starting update preparation..."

    check_dependencies
    find_newest_sig
    extract_version
    read_signature
    verify_appimage
    generate_pub_date
    build_download_url
    update_manifest
    clean_update_dir
    copy_files
    print_summary
}

main "$@"
