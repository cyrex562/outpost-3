# Reproducible build environment for the Harsh Realm Tauri desktop shell.
#
# Contains the Rust toolchain, Node.js, the Tauri CLI, and the Linux webview
# system libraries (webkit2gtk-4.1 + GTK3). The repo is bind-mounted at run
# time, so this image holds only tooling — rebuild it rarely.
#
# Build:  docker build -f docker/tauri-build.Dockerfile -t harsh-realm-tauri-build .
# Use:    docker run --rm -v "$PWD":/work -w /work harsh-realm-tauri-build <cmd>
FROM rust:1-bookworm

ENV DEBIAN_FRONTEND=noninteractive

# Tauri v2 Linux build dependencies + Node.js 20 (NodeSource).
RUN apt-get update && apt-get install -y --no-install-recommends \
        libwebkit2gtk-4.1-dev \
        libgtk-3-dev \
        libsoup-3.0-dev \
        libjavascriptcoregtk-4.1-dev \
        libayatana-appindicator3-dev \
        librsvg2-dev \
        libxdo-dev \
        libssl-dev \
        patchelf \
        file \
        build-essential \
        curl \
        wget \
        pkg-config \
    && curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && rm -rf /var/lib/apt/lists/*

# Tauri CLI v2 (prebuilt via npm — far faster than `cargo install tauri-cli`).
RUN npm install -g @tauri-apps/cli@^2

# Keep cargo registry/git caches on mountable volumes for fast re-runs.
ENV CARGO_HOME=/cargo
VOLUME ["/cargo"]

CMD ["bash"]
