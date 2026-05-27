#!/usr/bin/env bash

set -euo pipefail

if [ ! -d gen/path ]
then
  mkdir -p gen/path
  nix develop .#ios -c sh -c 'ln -s "$(which clang)" "$PWD/gen/path/cc"'
fi

if [ -d gen/Payload ] || [ -f gen/App.ipa ]
then
    echo "Remove / backup the gen/Payload directory and gen/App.ipa file"
    exit 1;
fi

if [ ! -d gen/apple ]
then
    nix develop .#ios -c sh -c 'cargo tauri ios init'
    nix develop .#ios -c sed -i 's/CODE_SIGN_IDENTITY = "iPhone Developer"/CODE_SIGN_IDENTITY = ""/g' gen/apple/dos-gui.xcodeproj/project.pbxproj
fi

(cd frontend && nix develop .#ios -c pnpm install)
nix develop .#ios -c cargo tauri icon ../icon.png

# This will fail because it won't sign the output, but that's
# ok since we'll just generate the ipa from the archive manually
# as follows
nix develop .#ios -c sh -c 'unset SDKROOT; PATH="$PWD/gen/path:$PATH" OTHER_LDFLAGS="$NIX_LDFLAGS" cargo tauri ios build' || true

mkdir gen/Payload

cp -r "gen/apple/build/dos-gui_iOS.xcarchive/Products/Applications/Diff of Services.app" gen/Payload/
(cd gen && zip -r App.ipa Payload)
