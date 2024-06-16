#!/bin/bash

cargo b -r || exit 1
failed=()

for f in "./tests/"*.bin; do
    ./target/release/rv64 "$f" --testing || failed+=(" $f ($?)")
done

IFS=,
echo -e "\nFailed tests:${failed[*]}"
exit "${#failed[@]}"
