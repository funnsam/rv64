#!/bin/bash

cargo b -r || exit 1
failed=()

for f in "./tests/"*.bin; do
    echo -e "\n\x1b[1mTesting:\x1b[0m $(basename $f)"
    timeout 5 ./target/release/rv64 "$f" --testing || failed+=("$(basename $f) ($?)")
done

fails=${#failed[@]}

if [[ $fails == 0 ]]; then
    echo -e "\n\x1b[1;32mAll tests passed 🎉\x1b[0m"
else
    if [[ $fails == 1 ]]; then
        echo -e "\n\x1b[1;31m$fails failed test\x1b[0m"
    else
        echo -e "\n\x1b[1;31m$fails failed tests\x1b[0m"
    fi

    for fail in "${failed[@]}"; do
        echo "    $fail"
    done
    exit $fails
fi
