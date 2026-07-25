#!/usr/bin/env bash
# Install IrieBook system dependencies on macOS with Homebrew.

set -euo pipefail

BREW_PACKAGES=(
    git
    just
    node
    pandoc
    rustup-init
)

BREW_CASKS=(
    calibre
    mactex-no-gui
)

CALIBRE_CLI_TOOLS=(
    ebook-convert
    ebook-meta
    ebook-viewer
)

DRY_RUN=false
SKIP_TEX=false
SKIP_BUILD_TOOLS=false

usage() {
    cat <<'USAGE'
Usage: scripts/install-macos-deps.sh [OPTIONS]

Installs the Homebrew packages and casks needed to build and run IrieBook on macOS.
This intentionally does not install UI E2E test dependencies.

Options:
  --skip-tex             Do not install MacTeX. EPUB/AZW3 still work, PDF may not.
  --runtime-only         Install runtime ebook tools only: git, pandoc, calibre, MacTeX.
  --dry-run              Print what would be installed without changing the system.
  -h, --help             Show this help.

Core groups covered:
  - Runtime: git, pandoc, calibre command-line tools
  - PDF: MacTeX no-GUI, including xelatex, KOMA-Script, titlesec, and fonts
  - Build: node, rustup-init, just

Future Apple Silicon macOS DMG build command after npm dependencies are installed:
  cd iriebook-tauri-ui && npm run tauri -- build
USAGE
}

log() {
    printf '[irie-macos-deps] %s\n' "$1"
}

die() {
    printf '[irie-macos-deps] ERROR: %s\n' "$1" >&2
    exit 1
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

brew_prefix() {
    brew --prefix
}

brew_formula_installed() {
    brew list --formula "$1" >/dev/null 2>&1
}

brew_cask_installed() {
    brew list --cask "$1" >/dev/null 2>&1
}

install_brew_packages() {
    local packages=("$@")
    local package
    local missing=()

    for package in "${packages[@]}"; do
        if ! brew_formula_installed "$package"; then
            missing+=("$package")
        fi
    done

    if [[ "${#missing[@]}" -eq 0 ]]; then
        log 'All Homebrew formulae are already installed.'
        return
    fi

    log "Installing Homebrew formulae: ${missing[*]}"
    run_cmd brew install "${missing[@]}"
}

install_brew_casks() {
    local casks=("$@")
    local cask
    local missing=()

    for cask in "${casks[@]}"; do
        if ! brew_cask_installed "$cask"; then
            missing+=("$cask")
        fi
    done

    if [[ "${#missing[@]}" -eq 0 ]]; then
        log 'All Homebrew casks are already installed.'
        return
    fi

    log "Installing Homebrew casks: ${missing[*]}"
    run_cmd brew install --cask "${missing[@]}"
}

ensure_xcode_command_line_tools() {
    if xcode-select -p >/dev/null 2>&1; then
        return
    fi

    log 'Xcode Command Line Tools are not installed.'
    log 'macOS will open the installer prompt; rerun this script after it finishes.'
    run_cmd xcode-select --install
}

ensure_rust_default_toolchain() {
    if [[ "$SKIP_BUILD_TOOLS" == true ]]; then
        return
    fi

    if ! command -v rustup >/dev/null 2>&1; then
        log 'rustup is not on PATH yet. Open a new shell if Homebrew installed rustup-init for the first time.'
        log 'If needed, run: rustup-init -y && rustup default stable'
        return
    fi

    if rustup default >/dev/null 2>&1; then
        return
    fi

    log 'Configuring the stable Rust toolchain.'
    run_cmd rustup default stable
}

calibre_app_binary() {
    local tool=$1
    local app_path="/Applications/calibre.app/Contents/MacOS/$tool"

    if [[ -x "$app_path" ]]; then
        printf '%s' "$app_path"
    fi
}

ensure_calibre_cli_links() {
    local prefix
    local tool
    local app_binary

    prefix=$(brew_prefix)

    for tool in "${CALIBRE_CLI_TOOLS[@]}"; do
        if command -v "$tool" >/dev/null 2>&1; then
            continue
        fi

        app_binary=$(calibre_app_binary "$tool" || true)
        if [[ -z "$app_binary" ]]; then
            log "Calibre tool is missing and no app binary was found: $tool"
            continue
        fi

        log "Linking $tool into $prefix/bin."
        run_cmd ln -sf "$app_binary" "$prefix/bin/$tool"
    done
}

verify_commands() {
    local required=(git pandoc ebook-convert ebook-meta ebook-viewer)
    local build_required=(node npm rustup just)
    local pdf_required=(xelatex kpsewhich)
    local command
    local missing=()

    if [[ "$SKIP_BUILD_TOOLS" != true ]]; then
        required+=("${build_required[@]}")
    fi

    if [[ "$SKIP_TEX" != true ]]; then
        required+=("${pdf_required[@]}")
    fi

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

verify_tex_files() {
    local required_files=(scrbook.cls titlesec.sty)
    local file
    local missing=()

    [[ "$SKIP_TEX" == false ]] || return

    if [[ "$DRY_RUN" == true ]]; then
        log 'Dry run: skipping TeX file verification.'
        return
    fi

    for file in "${required_files[@]}"; do
        if ! kpsewhich "$file" >/dev/null 2>&1; then
            missing+=("$file")
        fi
    done

    if [[ "${#missing[@]}" -gt 0 ]]; then
        log "These TeX files are still missing: ${missing[*]}"
        log 'MacTeX may need a fresh shell or a repaired TeX Live install.'
        return
    fi

    log 'Required TeX files are available.'
}

while [[ "$#" -gt 0 ]]; do
    case "$1" in
        --skip-tex)
            SKIP_TEX=true
            ;;
        --runtime-only)
            SKIP_BUILD_TOOLS=true
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

if [[ "$(uname -s)" != "Darwin" ]]; then
    die 'This installer is for macOS and requires Homebrew.'
fi

if ! command -v brew >/dev/null 2>&1; then
    die 'Homebrew is required. Install it from https://brew.sh, then rerun this script.'
fi

packages=()
for package in "${BREW_PACKAGES[@]}"; do
    if [[ "$SKIP_BUILD_TOOLS" == true ]]; then
        case "$package" in
            node|rustup-init|just) continue ;;
        esac
    fi
    packages+=("$package")
done

casks=("${BREW_CASKS[@]}")
if [[ "$SKIP_TEX" == true ]]; then
    casks=(calibre)
fi

ensure_xcode_command_line_tools
install_brew_packages "${packages[@]}"
install_brew_casks "${casks[@]}"
ensure_calibre_cli_links
ensure_rust_default_toolchain
verify_commands
verify_tex_files

log 'Dependency install finished. Cool runnings.'
