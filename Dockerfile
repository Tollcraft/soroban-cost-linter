# syntax=docker/dockerfile:1
# ── soroban-cost-linter development container ──────────────────────────────
# Reads the nightly toolchain pin from `rust-toolchain` (the single source of
# truth) so there is no hardcoded copy of the nightly version in this file.
#
# The base image is `ubuntu:22.04` pinned by its multi-arch manifest digest so
# the image is reproducible regardless of what the `22.04` tag points at when
# it is built. The container does not run as root: a `dev` user with uid/gid
# 1000 and passwordless sudo is created below.
#
# Image size notes:
#   rustc-dev is the largest component (~500 MB uncompressed).  We use
#   `--profile minimal` at rustup install time (no docs), strip the cargo
#   registry cache and git checkouts after `cargo install`, and avoid
#   unnecessary apt packages with `--no-install-recommends`.
# ────────────────────────────────────────────────────────────────────────────

FROM ubuntu:22.04@sha256:b8b6ee6aa931ecd9d0d952abc34dc0e5f7c6a30c6bb71b079fe399fde0329c02

# Avoid interactive prompts during package installation
ENV DEBIAN_FRONTEND=noninteractive

# ── system dependencies ─────────────────────────────────────────────────────
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    build-essential \
    pkg-config \
    libssl-dev \
    ca-certificates \
    git \
    sudo \
    && rm -rf /var/lib/apt/lists/*

# ── user setup ──────────────────────────────────────────────────────────────
RUN groupadd --gid 1000 dev && \
    useradd --uid 1000 --gid 1000 -m -s /bin/bash dev && \
    echo "dev ALL=(root) NOPASSWD:ALL" > /etc/sudoers.d/dev && \
    chmod 0440 /etc/sudoers.d/dev

USER dev
ENV HOME=/home/dev

# ── toolchain pin (single source of truth: rust-toolchain) ──────────────────
COPY --chown=dev:dev rust-toolchain /tmp/rust-toolchain

RUN NIGHTLY=$(sed -n 's/^channel = "\(nightly-[0-9]\{4\}-[0-9]\{2\}-[0-9]\{2\}\)"$/\1/p' /tmp/rust-toolchain) && \
    if [ -z "${NIGHTLY}" ]; then echo "ERROR: could not parse nightly from rust-toolchain" >&2; exit 1; fi && \
    echo ">> Installing Rust toolchain: ${NIGHTLY}" && \
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
        sh -s -- -y --default-toolchain "${NIGHTLY}" --profile minimal && \
    . "$HOME/.cargo/env" && \
    rustup component add rustc-dev llvm-tools-preview rustfmt clippy && \
    cargo install cargo-dylint dylint-link --version "^6.0.1" --locked && \
    # ── reduce image size ────────────────────────────────────────────────
    rm -rf ${HOME}/.cargo/registry/cache \
           ${HOME}/.cargo/registry/src \
           ${HOME}/.cargo/git/checkouts && \
    rm /tmp/rust-toolchain

# ── build the linter ────────────────────────────────────────────────────────
# Copy the full source tree so we can build cargo-cost-lint and the dylint
# library inside the image.  The library must be linked with dylint-link so
# it is registered correctly by `cargo dylint`.
COPY --chown=dev:dev . /tmp/soroban-cost-linter

RUN . "$HOME/.cargo/env" && \
    cd /tmp/soroban-cost-linter && \
    # Build the cargo-cost-lint CLI binary
    cargo build --release --locked --manifest-path cargo-cost-lint/Cargo.toml && \
    cp target/release/cargo-cost-lint "$HOME/.cargo/bin/" && \
    # Build the soroban_cost_lints dylint library with the correct linker
    RUSTFLAGS="-C linker=dylint-link" cargo build --release --locked -p soroban_cost_lints && \
    mkdir -p "$HOME/.dylint-libs" && \
    cp target/release/libsoroban_cost_lints@* "$HOME/.dylint-libs/" && \
    # Clean up build tree to reduce image size
    cd / && rm -rf /tmp/soroban-cost-linter

# ── runtime environment ─────────────────────────────────────────────────────
ENV PATH="${HOME}/.cargo/bin:${PATH}"
ENV DYLINT_LIBRARY_PATH="${HOME}/.dylint-libs"
WORKDIR /workspace
