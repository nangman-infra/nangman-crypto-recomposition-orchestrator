FROM public.ecr.aws/docker/library/rust:1.94-bookworm AS builder

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates pkg-config \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release --locked

FROM gcr.io/distroless/cc-debian12:nonroot AS runtime

COPY --from=builder --chown=nonroot:nonroot \
    /app/target/release/recomposition-orchestrator-app \
    /usr/local/bin/recomposition-orchestrator-app

USER nonroot:nonroot

ENV AWS_SDK_LOAD_CONFIG=1

ENTRYPOINT ["/usr/local/bin/recomposition-orchestrator-app"]
