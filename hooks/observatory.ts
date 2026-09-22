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
  function measuredUsage(message, live=false) {
    const u=message?.usage;
    const stamp=Number.isFinite(message?.timestamp)?message.timestamp:(live?Date.now():NaN);
    if(!u || !Number.isFinite(stamp) || Date.now()-stamp<0)return {};
    return {input:u.input,output_tokens:u.output,cache_read:u.cacheRead,cache_write:u.cacheWrite,
      usage_seq:Math.floor(stamp*1000),usage_source:'pi-extension'};
  }
  function seedUsage(ctx) {
    try {
      const branch=ctx.sessionManager.getBranch?.();
      if(!Array.isArray(branch))return;
      const latest=branch.slice(-128).reverse().find(entry=>entry.type==='message' && entry.message?.role==='assistant');
      const candidate=latest?measuredUsage(latest.message):{};
      // agent_end can precede persistence of the latest message in the branch.
      if(!usage.usage_seq || (candidate.usage_seq && candidate.usage_seq>=usage.usage_seq))usage=candidate;
    }catch{ /* Keep the last measured value if branch lookup fails. */ }
  }
  function totalsFor(ctx) {
    try {
      const entries=ctx.sessionManager.getEntries?.();
      if(!Array.isArray(entries) || entries.length>4096)return {};
      const totals={total_input:0,total_output:0,total_cache_read:0,total_cache_write:0,compactions:0};
      const fields={total_input:'input',total_output:'output',total_cache_read:'cacheRead',total_cache_write:'cacheWrite'};
      let measured=0;
      for(const entry of entries){
        if(entry?.type==='compaction')totals.compactions++;
        const u=entry?.type==='message' && entry.message?.role==='assistant'?entry.message.usage:['usage','compaction','branch_summary'].includes(entry?.type)?entry.usage:null;
        if(!u){if((entry?.type==='message' && entry.message?.role==='assistant') || ['usage','compaction','branch_summary'].includes(entry?.type))for(const key of Object.keys(fields))totals[key]=null;continue;}
        measured++;
        for(const [key,source] of Object.entries(fields)){
          const value=u[source];
          if(totals[key]===null)continue;
          totals[key]=Number.isSafeInteger(value) && value>=0 && Number.isSafeInteger(totals[key]+value)?totals[key]+value:null;
        }
      }
      if(!measured)return {compactions:totals.compactions};
      // Pi reports uncached input separately from cache read/write, unlike Codex.
      totals.total_uncached_input=totals.total_input;
      if(totals.total_input!==null && totals.total_cache_read!==null && totals.total_cache_write!==null){
        const allInput=totals.total_input+totals.total_cache_read+totals.total_cache_write;
        totals.total_input=Number.isSafeInteger(allInput)?allInput:null;
      }else totals.total_input=null;
      return totals;
    }catch{return {};}
  }
  function emit(event, nextPhase, ctx, extra = {}) {
    try {
      const context = ctx.getContextUsage?.();
      sequence = Math.max(sequence + 1, Date.now() * 1000);
      const item = { event, phase: nextPhase, seq: sequence,
        session_id: ctx.sessionManager.getSessionId(), session_path: ctx.sessionManager.getSessionFile(),
        model: ctx.model?.id, context: context?.tokens, window: ctx.model?.contextWindow,
        ...usage, ...totalsFor(ctx), context_percent:Number.isFinite(context?.percent)?Math.round(context.percent):undefined, ...extra };
      phase = nextPhase;
      if (queue.length >= 16) queue.shift();
      queue.push(item);
      void drain();
    } catch { /* telemetry failure must never affect the harness */ }
  }
  pi.on('session_start', (_e, ctx) => { queue = []; usage = {}; seedUsage(ctx); emit('session', 'ready', ctx); });
  pi.on('agent_start', (_e, ctx) => emit('turn', 'working', ctx));
  pi.on('agent_end', (_e, ctx) => { seedUsage(ctx); emit('idle', 'idle', ctx); });
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
    usage = measuredUsage(e.message,true);
    emit('output', 'output', ctx);
  });
  pi.on('session_before_compact', (_e, ctx) => emit('compact-start', 'compacting', ctx));
  pi.on('session_compact', (_e, ctx) => { usage = {}; emit('compact-end', 'working', ctx); });
  pi.on('session_compact_failed', (e, ctx) => emit('compact-failed', 'working', ctx,
    { result: e.aborted === true ? 'cancelled' : 'error' }));
  pi.on('model_select', (_e, ctx) => { usage = {}; emit('model', 'ready', ctx); });
  pi.on('session_shutdown', (_e, ctx) => { emit('end', 'ended', ctx); });
}
