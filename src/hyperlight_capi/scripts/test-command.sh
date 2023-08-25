#!/bin/bash
set -euo pipefail
LD_LIBRARY_PATH=../../target/$1 ./bin/test_c.out 