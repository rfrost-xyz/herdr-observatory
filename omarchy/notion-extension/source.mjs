// Only this module knows Notion's unsupported web response shapes.
const uuid = value => typeof value === 'string' && /^[0-9a-f]{8}(-[0-9a-f]{4}){3}-[0-9a-f]{12}$/.test(value);
export function binding(value) {
  if (!value || value.version !== 1 || !uuid(value.user_id) || !uuid(value.workspace_id)) throw new Error('Notion setup needed');
  return {version: 1, user_id: value.user_id, workspace_id: value.workspace_id};
}
function record(value) {
  for (let i = 0; i < 2 && value?.value; i++) value = value.value;
  return value;
}
export function verifyIdentity(spaces, config) {
  const account = spaces?.[config.user_id];
  const user = record(account?.notion_user?.[config.user_id]);
  const space = record(account?.space?.[config.workspace_id]);
  if (user?.id !== config.user_id || space?.id !== config.workspace_id) throw new Error('Notion account unavailable');
}
export function observation(raw, config, now) {
  const monthly = raw?.billingPeriodWindow;
  const finite = n => typeof n === 'number' && Number.isFinite(n);
  if (!['within_limit', 'rate_limited', 'over_limit'].includes(raw?.status)
      || monthly?.creditType !== 'basic_ai_credits' || monthly?.scope !== 'per_user'
      || monthly?.cadence !== 'billing_period'
      || !finite(monthly.used) || monthly.used < 0 || monthly.used > 9e15
      || !finite(monthly.limit) || monthly.limit <= 0 || monthly.limit > 9e15
      || !finite(monthly.used / monthly.limit * 100) || monthly.used / monthly.limit * 100 > 1e6
      || !Number.isSafeInteger(monthly.periodEndMs) || monthly.periodEndMs <= now * 1000
      || monthly.periodEndMs > (now + 32 * 86400) * 1000) throw new Error('Notion monthly allowance unavailable');
  return {...binding(config), sampled_at: now, available: true, used: monthly.used, limit: monthly.limit, resets_at: Math.floor(monthly.periodEndMs / 1000)};
}
export async function request(endpoint, body, user, fetcher = fetch) {
  if (!['getSpaces', 'getCreditRateLimitStatus'].includes(endpoint)) throw new Error('Unsupported Notion request');
  const control = new AbortController();
  const timer = setTimeout(() => control.abort(), 8000);
  try {
    const response = await fetcher('https://app.notion.com/api/v3/' + endpoint, {
      method: 'POST', credentials: 'include', redirect: 'error', cache: 'no-store', signal: control.signal,
      headers: {'Content-Type': 'application/json', 'x-notion-active-user-header': user}, body: JSON.stringify(body)
    });
    if (!response.ok || !response.body) throw new Error('Notion source unavailable');
    const reader = response.body.getReader();
    let size = 0, chunks = [];
    try {
      while (true) {
        const {value, done} = await reader.read();
        if (done) break;
        size += value.byteLength;
        if (size > 1024 * 1024) throw new Error('Notion response too large');
        chunks.push(value);
      }
    } finally { await reader.cancel().catch(() => {}); }
    const joined = new Uint8Array(size); let offset = 0;
    for (const chunk of chunks) { joined.set(chunk, offset); offset += chunk.length; }
    return JSON.parse(new TextDecoder('utf-8', {fatal: true}).decode(joined));
  } finally { clearTimeout(timer); }
}
export async function refresh(native, fetcher = fetch, clock = () => Date.now() / 1000) {
  const config = binding(await native({version: 1, operation: 'configuration'}));
  const started = clock();
  let value;
  try {
    verifyIdentity(await request('getSpaces', {}, config.user_id, fetcher), config);
    value = observation(await request('getCreditRateLimitStatus', {spaceId: config.workspace_id}, config.user_id, fetcher), config, started);
  } catch {
    value = {...config, sampled_at: started, available: false, used: null, limit: null, resets_at: null};
  }
  const receipt = await native(value);
  if (receipt?.ok !== true) throw new Error('Anton receiver unavailable');
  return value.available;
}
