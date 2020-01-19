#!/bin/bash
TARGET=${TARGET-x86_64-unknown-linux-musl}

VER=release
if [[ "$1" == "--debug" ]]; then
    VER=debug
fi

cargo build --target $TARGET --$VER

if [[ "$VER" == "release" ]]; then
  strip target/x86_64-unknown-linux-musl/release/dragonslayer
fi
