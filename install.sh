#!/bin/sh
# Installs the latest `tix` release binary for macOS or Linux.
#   sh install.sh                      # installs to ~/.local/bin
#   TIX_INSTALL_DIR=/usr/local/bin sh install.sh
#   TIX_VERSION=v0.1.0 sh install.sh   # a specific release
set -eu

repo="alexbruf/tix"
dir="${TIX_INSTALL_DIR:-$HOME/.local/bin}"
version="${TIX_VERSION:-latest}"

case "$(uname -s)" in
  Darwin) os="apple-darwin" ;;
  Linux) os="unknown-linux-musl" ;;
  *) echo "tix: unsupported OS $(uname -s); download a binary from https://github.com/$repo/releases" >&2; exit 1 ;;
esac
case "$(uname -m)" in
  arm64 | aarch64) arch="aarch64" ;;
  x86_64 | amd64) arch="x86_64" ;;
  *) echo "tix: unsupported CPU $(uname -m)" >&2; exit 1 ;;
esac

asset="tix-$arch-$os.tar.gz"
if [ "$version" = "latest" ]; then
  url="https://github.com/$repo/releases/latest/download/$asset"
else
  url="https://github.com/$repo/releases/download/$version/$asset"
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
echo "Downloading $url"
if command -v curl >/dev/null 2>&1; then
  curl -fsSL "$url" -o "$tmp/$asset"
else
  wget -qO "$tmp/$asset" "$url"
fi
tar -xzf "$tmp/$asset" -C "$tmp"
mkdir -p "$dir"
install -m 755 "$tmp/tix" "$dir/tix"
echo "Installed $("$dir/tix" --version) to $dir/tix"

case ":$PATH:" in
  *":$dir:"*) ;;
  *) echo "Add $dir to your PATH, e.g.: echo 'export PATH=\"$dir:\$PATH\"' >> ~/.zshrc" ;;
esac
