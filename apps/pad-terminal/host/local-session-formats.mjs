// Read-only adapters for local history. Content is rendered as text, never executed.
const TEXT_LIMIT = 12_000;
const clip = (text) => text.length > TEXT_LIMIT ? text.slice(0, TEXT_LIMIT) + '\n[预览已截断，完整内容保留在原会话]' : text;
export function contentText(content) {
  if (typeof content === 'string') return clip(content);
  if (!Array.isArray(content)) return '';
  return clip(content.map((part) => {
    if (typeof part?.text === 'string') return clip(part.text);
    if (part?.type === 'tool_use' || part?.type === 'toolCall') return `[${part.name ?? 'tool'}]\n${clip(JSON.stringify(part.input ?? part.arguments ?? {}))}`;
    if (part?.type === 'tool_result') return contentText(part.content);
    if (part?.type === 'image' || part?.type === 'input_image') return '[图片：保留在原会话中]';
    return '';
  }).filter(Boolean).join('\n'));
}

export function parseLines(text) {
  const entries = [];
  for (const line of text.split('\n')) {
    try { const e = JSON.parse(line); if (e && typeof e === 'object') entries.push(e); } catch { /* partial append / malformed record */ }
  }
  return entries;
}

export function activeBranch(entries) {
  const byId = new Map(entries.filter((e) => e.type !== 'session' && typeof e.id === 'string').map((e) => [e.id, e]));
  if (!byId.size) return entries; // v1 linear history
  const seen = new Set(), branch = [];
  let entry = [...byId.values()].at(-1);
  while (entry && !seen.has(entry.id)) {
    seen.add(entry.id); branch.push(entry);
    entry = byId.get(entry.parentId);
  }
  return branch.reverse();
}

export function sessionMessages(tool, entries) {
  const messages = [], seen = new Set();
  const hasCodexResponses = entries.some((e) => e.type === 'response_item' && e.payload?.type === 'message');
  const selected = tool === 'pi' ? activeBranch(entries) : entries;
  for (const [index, e] of selected.entries()) {
    let message;
    if (tool === 'pi') {
      if (e.type === 'message') message = e.message;
      if (e.type === 'custom_message' && e.display !== false) message = { role: 'notice', content: e.content };
      if (e.type === 'compaction' || e.type === 'branch_summary') message = { role: 'notice', content: e.summary };
    } else if (tool === 'claude') {
      if (e.type === 'user' || e.type === 'assistant') message = e.message;
    } else if (e.type === 'response_item') {
      const p = e.payload;
      if (p?.type === 'message') message = p;
      if (p?.type === 'function_call' || p?.type === 'custom_tool_call') message = { role: 'tool', content: `[${p.name ?? 'tool'}]\n${p.arguments ?? p.input ?? ''}` };
      if (p?.type === 'function_call_output' || p?.type === 'custom_tool_call_output') message = { role: 'tool', content: contentText(p.output) };
    } else if (!hasCodexResponses && e.type === 'event_msg') {
      if (e.payload?.type === 'user_message') message = { role: 'user', content: e.payload.message };
      if (e.payload?.type === 'agent_message') message = { role: 'assistant', content: e.payload.message };
    }
    if (message?.role === 'bashExecution') message = { role: 'tool', content: `$ ${message.command ?? ''}\n${message.output ?? ''}` };
    if (message?.role === 'custom' && message.display !== false) message = { ...message, role: 'notice' };
    if (message?.role === 'user' && Array.isArray(message.content) && message.content.every((p) => p.type === 'tool_result')) message = { ...message, role: 'tool' };
    if (!message || !['user', 'assistant', 'tool', 'toolResult', 'notice'].includes(message.role)) continue;
    const text = contentText(message.content);
    if (!text) continue;
    const id = String(e.uuid ?? e.id ?? `line-${index}`);
    if (seen.has(id)) continue;
    seen.add(id);
    messages.push({ id, role: message.role === 'toolResult' ? 'tool' : message.role, text });
  }
  return messages;
}

export function sessionMetadata(tool, entries) {
  let sessionId, cwd, title;
  for (const e of entries) {
    if (tool === 'pi' && e.type === 'session') { sessionId = e.id; cwd = e.cwd; }
    if (tool === 'pi' && e.type === 'session_info' && e.name) title = e.name;
    if (tool === 'codex' && e.type === 'session_meta') { sessionId = e.payload?.id; cwd = e.payload?.cwd; }
    if (tool === 'codex' && e.type === 'event_msg' && e.payload?.type === 'thread_name_updated') title = e.payload.thread_name;
    if (tool === 'claude') {
      sessionId ??= e.sessionId;
      cwd ??= e.cwd;
      if (e.type === 'custom-title' && e.customTitle) title = e.customTitle;
    }
  }
  if (!title) {
    title = sessionMessages(tool, entries).find((m) => m.role === 'user'
      && !/^(# (AGENTS|Instructions)|<environment|<system|<local-command|<command-name)/i.test(m.text))?.text;
  }
  return {
    sessionId: typeof sessionId === 'string' ? sessionId.slice(0, 200) : '',
    cwd: typeof cwd === 'string' && cwd.startsWith('/') ? cwd.slice(0, 4096) : '',
    title: typeof title === 'string' ? title.replace(/\s+/g, ' ').trim().slice(0, 160) : '',
  };
}
