# Demo: Prime Number Checker

This example demonstrates the full Megon 3.0 pipeline:
**User Task → Planner → Code Generation → Security Verification → Testing → Result**

## Run

```bash
node megon/orchestrator.mjs "Create a Python function that checks whether a number is prime, test it with numbers 2, 17, and 25, and explain the result."
```

## Expected Pipeline Steps

1. **Planner** analyzes the task and determines code generation + testing are needed
2. **Coding Agent** generates an `is_prime()` function with test cases
3. **Security Gate** performs static analysis on the generated code → SAFE verdict
4. **Test Runner** executes the code in a sandboxed subprocess with timeout
5. **Result** returns verified code, test output, and explanation

## What This Proves

- Megon understands natural-language tasks (no GitHub issue required)
- Code is generated autonomously by the AI agent
- Generated code passes through security verification before execution
- Tests run in isolation (sandboxed subprocess with 10s timeout)
- The full pipeline works with free OpenRouter models ($0 cost)

## Nosana Relevance

In production, each step (planning, coding, security analysis) would execute
on a separate Nosana GPU worker node, providing decentralized compute for
the entire AI agent workflow.
