db  &ff &ff &ff &00 &00 &00 &00 &00 &00 &ff &ff &ff \
    &ff &00 &00 &00 &00 &00 &00 &00 &00 &ff &00 &00 \
    &00 &00 &00 &00 &00 &00 &00 &00 &00 &00 &00 &00 \
    &ff &ff &ff &00 &00 &00 &00 &00 &00 &ff &ff &ff \
    &ff &ff &ff &00 &00 &00 &00 &00 &00 &ff &ff &ff \
    &ff &ff &ff &ff &ff &ff &ff &ff &ff &ff &ff &ff 

mov r0 &3fea_0700
mov r1 &3fea_0700
mov r2 15

mov r3 &960
mov r5 9
mov r6 1
mov r8 10

loop:

grapcpy r0 &4_b000 12 6

add r0 r0 r2

sub r4 r0 r1
div r4 r4 r3

jlg r4 r6 loop

mul r7 r3 r5
add r0 r0 r7

add r6 r6 r8

jmp loop