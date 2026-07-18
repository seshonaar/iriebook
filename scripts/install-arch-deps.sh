#!/usr/bin/env bash
# Install IrieBook system dependencies on CachyOS / Arch Linux.

set -euo pipefail

PACMAN_PACKAGES=(
    base-devel
    appmenu-gtk-module
    calibre
    curl
    file
    fuse2
    git
    gtk3
    jq
    just
    libayatana-appindicator
    librsvg
    nodejs
    npm
    openssl
    pandoc-cli
    pkgconf
    rustup
    texlive-fontsextra
    texlive-fontsrecommended
    texlive-latex
    texlive-latexrecommended
    texlive-xetex
    webkit2gtk-4.1
    wget
)

E2E_PACMAN_PACKAGES=(
    xdotool
)

E2E_AUR_PACKAGES=(
    webkit2gtk-driver
)

INSTALL_E2E=false
INSTALL_TAURI_DRIVER=false
DRY_RUN=false
NO_AUR=false

usage() {
    cat <<'USAGE'
Usage: scripts/install-arch-deps.sh [OPTIONS]

Installs the system packages needed to build and run IrieBook on CachyOS / Arch.

Options:
  --with-e2e             Install optional UI E2E test dependencies.
  --with-tauri-driver    Install tauri-driver with cargo install --locked.
                         Implied by --with-e2e.
  --no-aur               Do not use yay/paru for optional AUR packages.
  --dry-run              Print what would be installed without changing the system.
  -h, --help             Show this help.

Core groups covered:
  - Build/dev: base-devel, rustup, nodejs, npm, pkgconf, openssl
  - Runtime: git, calibre, pandoc-cli
  - PDF: texlive-xetex plus recommended/extra TeX fonts for EB Garamond
  - Tauri Linux: webkit2gtk-4.1, libayatana-appindicator, librsvg
USAGE
}

log() {
    printf '[irie-deps] %s\n' "$1"
}

die() {
    printf '[irie-deps] ERROR: %s\n' "$1" >&2
    exit 1
}

aur_helper() {
    if command -v paru >/dev/null 2>&1; then
        printf 'paru'
    elif command -v yay >/dev/null 2>&1; then
        printf 'yay'
    fi
}

pacman_target_installed() {
    local target=$1
    local member
    local group_members=()

    if pacman -Qq "$target" >/dev/null 2>&1; then
        return 0
    fi

    mapfile -t group_members < <(pacman -Sgq "$target" 2>/dev/null | sort -u)
    if [[ "${#group_members[@]}" -eq 0 ]]; then
        return 1
    fi

    for member in "${group_members[@]}"; do
        if ! pacman -Qq "$member" >/dev/null 2>&1; then
            return 1
        fi
    done

    return 0
}

missing_pacman_packages() {
    local package

    for package in "$@"; do
        if ! pacman_target_installed "$package"; then
            printf '%s\n' "$package"
        fi
    done
}

run_cmd() {
    if [[ "$DRY_RUN" == true ]]; then
        printf '+ '
        printf '%q ' "$@"
        printf '\n'
    else
        "$@"
    fi
}

install_pacman_packages() {
    local packages=("$@")
    local missing=()

    mapfile -t missing < <(missing_pacman_packages "${packages[@]}")

    if [[ "${#missing[@]}" -eq 0 ]]; then
        log 'All pacman packages are already installed.'
        return
    fi

    log "Installing pacman packages: ${missing[*]}"
    run_cmd sudo pacman -S --needed "${missing[@]}"
}

