SECTION "init", ROM0[$100]
LoopForever:
    halt
    nop
    jr LoopForever
