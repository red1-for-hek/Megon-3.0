import { Sandbox } from 'e2b'

const template = process.env.E2B_TEMPLATE || undefined
const timeoutMs = Number(process.env.E2B_TIMEOUT_MS || '120000')

let sandbox
try {
  sandbox = await Sandbox.create(template ? { template } : undefined)
  const result = await sandbox.commands.run('echo forge-e2b-live', {
    timeoutMs,
  })

  const stdout = String(result.stdout || '').trim()
  if (stdout !== 'forge-e2b-live') {
    console.error(`unexpected stdout: ${JSON.stringify(result.stdout)}`)
    process.exitCode = 1
  } else {
    console.log(`sandbox=${sandbox.sandboxId}`)
    console.log(`stdout=${stdout}`)
  }
} finally {
  if (sandbox) {
    await sandbox.kill()
  }
}
