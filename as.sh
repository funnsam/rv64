#!/bin/sh

riscv64-unknown-elf-as $1 -march=rv64ima -o program.o
riscv64-unknown-elf-objcopy program.o -O binary program.bin
