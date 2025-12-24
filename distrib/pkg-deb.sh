#!/usr/bin/env bash

# Make sure you run build-docker.sh first

set -e

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
cd $SCRIPT_DIR

VERSION=$(cat ${SCRIPT_DIR}/../VERSION)

mkdir -p release

fpm \
    --version ${VERSION} \
    --depends 'libasound2 >= 1.0.29' \
    --depends 'libc6 >= 2.35' \
    --depends 'libgl1' \
    --depends 'libx11-6' \
    --depends 'libx11-xcb1' \
    --depends 'libxcb-icccm4 >= 0.4.2' \
    --depends 'libxcb1 >= 1.12' \
    --depends 'libxcb1 >= 1.6' \
    -t deb -p release/mucap-${VERSION}-amd64.deb