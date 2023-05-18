#!/usr/bin/env bash

set -o errexit
set -o nounset
set -o pipefail

# set this to the release tag you want to download the test guests from
# See https://github.com/deislabs/hyperlight/releases
RELEASE_TAG="${RELEASE_TAG:=latest}"
mkdir -p src/tests/Guests/simpleguest/x64/debug/ && cd  src/tests/Guests/simpleguest/x64/debug/ && gh release download  ${RELEASE_TAG} -p 'simpleguest.exe' && cd -
mkdir -p src/tests/Guests/simpleguest/x64/release/ && cd  src/tests/Guests/simpleguest/x64/release/ && gh release download  ${RELEASE_TAG} -p 'simpleguest.exe' && cd -
mkdir -p src/tests/Guests/callbackguest/x64/debug/ && cd  src/tests/Guests/callbackguest/x64/debug/ && gh release download  ${RELEASE_TAG} -p 'callbackguest.exe' && cd -
mkdir -p src/tests/Guests/callbackguest/x64/release/ && cd  src/tests/Guests/callbackguest/x64/release/ && gh release download  ${RELEASE_TAG} -p 'callbackguest.exe' && cd -
