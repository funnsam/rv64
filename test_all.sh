#!/bin/sh

set -e

for f in "./tests/"*; do
    cargo r -r -- "$f" --testing
done
