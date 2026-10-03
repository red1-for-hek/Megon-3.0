#!/usr/bin/env bash
set -euo pipefail

if [[ ! -f .env.production ]]; then
  echo "Missing .env.production. Copy .env.production.example and fill it before deploying." >&2
  exit 1
fi

if [[ ! -f secrets/github-app.pem ]]; then
  echo "Missing secrets/github-app.pem. Add the GitHub App private key before deploying." >&2
  exit 1
fi

docker compose --env-file .env.production -f docker-compose.prod.yml pull caddy
docker compose --env-file .env.production -f docker-compose.prod.yml up -d --build
docker compose --env-file .env.production -f docker-compose.prod.yml ps
