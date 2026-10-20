import { expect, type Page } from '@playwright/test';

import { callCoreRpc } from './core-rpc';
import { waitForSelectedThreadId } from './chat-drive';

export async function selectedThreadTranscript(page: Page): Promise<{
  threadId: string;
  content: string;
  total: number;
}> {
  const threadId = await waitForSelectedThreadId(page);
  const response = await callCoreRpc<unknown>('openhuman.threads_transcript_get', {
    thread_id: threadId,
  });
  expect(response && typeof response === 'object' && 'data' in response).toBe(true);
  const data = (response as { data: unknown }).data;
  expect(data && typeof data === 'object' && 'hasTranscript' in data).toBe(true);
  expect((data as { hasTranscript: unknown }).hasTranscript).toBe(true);
  expect(Array.isArray((data as { items: unknown }).items)).toBe(true);
  return {
    threadId,
    content: JSON.stringify(data),
    total: (data as { total: number }).total,
  };
}

export async function inFlightThreadKeys(): Promise<string[]> {
  const response = await callCoreRpc<unknown>('openhuman.test_support_in_flight_chats', {});
  let status = response;
  for (let depth = 0; depth < 3; depth += 1) {
    if (!status || typeof status !== 'object' || !('result' in status)) break;
    status = (status as { result: unknown }).result;
  }
  expect(status && typeof status === 'object' && 'entries' in status).toBe(true);
  const entries = (status as { entries: unknown }).entries;
  expect(Array.isArray(entries)).toBe(true);
  return (entries as Array<{ key: string }>).map(entry => entry.key);
}
