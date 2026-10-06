run:
    cargo run -p twofip

# Run under gdb; on a crash, print every thread's backtrace (also saved to target/gdb-last-run.log).
debug:
    cargo build -p twofip
    gdb -q -batch -iex "set auto-load python-scripts off" -iex "set print thread-events off" \
        -ex "handle SIGPIPE nostop noprint" \
        -ex run \
        -ex "echo \n=== crashing thread ===\n" -ex bt \
        -ex "echo \n=== all threads ===\n" -ex "thread apply all bt 25" \
        --args target/debug/2fip 2>&1 | tee target/gdb-last-run.log

check:
    cargo check --workspace

test:
    cargo test --workspace

lint:
    cargo clippy --workspace -- -D warnings

fmt:
    cargo fmt --all

# Build an optimized binary (target/release/2fip).
release:
    cargo build --release -p twofip

# Install for the current user (~/.local): binary, launcher entry and icon.
install: release
    SOURCE_BIN=target/release/2fip \
    SOURCE_DESKTOP=res/io.github.chillalabs.TwoFip.desktop \
    SOURCE_ICON=res/icons/hicolor/scalable/apps/io.github.chillalabs.TwoFip.svg \
    res/install.sh

uninstall:
    res/install.sh --uninstall

# Package for another computer as dist/*.tar.gz (unpack there, run ./install.sh).
package: release
    #!/bin/sh
    set -eu
    version=$(cargo pkgid -p twofip | sed 's/.*[#@]//')
    name="2fip-$version-$(uname -m)"
    stage="dist/$name"
    rm -rf "$stage"
    install -Dm755 target/release/2fip "$stage/bin/2fip"
    install -Dm644 res/io.github.chillalabs.TwoFip.desktop "$stage/share/applications/io.github.chillalabs.TwoFip.desktop"
    install -Dm644 res/icons/hicolor/scalable/apps/io.github.chillalabs.TwoFip.svg "$stage/share/icons/hicolor/scalable/apps/io.github.chillalabs.TwoFip.svg"
    install -Dm755 res/install.sh "$stage/install.sh"
    tar -C dist -czf "dist/$name.tar.gz" "$name"
    rm -rf "$stage"
    echo "Created dist/$name.tar.gz"

# Build the Flatpak from this checkout and install it for the current user
# (needs: flatpak install flathub org.flatpak.Builder). Run it with
# `flatpak run io.github.chillalabs.TwoFip`.
flatpak:
    flatpak run org.flatpak.Builder --user --install --force-clean \
        --state-dir=.flatpak-builder flatpak/build flatpak/io.github.chillalabs.TwoFip.yml

# Re-create flatpak/cargo-sources.json after Cargo.lock changes (needs
# flatpak-cargo-generator.py from flatpak/flatpak-builder-tools).
flatpak-sources generator="flatpak-cargo-generator.py":
    python3 {{generator}} Cargo.lock -o flatpak/cargo-sources.json

# Build and add a signed release to the chillalabs Flatpak repository (a
# checkout of github.com/chillalabs/flatpak, served by GitHub Pages); then
# commit and push that checkout. Signed with the key in ~/.gnupg.
flatpak-publish site="../chillalabs-flatpak" key="D0A09A1D8E8EDB64":
    flatpak run org.flatpak.Builder --user --force-clean --default-branch=stable \
        --state-dir=.flatpak-builder flatpak/build flatpak/io.github.chillalabs.TwoFip.yml
    flatpak build-export --gpg-sign={{key}} {{site}}/repo flatpak/build stable
    flatpak build-update-repo --gpg-sign={{key}} --generate-static-deltas \
        --prune --prune-depth=5 --title="chillalabs" {{site}}/repo

# Package as dist/2fip-<version>-x86_64.AppImage: one file that runs on most
# distributions (glibc 2.39+, libxkbcommon). Downloads the official
# appimagetool from github.com/AppImage the first time.
appimage: release
    #!/bin/sh
    set -eu
    version=$(cargo pkgid -p twofip | sed 's/.*[#@]//')
    id=io.github.chillalabs.TwoFip
    tool="${XDG_CACHE_HOME:-$HOME/.cache}/2fip-build/appimagetool-x86_64.AppImage"
    if [ ! -x "$tool" ]; then
        mkdir -p "$(dirname "$tool")"
        curl -fsSL -o "$tool" https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage
        chmod +x "$tool"
    fi
    appdir=dist/2fip.AppDir
    rm -rf "$appdir"
    install -Dm755 target/release/2fip "$appdir/usr/bin/2fip"
    install -Dm644 res/$id.desktop "$appdir/usr/share/applications/$id.desktop"
    install -Dm644 res/icons/hicolor/scalable/apps/$id.svg "$appdir/usr/share/icons/hicolor/scalable/apps/$id.svg"
    install -Dm644 res/$id.metainfo.xml "$appdir/usr/share/metainfo/$id.appdata.xml"
    cp res/$id.desktop "$appdir/$id.desktop"
    cp res/icons/hicolor/scalable/apps/$id.svg "$appdir/$id.svg"
    ln -s $id.svg "$appdir/.DirIcon"
    printf '#!/bin/sh\nexec "$(dirname "$(readlink -f "$0")")/usr/bin/2fip" "$@"\n' > "$appdir/AppRun"
    chmod +x "$appdir/AppRun"
    ARCH=x86_64 APPIMAGE_EXTRACT_AND_RUN=1 "$tool" --no-appstream "$appdir" "dist/2fip-$version-x86_64.AppImage"
    rm -rf "$appdir"
    echo "Created dist/2fip-$version-x86_64.AppImage"
