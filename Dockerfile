FROM node:22-bookworm-slim AS web
WORKDIR /src/web
COPY web/package.json web/package-lock.json ./
RUN npm ci --ignore-scripts
COPY web/ ./
RUN npm run build

FROM rust:1-bookworm AS server
WORKDIR /src/server
COPY server/Cargo.toml server/Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release && rm -rf src
COPY server/ ./
RUN touch src/main.rs && cargo build --release

FROM debian:bookworm-slim
RUN useradd --system --uid 10001 --home /app hearth && mkdir -p /data/uploads && chown hearth /data/uploads
WORKDIR /app
COPY --from=server /src/server/target/release/hearth-server /app/hearth-server
COPY --from=web /src/web/dist /app/web
ENV BIND=0.0.0.0:8080 WEB_DIR=/app/web UPLOAD_DIR=/data/uploads
USER hearth
EXPOSE 8080
VOLUME /data/uploads
CMD ["/app/hearth-server"]
