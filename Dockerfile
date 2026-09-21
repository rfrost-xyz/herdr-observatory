FROM python:3.13-slim-bookworm@sha256:2325bb286ec344af3e5898cc224b5844e2707ac6e26b1632516fd3edc84a5e26
RUN apt-get update && apt-get install -y --no-install-recommends openssh-client \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 1000 observatory && useradd --uid 1000 --gid 1000 --no-create-home observatory
WORKDIR /app
COPY observatory /app/observatory
COPY web /app/web
ARG REVISION=unknown
LABEL org.opencontainers.image.title="Herdr Observatory" \
      org.opencontainers.image.source="https://github.com/rfrost-xyz/herdr-observatory" \
      org.opencontainers.image.revision=$REVISION
ENV PYTHONDONTWRITEBYTECODE=1 PYTHONUNBUFFERED=1 HOME=/tmp
USER 1000:1000
ENTRYPOINT ["python3", "-m", "observatory"]
CMD ["--config", "/config/config.json", "--profile", "work"]
