
mov r0 &000000_000000
mov r3 &010000_010000
mov r5 &000100_000100
;mov rc &000010_000010

mov r2 &4000_0000

mov r6 &960
mov r4 6
mov r8 &3fea_0700
mov rb 3

reset:
mov r1 &3fea_0700
;add r0 r0 rc

loop:

mva r1 r0 6

add r1 r4 r1

sub r9 r1 r8
div r7 r9 r6
mul r0 r3 r7

mul ra r7 r6
sub ra r9 ra
div ra ra rb
mul ra ra r5
or  r0 r0 ra

jlg r1 r2 loop
jmp reset