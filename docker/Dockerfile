# syntax=docker/dockerfile:1
# Local-dev images for strike-hub and the headless match server.
# The playable client stays on the host (it needs a native GPU window).

FROM rust:bookworm AS toolchain
WORKDIR /src
RUN apt-get update && apt-get install -y --no-install-recommends \
        clang cmake pkg-config \
        libasound2-dev libudev-dev libwayland-dev libxkbcommon-dev \
    && rm -rf /var/lib/apt/lists/*
COPY rust-toolchain.toml .
RUN rustup show

FROM toolchain AS hub-build
COPY Cargo.toml Cargo.lock ./
COPY crates/strike-hub ./crates/strike-hub
COPY src ./src
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --locked --release -p strike-hub \
    && mkdir -p /out \
    && cp /src/target/release/strike-hub /out/strike-hub

FROM debian:bookworm-slim AS hub
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=hub-build /out/strike-hub /app/strike-hub
ENV STRIKE_HUB_DB=/data/hub.db \
    STRIKE_HUB_LISTEN=0.0.0.0:7777 \
    STRIKE_SERVER_KEY=dev \
    STRIKE_SERVER_HOST=127.0.0.1
EXPOSE 7777
VOLUME /data
HEALTHCHECK --interval=5s --timeout=3s --start-period=5s --retries=12 \
    CMD curl -fsS http://127.0.0.1:7777/v1/health >/dev/null
CMD ["./strike-hub"]

FROM toolchain AS server-build
COPY Cargo.toml Cargo.lock ./
COPY crates/strike-hub ./crates/strike-hub
COPY src ./src
COPY assets/audio/generated/catalog.ron ./assets/audio/generated/catalog.ron
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --locked --release --bin strike-server --no-default-features \
    && mkdir -p /out \
    && cp /src/target/release/strike-server /out/

FROM debian:bookworm-slim AS server
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates libasound2 libvulkan1 mesa-vulkan-drivers \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=server-build /out/ /app/
COPY assets/generated /app/assets/generated
COPY assets/maps /app/assets/maps
COPY assets/audio/generated /app/assets/audio/generated
COPY assets/config /app/assets/config
COPY assets/fonts /app/assets/fonts
ENV STRIKE_SERVER_KEY=dev \
    WGPU_BACKEND=vulkan \
    WGPU_POWER_PREF=low
EXPOSE 27015/udp
CMD ["./strike-server", "--hub", "http://hub:7777", "--port", "27015", "--advertise", "127.0.0.1", "--mode", "tdm", "--map", "dust2", "--bots", "6"]