install_aur_packages() {
    local helper
    local package
    local missing=()

    [[ "$NO_AUR" == false ]] || return 0

    helper=$(aur_helper || true)
    if [[ -z "$helper" ]]; then
        log 'No AUR helper detected (looked for paru, yay). Skipping optional AUR packages.'
        log "Install manually if needed: $*"
        return 0
    fi

    for package in "$@"; do
        if ! pacman -Qq "$package" >/dev/null 2>&1; then
            missing+=("$package")
        fi
    done

    if [[ "${#missing[@]}" -eq 0 ]]; then
        log 'All optional AUR packages are already installed.'
        return
    fi

    log "Installing optional AUR packages with $helper: ${missing[*]}"
    if ! run_cmd "$helper" -S --needed "${missing[@]}"; then
        log "Optional AUR install failed. Continue manually if you need these packages: ${missing[*]}"
    fi
}

ensure_rust_default_toolchain() {
    if ! command -v rustup >/dev/null 2>&1; then
        log 'rustup is not on PATH yet. Open a new shell if pacman installed it for the first time.'
        return
    fi

    if rustup default >/dev/null 2>&1; then
        return
    fi

    log 'Configuring the stable Rust toolchain.'
    run_cmd rustup default stable
}

install_tauri_driver() {
    if [[ "$INSTALL_TAURI_DRIVER" != true ]]; then
        return
    fi

    if command -v tauri-driver >/dev/null 2>&1; then
        log 'tauri-driver is already installed.'
        return
    fi

    if ! command -v cargo >/dev/null 2>&1; then
        log 'cargo is not on PATH yet. Open a new shell, then run: cargo install tauri-driver --locked'
        return
    fi

    log 'Installing tauri-driver with cargo.'
    run_cmd cargo install tauri-driver --locked
}

verify_commands() {
    local required=(git node npm rustup pandoc ebook-convert ebook-meta ebook-viewer)
    local command
    local missing=()

    if [[ "$DRY_RUN" == true ]]; then
        log 'Dry run: skipping command verification.'
        return
    fi

    for command in "${required[@]}"; do
        if ! command -v "$command" >/dev/null 2>&1; then
            missing+=("$command")
        fi
    done

    if [[ "${#missing[@]}" -gt 0 ]]; then
        log "These commands are still missing from PATH: ${missing[*]}"
        log 'If packages were just installed, open a new shell and rerun this script.'
        return
    fi

    log 'Core commands are available.'
}

verify_e2e_commands() {
    local optional=(tauri-driver WebKitWebDriver)
    local command
    local missing=()

    [[ "$INSTALL_E2E" == true ]] || return

    if [[ "$DRY_RUN" == true ]]; then
        log 'Dry run: skipping optional E2E command verification.'
        return
    fi

    for command in "${optional[@]}"; do
        if ! command -v "$command" >/dev/null 2>&1; then
            missing+=("$command")
        fi
    done

    if [[ "${#missing[@]}" -gt 0 ]]; then
        log "Optional E2E commands are still missing from PATH: ${missing[*]}"
        log 'UI E2E tests may need distro-specific WebKit WebDriver packaging.'
        return
    fi

    log 'Optional E2E commands are available.'
}

while [[ "$#" -gt 0 ]]; do
    case "$1" in
        --with-e2e)
            INSTALL_E2E=true
            INSTALL_TAURI_DRIVER=true
            ;;
        --with-tauri-driver)
            INSTALL_TAURI_DRIVER=true
            ;;
        --no-aur)
            NO_AUR=true
            ;;
        --dry-run)
            DRY_RUN=true
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            die "Unknown option: $1"
            ;;
    esac
    shift
done

if ! command -v pacman >/dev/null 2>&1; then
    die 'This installer is for CachyOS / Arch Linux and requires pacman.'
fi

packages=("${PACMAN_PACKAGES[@]}")
if [[ "$INSTALL_E2E" == true ]]; then
    packages+=("${E2E_PACMAN_PACKAGES[@]}")
fi

install_pacman_packages "${packages[@]}"

if [[ "$INSTALL_E2E" == true ]]; then
    install_aur_packages "${E2E_AUR_PACKAGES[@]}"
fi

ensure_rust_default_toolchain
install_tauri_driver
verify_commands
verify_e2e_commands

log 'Dependency install finished. Cool runnings.'
