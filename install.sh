#!/bin/bash
set -euo pipefail

REPO="igmarin/agent-mcp-runtime"
BINARY_NAME="agent-mcp-runtime"
DEFAULT_INSTALL_DIR="$HOME/.local/bin"

info() { printf "\033[1;34m==>\033[0m %s\n" "$1"; }
success() { printf "\033[1;32m==>\033[0m %s\n" "$1"; }
error() { printf "\033[1;31mError:\033[0m %s\n" "$1" >&2; exit 1; }

detect_platform() {
  local os arch

  os="$(uname -s)"
  arch="$(uname -m)"

  case "$os" in
    Darwin) os="apple-darwin" ;;
    Linux)  os="unknown-linux-gnu" ;;
    MINGW*|MSYS*|CYGWIN*) os="pc-windows-msvc" ;;
    *) error "Unsupported operating system: $os" ;;
  esac

  case "$arch" in
    x86_64|amd64) arch="x86_64" ;;
    arm64|aarch64) arch="aarch64" ;;
    *) error "Unsupported architecture: $arch" ;;
  esac

  # Windows binaries have .exe extension
  if [ "$os" = "pc-windows-msvc" ]; then
    ARTIFACT="${BINARY_NAME}-${arch}-${os}.exe"
  else
    ARTIFACT="${BINARY_NAME}-${arch}-${os}"
  fi

  PLATFORM="${arch}-${os}"
}

fetch_latest_version() {
  info "Fetching latest release..."
  local url="https://api.github.com/repos/${REPO}/releases/latest"
  local response

  if command -v curl >/dev/null 2>&1; then
    response="$(curl -fsSL "$url" 2>/dev/null)" || error "Failed to fetch latest release. Check your internet connection."
  elif command -v wget >/dev/null 2>&1; then
    response="$(wget -qO- "$url" 2>/dev/null)" || error "Failed to fetch latest release. Check your internet connection."
  else
    error "Neither curl nor wget found. Please install one of them."
  fi

  VERSION="$(echo "$response" | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": *"//;s/".*//')"
  [ -n "$VERSION" ] || error "Could not determine latest version. No releases found at https://github.com/${REPO}/releases"
}

download_binary() {
  local url="https://github.com/${REPO}/releases/download/${VERSION}/${ARTIFACT}"
  local tmpdir

  tmpdir="$(mktemp -d)"
  trap 'rm -rf "$tmpdir"' EXIT

  info "Downloading ${BINARY_NAME} ${VERSION} for ${PLATFORM}..."

  if command -v curl >/dev/null 2>&1; then
    curl -fsSL -o "${tmpdir}/${BINARY_NAME}" "$url" || error "Download failed. Binary may not be available for ${PLATFORM}.\nURL: ${url}"
  elif command -v wget >/dev/null 2>&1; then
    wget -qO "${tmpdir}/${BINARY_NAME}" "$url" || error "Download failed. Binary may not be available for ${PLATFORM}.\nURL: ${url}"
  fi

  DOWNLOAD_PATH="${tmpdir}/${BINARY_NAME}"
}

install_binary() {
  local install_dir="$DEFAULT_INSTALL_DIR"

  # Offer /usr/local/bin if user prefers
  if [ -t 0 ]; then
    printf "\nInstall to:\n"
    printf "  1) %s (default)\n" "$DEFAULT_INSTALL_DIR"
    printf "  2) /usr/local/bin (requires sudo)\n"
    printf "Choose [1/2]: "
    read -r choice
    case "$choice" in
      2) install_dir="/usr/local/bin" ;;
      *) install_dir="$DEFAULT_INSTALL_DIR" ;;
    esac
  fi

  mkdir -p "$install_dir"

  if [ "$install_dir" = "/usr/local/bin" ]; then
    info "Installing to ${install_dir} (requires sudo)..."
    sudo install -m 755 "$DOWNLOAD_PATH" "${install_dir}/${BINARY_NAME}"
  else
    info "Installing to ${install_dir}..."
    install -m 755 "$DOWNLOAD_PATH" "${install_dir}/${BINARY_NAME}"
  fi

  success "Installed ${BINARY_NAME} to ${install_dir}/${BINARY_NAME}"
}

ensure_path() {
  local install_dir="$1"

  # Skip PATH check for /usr/local/bin (usually already in PATH)
  if [ "$install_dir" = "/usr/local/bin" ]; then
    return
  fi

  # Check if install_dir is already in PATH
  case ":${PATH}:" in
    *":${install_dir}:"*) return ;;
  esac

  info "${install_dir} is not in your PATH. Adding it now..."

  local shell_config=""
  local export_line="export PATH=\"${install_dir}:\$PATH\""

  # Detect shell and add to appropriate config
  if [ -n "${BASH_VERSION:-}" ] || [ "$(basename "${SHELL:-}")" = "bash" ]; then
    shell_config="$HOME/.bashrc"
  fi

  if [ "$(basename "${SHELL:-}")" = "zsh" ] || [ -f "$HOME/.zshrc" ]; then
    shell_config="$HOME/.zshrc"
  fi

  if [ "$(basename "${SHELL:-}")" = "fish" ] || [ -f "$HOME/.config/fish/config.fish" ]; then
    local fish_config="$HOME/.config/fish/config.fish"
    mkdir -p "$(dirname "$fish_config")"
    local fish_line="set -gx PATH ${install_dir} \$PATH"
    if ! grep -qF "$fish_line" "$fish_config" 2>/dev/null; then
      echo "$fish_line" >> "$fish_config"
      info "Added PATH entry to ${fish_config}"
    fi
    return
  fi

  if [ -n "$shell_config" ]; then
    if ! grep -qF "$export_line" "$shell_config" 2>/dev/null; then
      echo "$export_line" >> "$shell_config"
      info "Added PATH entry to ${shell_config}"
    fi
  else
    printf "\n  Add this to your shell config:\n"
    printf "    %s\n\n" "$export_line"
  fi
}

main() {
  info "Installing ${BINARY_NAME}..."
  echo

  detect_platform
  fetch_latest_version
  download_binary
  install_binary

  # Determine install dir used (re-check choice logic)
  local install_dir="$DEFAULT_INSTALL_DIR"
  if command -v "$BINARY_NAME" >/dev/null 2>&1; then
    local bin_path
    bin_path="$(command -v "$BINARY_NAME")"
    install_dir="$(dirname "$bin_path")"
  fi

  ensure_path "$install_dir"

  echo
  success "${BINARY_NAME} ${VERSION} installed successfully!"
  echo
  printf "  Verify installation:\n"
  printf "    %s --version\n" "$BINARY_NAME"
  echo
  printf "  Get started:\n"
  printf "    %s --help\n" "$BINARY_NAME"
  echo

  # Remind user to reload shell if PATH was modified
  if ! command -v "$BINARY_NAME" >/dev/null 2>&1; then
    printf "  \033[1;33mNote:\033[0m Restart your shell or run:\n"
    printf "    export PATH=\"%s:\$PATH\"\n\n" "$DEFAULT_INSTALL_DIR"
  fi
}

main
