#!/bin/bash

set -eoux pipefail

VALGRIND_VERSION=${VALGRIND_VERSION:-3.19.0}
VALGRIND_FNAME=valgrind-${VALGRIND_VERSION}
VALGRIND_TGZNAME=${VALGRIND_FNAME}.tar.bz2

curl -L -o ${VALGRIND_TGZNAME} https://sourceware.org/pub/valgrind/${VALGRIND_TGZNAME}
tar -xjf ${VALGRIND_TGZNAME}
cd ${VALGRIND_FNAME}
./configure --prefix=/usr/local
make
sudo make install

sudo apt-get install -y libc6-dbg

cd ..
rm -r ${VALGRIND_FNAME}
rm ${VALGRIND_TGZNAME}
