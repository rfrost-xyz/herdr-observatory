FROM rust:1.96-slim-bookworm@sha256:e18a79fc84dfcfc3ab5ba72290398a644c135c97eaa881447fddc354ee4701a3 AS text-builder
WORKDIR /build
COPY renderer/Cargo.toml renderer/Cargo.lock ./
COPY renderer/src ./src
RUN cargo build --release --locked

FROM python:3.13-slim-bookworm@sha256:2325bb286ec344af3e5898cc224b5844e2707ac6e26b1632516fd3edc84a5e26
RUN apt-get update && apt-get install -y --no-install-recommends openssh-client \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 1000 observatory && useradd --uid 1000 --gid 1000 --no-create-home observatory
WORKDIR /app
COPY observatory /app/observatory
COPY web /app/web
COPY --from=text-builder /build/target/release/observatory-text /usr/local/bin/observatory-text
COPY renderer/NOTICE renderer/TTFX-LICENSE /usr/share/doc/observatory-text/
ARG REVISION=unknown
LABEL org.opencontainers.image.title="Herdr Observatory" \
      org.opencontainers.image.source="https://github.com/rfrost-xyz/herdr-observatory" \
      org.opencontainers.image.revision=$REVISION
ENV PYTHONDONTWRITEBYTECODE=1 PYTHONUNBUFFERED=1 HOME=/tmp
USER 1000:1000
ENTRYPOINT ["python3", "-m", "observatory"]
CMD ["--config", "/config/config.json", "--profile", "work"]
