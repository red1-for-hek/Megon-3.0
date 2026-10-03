# Megon 3.0 — Decentralized AI Software Engineering Agent

Megon 3.0 is a unified autonomous AI agent that runs on the **Nosana decentralized GPU network**. It combines orchestration (AgentForge), coding capabilities (Forge), and security verification (Airlock) into a single deployable stack.

## Zero-Cost Setup (Free Models)
This project is designed to run with **$0 personal spending** by using free OpenRouter models.

1. Copy environment template:
```bash
cp .env.example .env
```
2. Edit `.env`:
   - Get a free API key from [OpenRouter](https://openrouter.ai/)
   - Set `MODEL=openrouter/auto` (or any free model like `meta-llama/llama-3-8b-instruct:free`)
   - Set `BASE_URL=https://openrouter.ai/api/v1`
   - Set `API_KEY=your_openrouter_key`
   - Add your `NOSANA_API_KEY` for decentralized compute deployment.

## Run Locally (Docker Compose)
```bash
docker compose up --build
```
- **Orchestrator UI/API**: http://localhost:3000
- **Forge Coding API**: http://localhost:5000
- **Airlock Security API**: http://localhost:5001

## Deploy to Nosana
The architecture is pre-configured for Nosana GPU workers.
```bash
nosana job deploy nosana/megon_job_definition.json
```

## Architecture
User → AgentForge Orchestrator → (Forge Coding Agent + Airlock Security Gate) → Verified Result.
All components communicate over internal Docker networking. No paid APIs are required if using free OpenRouter endpoints.
