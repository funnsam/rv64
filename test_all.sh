#!/bin/sh

cargo b -r || exit 1
failed=()

for f in "./tests/"*.bin; do
    cargo r -r -- "$f" --testing || failed+=(" $f ($?)")
done

IFS=,
echo -e "\nFailed tests:${failed[*]}"
exit "${#failed[@]}"
