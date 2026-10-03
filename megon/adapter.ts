/**
 * Megon 3.0 Adapter — Bridges AgentForge orchestrator to Forge + Airlock APIs.
 * This module is called by the orchestrator when a user task requires coding or security checks.
 * Uses standard fetch (available in Node 23/Bun) — no extra dependencies needed.
 */

const FORGE_API = process.env.FORGE_API_URL || 'http://localhost:5000';
const AIRLOCK_API = process.env.AIRLOCK_API_URL || 'http://localhost:5001';

export interface ForgeRunRequest {
  problem_text: string;
  repo?: string;
  issue_number?: number;
}

export interface ForgeRunResponse {
  exit_status: string;
  has_submission: boolean;
  submission_preview?: string;
  trajectory_path?: string;
}

export interface AirlockCheckRequest {
  code?: string;
  package?: string;
  mode?: 'code' | 'package';
}

export interface AirlockVerdict {
  verdict: 'SAFE' | 'BLOCK' | 'UNKNOWN';
  reasons: string[];
  details: Record<string, unknown>;
}

/**
 * Send a coding task to the Forge agent via its REST API.
 * Works with both GitHub issues and raw problem descriptions.
 */
export async function runForgeTask(req: ForgeRunRequest): Promise<ForgeRunResponse> {
  const res = await fetch(`${FORGE_API}/api/run`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      problem_text: req.problem_text,
      github_url: req.repo ? `https://github.com/${req.repo}` : undefined,
      issue_number: req.issue_number,
    }),
  });

  if (!res.ok) {
    throw new Error(`Forge API error: ${res.status} ${await res.text()}`);
  }

  return await res.json() as ForgeRunResponse;
}

/**
 * Send generated code to Airlock for security verification.
 * Returns SAFE/BLOCK verdict without requiring paid APIs.
 */
export async function checkSecurity(req: AirlockCheckRequest): Promise<AirlockVerdict> {
  try {
    const res = await fetch(`${AIRLOCK_API}/api/check`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(req),
    });

    if (!res.ok) {
      return { verdict: 'UNKNOWN', reasons: [`Airlock HTTP ${res.status}`], details: {} };
    }

    return await res.json() as AirlockVerdict;
  } catch (err) {
    // If Airlock is down, don't block the pipeline — warn and continue
    console.warn('[Megon] Airlock unavailable, skipping security check:', err);
    return { verdict: 'UNKNOWN', reasons: ['Airlock service unavailable'], details: {} };
  }
}

/**
 * Full pipeline: Research → Code → Verify → Return
 * Called by the orchestrator when user gives a general task.
 */
export async function executeMegonPipeline(task: string): Promise<{
  result: string;
  security: AirlockVerdict;
}> {
  // Step 1: Send to Forge for coding/reasoning
  const forgeResult = await runForgeTask({ problem_text: task });
  const codeOutput = forgeResult.submission_preview || 'No code generated';

  // Step 2: Verify with Airlock
  const security = await checkSecurity({ code: codeOutput, mode: 'code' });

  return {
    result: codeOutput,
    security,
  };
}
