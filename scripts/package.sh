#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
test "$(uname -m)" = x86_64
test "$(uname -s)" = Linux
cargo build --release --locked
package=spool-v0.0.1-linux-x86_64
mkdir -p "dist/$package"
install -m755 target/release/spool "dist/$package/spool"
install -m644 README.md LICENSE docs/v0.0.1.md "dist/$package/"
tar -C dist -czf "dist/$package.tar.gz" "$package"
(cd dist && sha256sum "$package.tar.gz" > "$package.tar.gz.sha256")
printf 'Prepared %s\n' "dist/$package.tar.gz"
