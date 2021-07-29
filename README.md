# Rust-Arm11-Emulator

![RustArmEmulator](https://user-images.githubusercontent.com/44177991/127408055-2eeb0aee-5d17-49e4-ac66-2ddd6fa914bc.gif)

A reimplementation of part II of the first year C project in Rust, implemented in an object oriented style (CPU as a single class).

Usage:
```
> ./emulate path/to/binary
```

e.g basic factorial program
```
mov r0,#1
mov r1,#5
loop:
mul r2,r1,r0
mov r0,r2
sub r1,r1,#1
cmp r1,#0
bne loop
mov r3,#0x100
str r2,[r3]
```
```
Registers:
$0  :        120 (0x00000078)
$1  :          0 (0x00000000)
$2  :        120 (0x00000078)
$3  :        256 (0x00000100)
$4  :          0 (0x00000000)
$5  :          0 (0x00000000)
$6  :          0 (0x00000000)
$7  :          0 (0x00000000)
$8  :          0 (0x00000000)
$9  :          0 (0x00000000)
$10 :          0 (0x00000000)
$11 :          0 (0x00000000)
$12 :          0 (0x00000000)
PC  :         44 (0x0000002c)
CPSR: 1610612736 (0x60000000)
Non-zero memory:
0x00000000: 0x0100a0e3
0x00000004: 0x0510a0e3
0x00000008: 0x910002e0
0x0000000c: 0x0200a0e1
0x00000010: 0x011041e2
0x00000014: 0x000051e3
0x00000018: 0xfaffff1a
0x0000001c: 0x013ca0e3
0x00000020: 0x002083e5
0x00000100: 0x78000000
```
