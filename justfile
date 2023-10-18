set shell := ["cmd.exe", "/c"]

test-1: 
    cargo run blarggs/01-special.gb
test-2:
    cargo run blarggs/02-interrupts.gb
test-3: 
    cargo run blarggs/03-op-sp.gb
test-4:
    cargo run blarggs/04-op-r.gb
test-5:
    cargo run blarggs/05-op-rp.gb
test-6:
    cargo run blarggs/06-ld.gb
test-7:
    cargo run blarggs/07-jr.gb
test-8:
    cargo run blarggs/08-misc.gb
test-9:
    cargo run blarggs/09-op-r.gb
test-10:
    cargo run blarggs/10-bit.gb
test-11:
    cargo run blarggs/11-op-hl.gb
test-full:
    cargo run blarggs/cpu_instrs.gb