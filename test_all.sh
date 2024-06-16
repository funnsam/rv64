#!/bin/sh

for f in "./tests/"*; do
    cargo r -r -- "$f" | less
done
