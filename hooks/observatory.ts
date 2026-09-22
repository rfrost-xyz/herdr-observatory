// herdr-observatory adapter v1; installed from the running image
// Reports event metadata only. Never returns hook decisions or message content.
import { spawn } from 'node:child_process';

export default function (pi) {
  const pane = process.env.HERDR_PANE_ID;
  if (process.env.HERDR_ENV !== '1' || !pane) return;
  let sequence = Date.now() * 1000;
  let usage = {};
  let phase = '';
  let queue = [];
  let running = false;
  async function drain() {
    if (running) return;
    running = true;
    try {
      while (queue.length) {
        const item = queue.shift();
        await new Promise((resolve) => {
          const child = spawn('docker', ['exec', '-i', '__CONTAINER__', 'python3', '-m',
            'observatory.telemetry', 'pi', pane, String(item.seq)], { stdio: ['pipe', 'ignore', 'ignore'] });
          const timer = setTimeout(() => { child.kill('SIGKILL'); resolve(); }, 2000);
          const done = () => { clearTimeout(timer); resolve(); };
          child.on('error', done); child.on('close', done);
          child.stdin.on('error', () => {});
          child.stdin.end(JSON.stringify(item));
        });
      }
    } finally { running = false; }
  }
  function emit(event, nextPhase, ctx, extra = {}) {
    try {
      const context = ctx.getContextUsage?.();
      sequence = Math.max(sequence + 1, Date.now() * 1000);
      const item = { event, phase: nextPhase, seq: sequence,
        session_id: ctx.sessionManager.getSessionId(), session_path: ctx.sessionManager.getSessionFile(),
        model: ctx.model?.id, context: context?.tokens, window: ctx.model?.contextWindow,
        ...usage, ...extra };
      phase = nextPhase;
      if (queue.length >= 16) queue.shift();
      queue.push(item);
      void drain();
    } catch { /* telemetry failure must never affect the harness */ }
  }
  pi.on('session_start', (_e, ctx) => { queue = []; usage = {}; emit('session', 'ready', ctx); });
  pi.on('agent_start', (_e, ctx) => emit('turn', 'working', ctx));
  pi.on('agent_end', (_e, ctx) => emit('idle', 'idle', ctx));
  pi.on('tool_execution_start', (e, ctx) => emit('tool-start', 'tool', ctx, { tool: e.toolName }));
  pi.on('tool_execution_end', (e, ctx) => emit('tool-end', 'working', ctx,
    { tool: e.toolName, result: e.isError === true ? 'error' : 'finished' }));
  pi.on('message_update', (e, ctx) => {
    const type = e.assistantMessageEvent?.type;
    if (type === 'thinking_start' && phase !== 'thinking') emit('thinking', 'thinking', ctx);
    if (type === 'text_start' && phase !== 'output') emit('output', 'output', ctx);
  });
  pi.on('message_end', (e, ctx) => {
    if (e.message?.role !== 'assistant') return;
    const u = e.message.usage;
    usage = u ? { input: u.input, output_tokens: u.output, cache_read: u.cacheRead, cache_write: u.cacheWrite } : {};
    emit('output', 'output', ctx);
  });
  pi.on('session_before_compact', (_e, ctx) => emit('compact-start', 'compacting', ctx));
  pi.on('session_compact', (_e, ctx) => { usage = {}; emit('compact-end', 'working', ctx); });
  pi.on('session_compact_failed', (e, ctx) => emit('compact-failed', 'working', ctx,
    { result: e.aborted === true ? 'cancelled' : 'error' }));
  pi.on('model_select', (_e, ctx) => { usage = {}; emit('model', 'ready', ctx); });
  pi.on('session_shutdown', (_e, ctx) => { emit('end', 'ended', ctx); });
}
