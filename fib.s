main:
    li t1, 1
    li t2, 0
    li t3, 100000

    .L0:
    add t1, t1, t2
    sub t2, t1, t2
    csrrw zero, 0, t2
    blt t2, t3, .L0

    .L1:
    j .L1
