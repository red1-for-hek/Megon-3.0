/**
 * Megon 3.0 Orchestrator — Hackathon MVP
 * 
 * Pure Node.js (23+), zero dependencies. Uses built-in fetch.
 * Flow: User Task → LLM Planner → Code Generation → Security Check → Result
 * 
 * Usage: node megon/orchestrator.mjs "your task here"
 */

import { readFileSync, existsSync, writeFileSync, unlinkSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';
import { execSync } from 'child_process';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, '..');

// ── Load .env manually (no dotenv dependency) ──────────────────────
function loadEnv() {
  const envPath = resolve(ROOT, '.env');
  if (!existsSync(envPath)) {
    console.error('❌ ERROR: No .env file found at', envPath);
    console.error('   Run: cp .env.example .env && edit .env with your API key');
    process.exit(1);
  }
  const lines = readFileSync(envPath, 'utf8').split('\n');
  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('#')) continue;
    const eqIdx = trimmed.indexOf('=');
    if (eqIdx === -1) continue;
    const key = trimmed.slice(0, eqIdx).trim();
    const val = trimmed.slice(eqIdx + 1).trim();
    if (key && !process.env[key]) process.env[key] = val;
  }
}
loadEnv();

// ── Config validation ──────────────────────────────────────────────
const MODEL = process.env.MODEL;
const BASE_URL = process.env.BASE_URL;
const API_KEY = process.env.API_KEY;
const AIRLOCK_URL = process.env.AIRLOCK_API_URL || `http://localhost:${process.env.AIRLOCK_PORT || 5001}`;
const FORGE_URL = process.env.FORGE_API_URL || `http://localhost:${process.env.FORGE_PORT || 5000}`;

if (!API_KEY || API_KEY.includes('your_') || API_KEY.trim() === '') {
  console.error('❌ ERROR: API_KEY is not set in .env');
  console.error('   Get a free key at https://openrouter.ai/keys');
  console.error('   Then set API_KEY=sk-or-v1-... in your .env file');
  process.exit(1);
}
if (!MODEL) { console.error('❌ ERROR: MODEL not set in .env'); process.exit(1); }
if (!BASE_URL) { console.error('❌ ERROR: BASE_URL not set in .env'); process.exit(1); }

console.log(`🔧 Megon 3.0 configured: model=${MODEL}, base=${BASE_URL.replace(/\/v1.*/, '')}/...`);

