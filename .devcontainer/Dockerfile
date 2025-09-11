FROM mcr.microsoft.com/devcontainers/base:ubuntu

# Install cc65 (6502 toolchain) and build essentials
RUN apt-get update \
  && apt-get install -y --no-install-recommends \
  build-essential curl ca-certificates git python3 unzip \
  && rm -rf /var/lib/apt/lists/*

# cc65 toolchain
RUN git clone --depth 1 https://github.com/cc65/cc65.git /tmp/cc65 \
  && make -C /tmp/cc65 \
  && make -C /tmp/cc65 install PREFIX=/usr/local \
  && rm -rf /tmp/cc65

# Install Rust via feature (layer kept minimal here)

ENV CARGO_TERM_COLOR=always

WORKDIR /workspaces/BASIC-M6502
