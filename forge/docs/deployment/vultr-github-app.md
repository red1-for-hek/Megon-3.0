# Deploy Forge On Vultr With A GitHub App

This guide gets Forge a public URL so GitHub users can install the Forge GitHub App, select repositories, and trigger Forge from issues or PR comments.

## What Works Today

The current backend can:

- Receive GitHub App webhooks at `/api/github/webhook`.
- Verify `x-hub-signature-256` when `GITHUB_WEBHOOK_SECRET` is configured.
- Parse real GitHub `issues` and `issue_comment` payloads.
- Convert issue label `forge` into `ForgeCommand::Plan`.
- Convert comments like `/forge plan` and `/forge review` into Forge commands.
- Create a GitHub App JWT and exchange it for an installation token.
- Post GitHub issue/PR comments with the installation token.
- Run a real E2B live smoke test that creates a sandbox and executes a command.
- Keep legacy `/api/run` disabled in production unless `FORGE_ENABLE_LEGACY_RUN_API=true`.

## What Is Still Missing

Forge is not a complete public product yet. Remaining work:

- GitHub App install landing page and setup UI.
- Persistent install/job storage.
- Background worker queue.
- Real model-generated `/forge plan` implementation.
- `/forge approve` state transition.
- E2B issue-fixing runner that clones a repo, edits code, runs checks, and returns a diff.
- Branch publishing from E2B output.
- Native PR review implementation inspired by PR-Agent, without importing or shipping PR-Agent.
- Production hardening for legacy endpoints like `/api/run`.
- End-to-end live test against a real installed GitHub App and test repo.

## Local Testing With A Public Tunnel

GitHub cannot send webhooks to `localhost`. For local testing, expose the local API through a tunnel.

Run Forge locally:

```bash
FORGE_API_PORT=5000 \
GITHUB_WEBHOOK_SECRET=your-secret \
GITHUB_APP_ID=your-app-id \
GITHUB_APP_PRIVATE_KEY_PATH=/absolute/path/to/github-app.pem \
E2B_API_KEY=your-e2b-key \
cargo run -p forge-api
```

Expose it with a tunnel such as Cloudflare Tunnel or ngrok:

```bash
cloudflared tunnel --url http://localhost:5000
```

Use the generated HTTPS URL as the GitHub App webhook URL:

```text
https://your-tunnel-url.trycloudflare.com/api/github/webhook
```

When local testing is finished, update the GitHub App webhook URL to the Vultr domain.

## Create The GitHub App

In GitHub, open:

```text
Settings -> Developer settings -> GitHub Apps -> New GitHub App
```

Use these values:

- GitHub App name: `Forge`
- Homepage URL: `https://forge.yourdomain.com`
- Webhook URL: `https://forge.yourdomain.com/api/github/webhook`
- Webhook secret: create a long random value and save it as `GITHUB_WEBHOOK_SECRET`

Permissions:

- Metadata: Read-only
- Contents: Read and write
- Issues: Read and write
- Pull requests: Read and write
- Checks: Read-only
- Actions: Read-only

Subscribe to events:

- Issues
- Issue comments
- Pull requests
- Pull request review comments
- Check runs

After creating the app:

1. Generate a private key.
2. Save the app ID as `GITHUB_APP_ID`.
3. Install the app on a test repository.

## Deploy On Vultr

Create a Vultr server:

- Ubuntu 24.04
- At least 1 vCPU / 1 GB RAM for early testing
- Open firewall ports `80` and `443`

For the current test server, the public IP is:

```text
149.28.121.155
```

SSH into the server and install Docker if the image does not already include it:

```bash
sudo apt-get update
sudo apt-get install -y ca-certificates curl git docker.io docker-compose-plugin
sudo usermod -aG docker "$USER"
newgrp docker
```

Clone Forge:

```bash
git clone https://github.com/OkeyAmy/forge.git
cd forge
```

Create the production environment file:

```bash
cp .env.production.example .env.production
nano .env.production
```

Set these values:

```dotenv
FORGE_DOMAIN=forge.yourdomain.com
FORGE_ENABLE_LEGACY_RUN_API=false
E2B_API_KEY=...
GITHUB_WEBHOOK_SECRET=...
GITHUB_APP_ID=...
FORGE_MODEL=...
FORGE_BASE_URL=...
FORGE_API_KEY=...
```

Add the GitHub App private key:

```bash
mkdir -p secrets
nano secrets/github-app.pem
chmod 600 secrets/github-app.pem
```

Point DNS:

```text
forge.yourdomain.com -> Vultr server public IP
```

Start Forge:

```bash
chmod +x scripts/deploy-vultr.sh
./scripts/deploy-vultr.sh
```

Check the API:

```bash
curl https://forge.yourdomain.com/health
```

Expected response:

```json
{"status":"ok","version":"0.1.0"}
```

## Connect Users

Users connect by installing your GitHub App:

1. Open the public GitHub App installation page:
   ```text
   https://github.com/apps/<your-forge-app-name>/installations/new
   ```
2. Select repositories.
3. Add the `forge` label to an issue or comment `/forge plan`.
4. GitHub sends the webhook to Forge.
5. Forge uses the installation token for that repository.

The user does not need to configure tokens manually.

For a public product, link users to the installation URL from the Forge landing page. The hosted API receives all repository events through installation-scoped tokens, so users do not paste personal access tokens into Forge.

## Live Test Checklist

After deployment:

1. Install the app on a test repo.
2. Create a GitHub issue.
3. Add label `forge`.
4. Confirm the Forge server logs receive an `issues` webhook.
5. Confirm Forge posts a plan comment.
6. Comment `/forge review` on a PR and confirm Forge accepts the PR command.
