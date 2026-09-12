# Chatbot server, no Tauri and no desktop dependencies.
# The .langolier bundle and models/ are mounted from ./data.
FROM rust:1-bookworm AS build
RUN apt-get update \
 && apt-get install -y --no-install-recommends cmake clang libclang-dev pkg-config \
 && rm -rf /var/lib/apt/lists/*
WORKDIR /build
COPY src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/build.rs ./
COPY src-tauri/src ./src
COPY src-tauri/web ./web
# llama.cpp tunes itself for the CPU that compiles it, so build on the target host.
RUN cargo build --release --no-default-features --bin langolier \
 && strip target/release/langolier

FROM debian:bookworm-slim
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates libgomp1 \
 && rm -rf /var/lib/apt/lists/*
COPY --from=build /build/target/release/langolier /usr/local/bin/langolier
ENV LANGOLIER_BUNDLE_DIR=/data
VOLUME ["/data", "/state"]
EXPOSE 8787
ENTRYPOINT ["langolier", "--serve", "--host", "0.0.0.0", "--port", "8787", \
            "--bundle", "/data", "--data", "/state"]
