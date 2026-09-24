run:
    cargo run -p pa2

# Run under gdb; on a crash, print every thread's backtrace (also saved to target/gdb-last-run.log).
debug:
    cargo build -p pa2
    gdb -q -batch -iex "set auto-load python-scripts off" -iex "set print thread-events off" \
        -ex "handle SIGPIPE nostop noprint" \
        -ex run \
        -ex "echo \n=== crashing thread ===\n" -ex bt \
        -ex "echo \n=== all threads ===\n" -ex "thread apply all bt 25" \
        --args target/debug/pa2 2>&1 | tee target/gdb-last-run.log

check:
    cargo check --workspace

test:
    cargo test --workspace

lint:
    cargo clippy --workspace -- -D warnings

fmt:
    cargo fmt --all

# Build an optimized binary (target/release/pa2).
release:
    cargo build --release -p pa2

# Install for the current user (~/.local): binary, launcher entry and icon.
install: release
    SOURCE_BIN=target/release/pa2 \
    SOURCE_DESKTOP=res/io.github.gonzaloism.pa2.desktop \
    SOURCE_ICON=res/icons/hicolor/scalable/apps/io.github.gonzaloism.pa2.svg \
    res/install.sh

uninstall:
    res/install.sh --uninstall

# Package for another computer as dist/*.tar.gz (unpack there, run ./install.sh).
package: release
    #!/bin/sh
    set -eu
    version=$(cargo pkgid -p pa2 | sed 's/.*[#@]//')
    name="pa2-$version-$(uname -m)"
    stage="dist/$name"
    rm -rf "$stage"
    install -Dm755 target/release/pa2 "$stage/bin/pa2"
    install -Dm644 res/io.github.gonzaloism.pa2.desktop "$stage/share/applications/io.github.gonzaloism.pa2.desktop"
    install -Dm644 res/icons/hicolor/scalable/apps/io.github.gonzaloism.pa2.svg "$stage/share/icons/hicolor/scalable/apps/io.github.gonzaloism.pa2.svg"
    install -Dm755 res/install.sh "$stage/install.sh"
    tar -C dist -czf "dist/$name.tar.gz" "$name"
    rm -rf "$stage"
    echo "Created dist/$name.tar.gz"
