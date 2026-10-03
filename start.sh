#!/bin/bash
set -e
echo "🚀 Starting Megon 3.0 Decentralized AI Agent..."
echo "   Model: ${MODEL:-not set}"
echo "   Base URL: ${BASE_URL:-not set}"

# Map unified env vars to service-specific ones
export OPENAI_API_KEY="${API_KEY}"
export OPENAI_API_URL="${BASE_URL}"
export MODEL_NAME="${MODEL}"
export FORGE_MODEL="${MODEL}"
export FORGE_BASE_URL="${BASE_URL}"
export FORGE_API_KEY="${API_KEY}"
export FORGE_ENABLE_LEGACY_RUN_API=true
export FORGE_API_PORT=5000

# Start Airlock Security API in background
echo "[1/3] Starting Airlock Security Gate on :5001..."
cd /app/airlock && /opt/airlock-venv/bin/uvicorn api:app --host 0.0.0.0 --port 5001 &
AIRLOCK_PID=$!

# Start Forge Coding API in background
echo "[2/3] Starting Forge Coding Agent on :5000..."
forge-api &
FORGE_PID=$!

# Wait for sub-services to initialize
sleep 3

# Start AgentForge Orchestrator (uses its own entrypoint logic inline)
echo "[3/3] Starting AgentForge Orchestrator..."
cd /app/agentforge
export SERVER_PORT=3002
export CORS_ORIGIN="*"
bun run start &
ELIZA_PID=$!

# Wait for Fleet API
echo "[Megon] Waiting for Fleet API on :3001..."
for i in $(seq 1 60); do
    if curl -sf http://127.0.0.1:3001/fleet/auth/token > /dev/null 2>&1; then
        echo "[Megon] Fleet API ready"
        break
    fi
    sleep 1
done

# Start nginx
if [ -f /app/agentforge/nginx.conf ]; then
    echo "[Megon] Starting nginx reverse proxy on :3000"
    nginx -c /app/agentforge/nginx.conf &
fi

echo "✅ Megon 3.0 is running!"
echo "   UI/API: http://localhost:3000"
echo "   Forge:  http://localhost:5000"
echo "   Airlock: http://localhost:5001"

# Wait for any process to exit
wait $AIRLOCK_PID $FORGE_PID $ELIZA_PID
