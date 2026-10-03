# Forge Deployment Notes

This directory contains production deployment files for Forge.

- `Caddyfile` terminates HTTPS and proxies to `forge-api`.
- `docker-compose.prod.yml` at the repo root runs `forge-api` and Caddy.
- `.env.production.example` documents required runtime variables.

Do not copy GitHub App private keys or `.env.production` into git. Keep them on the server only.

The production Docker build intentionally excludes copied/reference projects such as `pr-agent/` through `.dockerignore`; Forge must ship as its own application.
