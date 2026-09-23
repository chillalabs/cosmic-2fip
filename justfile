run:
    cargo run -p cosmic-commander

# Run under gdb; on a crash, print every thread's backtrace (also saved to target/gdb-last-run.log).
debug:
    cargo build -p cosmic-commander
    gdb -q -batch -iex "set auto-load python-scripts off" -iex "set print thread-events off" \
        -ex "handle SIGPIPE nostop noprint" \
        -ex run \
        -ex "echo \n=== crashing thread ===\n" -ex bt \
        -ex "echo \n=== all threads ===\n" -ex "thread apply all bt 25" \
        --args target/debug/cosmic-commander 2>&1 | tee target/gdb-last-run.log

check:
    cargo check --workspace

test:
    cargo test --workspace

lint:
    cargo clippy --workspace -- -D warnings

fmt:
    cargo fmt --all
