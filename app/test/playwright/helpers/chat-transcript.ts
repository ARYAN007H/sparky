import { expect, type Page } from '@playwright/test';

import { callCoreRpc } from './core-rpc';
import { waitForSelectedThreadId } from './chat-drive';

export function threadTranscriptPath(threadId: string): string {
  const encoded = Array.from(new TextEncoder().encode(threadId))
    .map(byte => byte.toString(16).padStart(2, '0'))
    .join('');
  return `memory/conversations/threads/${encoded}.jsonl`;
}

function unwrapResult(value: unknown): unknown {
  let current = value;
  for (let depth = 0; depth < 3; depth += 1) {
    if (!current || typeof current !== 'object' || !('result' in current)) return current;
    current = (current as { result: unknown }).result;
  }
  return current;
}

export async function selectedThreadTranscript(page: Page): Promise<{
  threadId: string;
  path: string;
  content: string;
}> {
  const threadId = await waitForSelectedThreadId(page);
  const relativePath = threadTranscriptPath(threadId);
  const file = unwrapResult(
    await callCoreRpc<unknown>('openhuman.test_support_read_workspace_file', {
      rel_path: relativePath,
      max_bytes: 131_072,
    })
  );
  expect(file && typeof file === 'object' && 'content_utf8' in file, `transcript exists at ${relativePath}`).toBe(true);
  const content = (file as { content_utf8: unknown }).content_utf8;
  expect(typeof content, 'transcript file content is UTF-8 text').toBe('string');
  return { threadId, path: relativePath, content: content as string };
}

export async function listedThreadTranscripts(): Promise<
  Array<{ rel_path: string; size: number; is_dir: boolean }>
> {
  const listing = unwrapResult(
    await callCoreRpc<unknown>('openhuman.test_support_list_workspace_files', {
      rel_root: 'memory/conversations/threads',
      max_depth: 1,
    })
  );
  expect(listing && typeof listing === 'object' && 'entries' in listing, 'workspace file list exists').toBe(true);
  const entries = (listing as { entries: unknown }).entries;
  expect(Array.isArray(entries), 'workspace file list contains entries').toBe(true);
  return entries as Array<{ rel_path: string; size: number; is_dir: boolean }>;
}

export async function inFlightThreadKeys(): Promise<string[]> {
  const status = unwrapResult(
    await callCoreRpc<unknown>('openhuman.test_support_in_flight_chats', {})
  );
  expect(status && typeof status === 'object' && 'entries' in status, 'in-flight test support returns entries').toBe(true);
  const entries = (status as { entries: unknown }).entries;
  expect(Array.isArray(entries), 'in-flight entries are a list').toBe(true);
  return (entries as Array<{ key: string }>).map(entry => entry.key);
}
