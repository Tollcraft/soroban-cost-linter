# syntax=docker/dockerfile:1
# ── soroban-cost-linter development container ──────────────────────────────
# Reads the nightly toolchain pin from `rust-toolchain` (the single source of
# truth) so there is no hardcoded copy of the nightly version in this file.
#
# Image size notes:
#   rustc-dev is the largest component (~500 MB uncompressed).  We use
#   `--profile minimal` at rustup install time (no docs), strip the cargo
#   registry cache and git checkouts after `cargo install`, and avoid
#   unnecessary apt packages with `--no-install-recommends`.
# ────────────────────────────────────────────────────────────────────────────

FROM ubuntu:22.04

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
    cargo install cargo-dylint dylint-link --version "^6.0.1" && \
    # ── reduce image size ────────────────────────────────────────────────
    rm -rf ${HOME}/.cargo/registry/cache \
           ${HOME}/.cargo/registry/src \
           ${HOME}/.cargo/git/checkouts && \
    rm /tmp/rust-toolchain

# ── runtime environment ─────────────────────────────────────────────────────
ENV PATH="${HOME}/.cargo/bin:${PATH}"
WORKDIR /workspace
