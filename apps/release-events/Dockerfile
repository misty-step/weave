FROM rust:1-bookworm AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY apps/release-events/Cargo.toml apps/release-events/Cargo.toml
COPY apps/release-events/src apps/release-events/src
RUN cargo build --locked --release -p weave-release-events

FROM debian:bookworm-slim

RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates \
  && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/weave-release-events /usr/local/bin/weave-release-events

ENV RELEASE_EVENTS_ADDR=0.0.0.0:8080
ENV RELEASE_EVENTS_ROOT=/data/events

CMD ["weave-release-events"]
