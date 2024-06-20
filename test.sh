#!/bin/bash

mkdir logs
cargo b -r || exit 1
failed=()

for f in "./tests/rv64$1"*.bin; do
    echo -e "     \x1b[34;1mTesting\x1b[0m \`$(basename $f)\`"
    log="./logs/$(basename $f).log"

    timeout 5 ./target/release/rv64 "$f" --testing 2>&1 > $log && {
        rm $log &
    } || {
        failed+=("$(basename $f)")
    }
done

fails=${#failed[@]}

if [[ $fails == 0 ]]; then
    echo -e "\x1b[1;32mAll tests passed 🎉\x1b[0m"
else
    if [[ $fails == 1 ]]; then
        echo -e "\x1b[1;31m$fails failed test\x1b[0m"
    else
        echo -e "\x1b[1;31m$fails failed tests\x1b[0m"
    fi

    for fail in "${failed[@]}"; do
        echo "    $fail"
    done
fi

exit $fails
