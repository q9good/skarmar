# syntax=docker/dockerfile:1.7
FROM node:24.19.0-bookworm-slim AS web
WORKDIR /src
COPY package.json package-lock.json ./
COPY apps/client/package.json apps/client/package.json
RUN --mount=type=secret,id=proxy_ca \
    if [ -f /run/secrets/proxy_ca ]; then export NODE_EXTRA_CA_CERTS=/run/secrets/proxy_ca; fi; \
    npm ci --no-audit --no-fund --strict-ssl=true
COPY apps/client apps/client
ENV CI=1 EXPO_NO_TELEMETRY=1 EXPO_OFFLINE=1 EXPO_PUBLIC_API_URL=""
RUN npm run typecheck && npm run web:build

FROM rust:1.99.0-slim-bookworm AS rust
WORKDIR /src
RUN --mount=type=secret,id=proxy_ca \
    task_ca_file=/etc/ssl/certs/ca-certificates.crt; \
    if [ -f /run/secrets/proxy_ca ]; then task_ca_file=/run/secrets/proxy_ca; fi; \
    apt-get -o "Acquire::https::CaInfo=$task_ca_file" update && \
    apt-get -o "Acquire::https::CaInfo=$task_ca_file" install -y --no-install-recommends gcc libc6-dev && \
    rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock ./
COPY crates crates
RUN --mount=type=secret,id=proxy_ca \
    if [ -f /run/secrets/proxy_ca ]; then export CARGO_HTTP_CAINFO=/run/secrets/proxy_ca; fi; \
    cargo build -p skarma-api --release --locked -j 4

FROM debian:bookworm-slim
RUN --mount=type=secret,id=proxy_ca \
    task_ca_file=/etc/ssl/certs/ca-certificates.crt; \
    if [ -f /run/secrets/proxy_ca ]; then task_ca_file=/run/secrets/proxy_ca; fi; \
    apt-get -o "Acquire::https::CaInfo=$task_ca_file" update && \
    apt-get -o "Acquire::https::CaInfo=$task_ca_file" install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/* && \
    groupadd --gid 10001 skarma && useradd --uid 10001 --gid skarma --no-create-home skarma && \
    mkdir -p /app/web /data && chown 10001:10001 /data && chmod 700 /data
COPY --from=rust /src/target/release/skarma-api /usr/local/bin/skarma-api
COPY --from=web /src/apps/client/dist /app/web
USER 10001:10001
WORKDIR /app
ENV SKARMA_DATABASE=/data/skarma.db SKARMA_WEB_ASSETS=/app/web SKARMA_STORAGE=sqlite SKARMA_AUTH=required SKARMA_DEMO=0 SKARMA_BIND=0.0.0.0 SKARMA_PORT=3001
EXPOSE 3001
ENTRYPOINT ["skarma-api"]
CMD ["serve"]
