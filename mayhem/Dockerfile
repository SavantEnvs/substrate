# syntax=docker/dockerfile:1.7
#
# Commit-image Dockerfile for substrate (sp-arithmetic libFuzzer target).
# Builds on top of the org C/C++ base, adding a pinned Rust toolchain + cargo-fuzz,
# then runs mayhem/build.sh.

FROM ghcr.io/savantenvs/base:20260621

ARG COMMIT
ARG REPO_URL
ARG SOURCE_DATE_EPOCH
ARG SANITIZER_FLAGS

ARG RUST_CHANNEL=nightly-2025-05-14
ARG CARGO_FUZZ_VERSION=0.12.0

ENV RUSTUP_HOME=/opt/toolchains/rust/rustup \
    CARGO_HOME=/opt/toolchains/rust/cargo \
    PATH=/opt/toolchains/rust/cargo/bin:${PATH}

USER root
RUN install -d /opt/toolchains/rust \
 && curl --proto '=https' --tlsv1.2 -fsSL https://sh.rustup.rs -o /tmp/rustup-init.sh \
 && sh /tmp/rustup-init.sh -y --no-modify-path --profile minimal \
        --default-toolchain "${RUST_CHANNEL}" \
 && rustup component add rust-src --toolchain "${RUST_CHANNEL}" \
 && cargo install cargo-fuzz --version "${CARGO_FUZZ_VERSION}" --locked \
 && rm -f /tmp/rustup-init.sh \
 && chown -R 2000:2000 /opt/toolchains/rust

USER mayhem
WORKDIR /mayhem
COPY --chown=2000:2000 . /mayhem

# Generate the Cargo.lock for the fuzz workspace (mayhem/fuzz/) with fresh
# compatible deps. The fuzz workspace is standalone ([workspace] in its Cargo.toml)
# and resolves hashbrown 0.17.x which bundles ahash internally — no legacy
# ahash 0.7.6 / 0.8.3 (incompatible with nightly-2025-05-14) is pulled in.
# This lock file is baked into the image so build.sh (re-run offline by the PATCH
# tier) resolves from $CARGO_HOME cache without network access.
RUN cd /mayhem/mayhem/fuzz && cargo generate-lockfile

RUN bash mayhem/build.sh

RUN if [ -f mayhem/test.sh ]; then bash mayhem/test.sh; else echo "no mayhem/test.sh"; fi

LABEL org.opencontainers.image.source="${REPO_URL}" \
      org.opencontainers.image.revision="${COMMIT}" \
      rlenv.tier="commit" \
      rlenv.source_date_epoch="${SOURCE_DATE_EPOCH}" \
      rlenv.user="mayhem"

USER mayhem
WORKDIR /mayhem
