#!/usr/bin/env bash

set -o errexit
set -o nounset
set -o pipefail

SCRIPT_DIR=$(realpath $(dirname "$0"))
REPO_DIR=$(realpath ${SCRIPT_DIR}/..)
pushd $REPO_DIR

# set this to the release tag you want to download the test guests from
# See https://github.com/deislabs/hyperlight/releases
RELEASE_TAG="${RELEASE_TAG:=latest}"

for guest in "callbackguest" "simpleguest" "dummyguest"; do
  mkdir -p src/tests/Guests/${guest}/x64/debug/ && cd src/tests/Guests/${guest}/x64/debug/ && gh release download  ${RELEASE_TAG} -p ${guest}.exe --clobber
  cd $REPO_DIR
  mkdir -p src/tests/Guests/${guest}/x64/release/ && cd src/tests/Guests/${guest}/x64/release/ && gh release download  ${RELEASE_TAG} -p ${guest}.exe --clobber
  cd $REPO_DIR

done

popd