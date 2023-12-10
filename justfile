set shell := ["cmd.exe", "/c"]

test TEST:
    cargo run blarggs/{{TEST}}

release:
    cargo build --release

run ROM:
    cargo run roms/{{ROM}}