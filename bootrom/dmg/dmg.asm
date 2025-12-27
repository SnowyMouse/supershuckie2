# This is adapted from the disassembly @ ISSOtm / gb-bootroms
# https://codeberg.org/ISSOtm/gb-bootroms/src/branch/master

INCLUDE "hardware.inc"

SECTION "Boot ROM", ROM0[$000]
EntryPoint:
    ld sp, $FFFE
    ld a, $30
    ldh [rP1], a
    xor a
    ld hl, _VRAM + SIZEOF(VRAM) - 1
.clearVRAM
    ld [hld], a
    bit 7, h
    jr nz, .clearVRAM
    ld hl, rNR52
    ld c, LOW(rNR11)
    ld a, AUDENA_ON
    ld [hld], a
    ldh [c], a
    inc c
    ld a, (15 << 4) | AUDENV_DOWN | 3
    ldh [c], a
    ld [hld], a
    ld a, $77
    ld [hl], a
    ld a, %11_11_11_00
    ldh [rBGP], a
    jp Done

SECTION "Boot ROM Entry Point", ROM0[$100 - 4]
Done:
    ld a, 1
    ldh [rBANK], a
