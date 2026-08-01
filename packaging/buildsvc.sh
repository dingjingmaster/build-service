#!/usr/bin/env bash
set -eu

ROOT=/usr/local/buildsvc
BIN="$ROOT/bin/buildsvc"
LIB="$ROOT/lib"

if [ ! -x "$BIN" ]; then
    echo "missing buildsvc binary: $BIN" >&2
    exit 1
fi

loader=""
for candidate in "$LIB"/ld-linux*.so*; do
    if [ -x "$candidate" ]; then
        loader="$candidate"
        break
    fi
done

if [ -z "$loader" ]; then
    echo "missing dynamic loader in: $LIB" >&2
    exit 1
fi

export LD_LIBRARY_PATH="$LIB${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
exec "$loader" --library-path "$LIB" "$BIN" "$@"