// ── LLM Client (OpenAI-compatible) ─────────────────────────────────
async function llmChat(systemPrompt, userMessage) {
  const url = `${BASE_URL.replace(/\/$/, '')}/chat/completions`;
  const res = await fetch(url, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${API_KEY}`,
    },
    body: JSON.stringify({
      model: MODEL,
      messages: [
        { role: 'system', content: systemPrompt },
        { role: 'user', content: userMessage }
      ],
      temperature: 0.3,
      max_tokens: 4096,
    }),
  });
  if (!res.ok) {
    const errText = await res.text();
    throw new Error(`LLM API error ${res.status}: ${errText}`);
  }
  const data = await res.json();
  return data.choices[0].message.content;
}

// ── Airlock Security Check ─────────────────────────────────────────
async function securityCheck(code) {
  try {
    const res = await fetch(`${AIRLOCK_URL}/api/check`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ code, mode: 'code' }),
    });
    if (!res.ok) return { verdict: 'UNKNOWN', reasons: [`Airlock HTTP ${res.status}`] };
    return await res.json();
  } catch {
    console.log('⚠️  Airlock API not reachable, using local static analysis fallback');
    return localSecurityCheck(code);
  }
}

function localSecurityCheck(code) {
  const reasons = [];
  const patterns = [
    [/eval\s*\(/, 'Uses eval()'],
    [/exec\s*\(/, 'Uses exec()'],
    [/os\.system\s*\(/, 'Uses os.system()'],
    [/subprocess\.call.*shell\s*=\s*True/, 'Subprocess with shell=True'],
    [/__import__\s*\(/, 'Dynamic import'],
    [/curl\s+.*\|\s*bash/, 'Pipe to bash pattern'],
    [/rm\s+-rf\s+\//, 'Destructive rm command'],
  ];
  for (const [regex, desc] of patterns) {
    if (regex.test(code)) reasons.push(`Suspicious: ${desc}`);
  }
  return {
    verdict: reasons.length > 0 ? 'BLOCK' : 'SAFE',
    reasons: reasons.length > 0 ? reasons : ['Local static analysis passed'],
    details: { engine: 'megon-local-fallback' }
  };
}

// ── Forge Integration (optional — uses LLM directly if unavailable) ─
async function forgeCode(task) {
  try {
    const res = await fetch(`${FORGE_URL}/api/run`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ problem_text: task }),
    });
    if (res.ok) {
      const data = await res.json();
      if (data.has_submission) return { source: 'forge', code: data.submission_preview };
    }
  } catch {
    // Forge not running — fall through to LLM direct
  }
  return null;
}

// ── Sandboxed Test Runner ──────────────────────────────────────────
function runSandboxTest(code) {
  const tmpFile = resolve(ROOT, '.megon_test_tmp.py');
  try {
    writeFileSync(tmpFile, code);
    const output = execSync(`timeout 10 python3 ${tmpFile} 2>&1`, {
      encoding: 'utf8',
      timeout: 15000,
    });
    return `✅ Passed\n${output.trim()}`;
  } catch (e) {
    if (e.stdout) return `⚠️ Output: ${String(e.stdout).trim().slice(0, 500)}`;
    return `❌ Failed: ${e.message.slice(0, 200)}`;
  } finally {
    try { unlinkSync(tmpFile); } catch {}
  }
}

// ── Main Pipeline ──────────────────────────────────────────────────
async function runPipeline(task) {
  console.log('\n📋 TASK:', task);
  console.log('─'.repeat(60));

  // Step 1: Plan
  console.log('\n🧠 Step 1: Planning...');
  const plan = await llmChat(
    'You are the Megon 3.0 Planner. Analyze the task and output a brief execution plan as JSON: {"steps": [...], "needs_code": bool, "needs_research": bool}. Keep it concise.',
    task
  );
  console.log('Plan:', plan.slice(0, 300));

  // Step 2: Try Forge first, fall back to direct LLM coding
  console.log('\n⚡ Step 2: Generating solution...');
  let code = '';
  let explanation = '';

  const forgeResult = await forgeCode(task);
  if (forgeResult) {
    console.log('✅ Used Forge coding agent');
    code = forgeResult.code;
  } else {
    console.log('ℹ️  Forge unavailable, using direct LLM generation');
    const response = await llmChat(
      `You are the Megon 3.0 Coding Agent. Complete the user's task.
If code is needed, wrap ALL code in a single markdown code block \`\`\`python ... \`\`\` or appropriate language.
After the code, provide a brief explanation.
Do NOT use any external APIs or network calls in generated code unless explicitly asked.
Keep code simple, safe, and testable.`,
      task
    );
    const codeMatch = response.match(/```(?:python|javascript|bash)?\n?([\s\S]*?)```/);
    code = codeMatch ? codeMatch[1].trim() : '';
    explanation = codeMatch ? response.replace(codeMatch[0], '').trim() : response;
  }

  // Step 3: Security check via Airlock
  console.log('\n🔒 Step 3: Security verification...');
  const verdict = await securityCheck(code);
  console.log(`Verdict: ${verdict.verdict}`);
  if (verdict.reasons?.length) {
    console.log('Reasons:', verdict.reasons.join(', '));
  }

  // Step 4: Test execution (sandboxed)
  console.log('\n🧪 Step 4: Testing...');
  let testResult = 'Skipped (non-Python or no code)';
  if (code && verdict.verdict !== 'BLOCK') {
    testResult = runSandboxTest(code);
  }

  // Step 5: Final summary
  console.log('\n' + '═'.repeat(60));
  console.log('✅ MEGON 3.0 RESULT');
  console.log('═'.repeat(60));
  console.log('\n📝 CODE:\n' + (code || 'No code generated'));
  console.log('\n🔒 SECURITY: ' + verdict.verdict);
  console.log('📊 TEST: ' + testResult);
  if (explanation) console.log('\n💡 EXPLANATION:\n' + explanation);
  console.log('\n' + '═'.repeat(60));

  return { code, verdict, testResult, explanation };
}

// ── CLI Entry ───────────────────────────────────────────────────────
const task = process.argv.slice(2).join(' ') || 'Create a Python function that checks whether a number is prime, test it with numbers 2, 17, and 25, and explain the result.';

runPipeline(task).catch(err => {
  console.error('❌ Pipeline failed:', err.message);
  process.exit(1);
});
