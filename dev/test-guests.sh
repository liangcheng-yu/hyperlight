#!/usr/bin/env bash

set -o errexit
set -o nounset
set -o pipefail

# set this to the release tag you want to download the test guests from
# See https://github.com/deislabs/hyperlight/releases
RELEASE_TAG="${RELEASE_TAG:=latest}"
mkdir -p src/tests/Guests/dummyguest/x64/debug/ && cd  src/tests/Guests/dummyguest/x64/debug/ && gh release download  ${RELEASE_TAG} -p 'dummyguest.exe' --clobber && cd -
mkdir -p src/tests/Guests/dummyguest/x64/release/ && cd  src/tests/Guests/dummyguest/x64/release/ && gh release download  ${RELEASE_TAG} -p 'dummyguest.exe' --clobber && cd -
mkdir -p src/tests/Guests/simpleguest/x64/debug/ && cd  src/tests/Guests/simpleguest/x64/debug/ && gh release download  ${RELEASE_TAG} -p 'simpleguest.exe' --clobber && cd -
mkdir -p src/tests/Guests/simpleguest/x64/release/ && cd  src/tests/Guests/simpleguest/x64/release/ && gh release download  ${RELEASE_TAG} -p 'simpleguest.exe' --clobber && cd -
mkdir -p src/tests/Guests/callbackguest/x64/debug/ && cd  src/tests/Guests/callbackguest/x64/debug/ && gh release download  ${RELEASE_TAG} -p 'callbackguest.exe' --clobber && cd -
mkdir -p src/tests/Guests/callbackguest/x64/release/ && cd  src/tests/Guests/callbackguest/x64/release/ && gh release download  ${RELEASE_TAG} -p 'callbackguest.exe' --clobber && cd -

mkdir -p src/tests/Hyperlight.Tests/bin/debug/net6.0
mkdir -p src/tests/Hyperlight.Tests/bin/release/net6.0

cp src/tests/Guests/dummyguest/x64/debug/dummyguest.exe src/tests/Hyperlight.Tests/bin/debug/net6.0
cp src/tests/Guests/dummyguest/x64/release/dummyguest.exe src/tests/Hyperlight.Tests/bin/release/net6.0
cp src/tests/Guests/simpleguest/x64/debug/simpleguest.exe src/tests/Hyperlight.Tests/bin/debug/net6.0
cp src/tests/Guests/simpleguest/x64/release/simpleguest.exe src/tests/Hyperlight.Tests/bin/release/net6.0
cp src/tests/Guests/callbackguest/x64/debug/callbackguest.exe src/tests/Hyperlight.Tests/bin/debug/net6.0
cp src/tests/Guests/callbackguest/x64/release/callbackguest.exe src/tests/Hyperlight.Tests/bin/release/net6.0
