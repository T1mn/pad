#!/bin/sh
# PAD Desktop uses the filename `node` for the login SDK entrypoint.  When
# packaged with Bun, this keeps the Swift login bridge unchanged while using a
# self-contained runtime whose dependencies are macOS system libraries.
set -eu
SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
exec "$SCRIPT_DIR/bun" "$@"
