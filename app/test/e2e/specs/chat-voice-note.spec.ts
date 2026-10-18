// @ts-nocheck
/**
 * Chat voice notes - a recorded note and a spoken assistant reply survive a
 * reload and stay replayable.
 *
 * The microphone and the hosted STT/TTS engines are not part of the mock
 * backend, so the two audio-producing steps are driven through the same core
 * RPCs the composer and the turn runner call:
 *
 *   - `channel_web_voice_note_store` is what the composer calls after a
 *     recording (user note) and what the turn runner calls with synthesized
 *     speech (assistant reply, stored under `reply-<request_id>`).
 *   - `threads_message_append` writes the rows with `extraMetadata.voiceNote`.
 *
 * What is asserted is the part users see: after a full app reload both rows
 * render a voice-note player, the user note offers its transcript, and the
 * core still serves the stored audio for playback.
 */
import { waitForApp } from '../helpers/app-helpers';
import { chatMounted, clickByTitle, getSelectedThreadId } from '../helpers/chat-harness';
import { callOpenhumanRpc } from '../helpers/core-rpc';
import { clickTestId, textExists, waitForTestId } from '../helpers/element-helpers';
import { resetApp } from '../helpers/reset-app';
import { navigateViaHash } from '../helpers/shared-flows';
import { startMockServer, stopMockServer } from '../mock-server';

const USER_ID = 'e2e-chat-voice-note';
const TRANSCRIPT = 'voice-note-canary remind me about the dentist';
const REPLY = 'Sure, I will remind you about the dentist.';
// "abc" - the store only cares that the bytes decode and are non-empty.
const AUDIO_B64 = 'YWJj';

describe('Chat voice notes', () => {
  let threadId: string;

  before(async function beforeSuite() {
    this.timeout(90_000);
    await startMockServer();
    await waitForApp();
    await resetApp(USER_ID);
  });

  after(async () => {
    await stopMockServer();
  });

  it('seeds a user voice note and a spoken assistant reply', async function () {
    this.timeout(60_000);
    await navigateViaHash('/chat');
    await browser.waitUntil(async () => await chatMounted(), { timeout: 15_000 });
    expect(await clickByTitle('New thread', 8_000)).toBe(true);
    threadId = (await browser.waitUntil(async () => await getSelectedThreadId(), {
      timeout: 8_000,
    })) as string;

    const userNote = await callOpenhumanRpc('openhuman.channel_web_voice_note_store', {
      note_id: 'vn_e2e_user',
      audio_base64: AUDIO_B64,
      mime: 'audio/webm',
    });
    expect(userNote.ok).toBe(true);
    const replyNote = await callOpenhumanRpc('openhuman.channel_web_voice_note_store', {
      note_id: 'reply-e2e-req',
      audio_base64: AUDIO_B64,
      mime: 'audio/mpeg',
    });
    expect(replyNote.ok).toBe(true);

    const now = new Date().toISOString();
    const user = await callOpenhumanRpc('openhuman.threads_message_append', {
      thread_id: threadId,
      message: {
        id: 'msg_e2e_voice_user',
        content: TRANSCRIPT,
        type: 'text',
        extraMetadata: { voiceNote: { noteId: 'vn_e2e_user', mime: 'audio/webm', durationMs: 4000 } },
        sender: 'user',
        createdAt: now,
      },
    });
    expect(user.ok).toBe(true);
    const agent = await callOpenhumanRpc('openhuman.threads_message_append', {
      thread_id: threadId,
      message: {
        id: 'agent:e2e-req',
        content: REPLY,
        type: 'text',
        extraMetadata: {
          scope: 'web_chat_reply',
          requestId: 'e2e-req',
          voiceNote: { noteId: 'reply-e2e-req', mime: 'audio/mpeg' },
        },
        sender: 'agent',
        createdAt: new Date(Date.now() + 1000).toISOString(),
      },
    });
    expect(agent.ok).toBe(true);
  });

  it('shows both players after a reload, with the transcript under the user note', async function () {
    this.timeout(90_000);
    await browser.refresh();
    await waitForApp();
    await navigateViaHash('/chat');
    await browser.waitUntil(async () => await chatMounted(), { timeout: 20_000 });
    await browser.execute(tid => {
      window.__OPENHUMAN_STORE__?.dispatch({ type: 'thread/setSelectedThread', payload: tid });
    }, threadId);

    await waitForTestId('voice-note-player', 20_000);
    const players = await browser.$$('[data-testid="voice-note-player"]');
    expect(players.length).toBe(2);

    // The assistant reply keeps its text; the user note shows it as a transcript.
    expect(await textExists(REPLY)).toBe(true);
    await clickTestId('voice-note-transcript-toggle');
    await browser.waitUntil(async () => await textExists(TRANSCRIPT), { timeout: 5_000 });
  });

  it('plays back from the stored audio and the core still serves it', async function () {
    this.timeout(30_000);
    await clickTestId('voice-note-toggle');
    const read = await callOpenhumanRpc('openhuman.channel_web_voice_note_read', {
      note_id: 'reply-e2e-req',
    });
    expect(read.ok).toBe(true);
    expect(read.result?.result?.audio_base64 ?? read.result?.audio_base64).toBe(AUDIO_B64);
  });
});
