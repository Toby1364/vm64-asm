#image test test.png

mov r0 &3fea_0700
grapcpy r0 test 100 100 64 16

loop:
jmp loop
