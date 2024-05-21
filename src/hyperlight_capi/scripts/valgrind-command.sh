#!/bin/bash
set -euo pipefail

GEN_SUPPRESSIONS=""
if [[ "${GEN_SUPPRESSIONS_ON:-1}" == "1" ]]; then
    GEN_SUPPRESSIONS="--gen-suppressions=all"
fi

LOG_FILE=""
if [[ "${LOG_FILE_ON:-0}" == "1" ]]; then
    LOG_FILE="--log-file=minimalraw.log"
fi

LD_LIBRARY_PATH=../../target/$1 \
valgrind ${GEN_SUPPRESSIONS} ${LOG_FILE} \
--leak-check=full \
--error-exitcode=1 \
--track-origins=yes \
--sim-hints=lax-ioctls \
-s \
--suppressions=./valgrind_suppressions/minimal.supp \
--suppressions=./valgrind_suppressions/hyperv_linux.supp \
--suppressions=./valgrind_suppressions/prometheus.supp \
./bin/test_c.out \
--log-visible info \
--show-stderr
