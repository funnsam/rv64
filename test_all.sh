#!/bin/sh

set -e

for f in "./tests/"*.bin; do
    cargo r -r -- "$f" --testing
done
