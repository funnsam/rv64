main:
    addi x1, x0, 1
    addi x2, x0, 0
    addi x3, x0, 1000

    .L0:
    add x1, x1, x2
    sub x2, x1, x2
    csrrw x0, x2, 0
    blt x2, x3, .L0

    .L1:
    j .L1
