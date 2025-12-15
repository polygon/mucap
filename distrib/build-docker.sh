#!/usr/bin/env bash
set -e

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
cd $SCRIPT_DIR

rm -rf target || true
mkdir -p target

docker build -t mucap-deb-builder .
docker run --rm -v ${SCRIPT_DIR}/..:/mucap -v ./target:/mucap/target mucap-deb-builder bash -c "
  cd /mucap &&
  cargo xtask bundle mucap --release
"