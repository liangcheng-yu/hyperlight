#!/bin/bash
set -euo pipefail

LD_LIBRARY_PATH=../../target/$1 \
valgrind \
--leak-check=full \
--error-exitcode=1 \
--track-origins=yes \
-s \
--suppressions=./minimal.supp \
./bin/test_c.out \
--log-visible info \
--show-stderr
