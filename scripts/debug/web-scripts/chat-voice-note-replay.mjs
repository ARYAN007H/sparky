const TRANSCRIPT = 'Please remind me about the dentist';
const REPLY = 'I will remind you about the dentist.';

function silentWavBase64() {
  const sampleRate = 8000;
  const samples = new Uint8Array(sampleRate * 5);
  const bytes = new Uint8Array(44 + samples.length);
  const view = new DataView(bytes.buffer);
  const tag = (offset, value) =>
    [...value].forEach((char, index) => (bytes[offset + index] = char.charCodeAt(0)));
  tag(0, 'RIFF');
  view.setUint32(4, 36 + samples.length, true);
  tag(8, 'WAVE');
  tag(12, 'fmt ');
  view.setUint32(16, 16, true);
  view.setUint16(20, 1, true);
  view.setUint16(22, 1, true);
  view.setUint32(24, sampleRate, true);
  view.setUint32(28, sampleRate, true);
  view.setUint16(32, 1, true);
  view.setUint16(34, 8, true);
  tag(36, 'data');
  view.setUint32(40, samples.length, true);
  bytes.set(samples, 44);
  return Buffer.from(bytes).toString('base64');
}

const AUDIO_B64 = silentWavBase64();

export default async function voiceNoteReplay({ page, mock, log, screenshot }) {
  page.on('response', async response => {
    const body = response.request().postData() ?? '';
    if (body.includes('channel_web_voice_note_read')) {
      let result = '<unreadable>';
      try { result = JSON.stringify(await response.json(), (_key, value) => typeof value === 'string' && value.length > 256 ? `[string:${value.length}]` : value); } catch {}
      log(`voice note read RPC ${body}: ${result}`);
    }
  });
  mock.set('audioTranscriptionText', TRANSCRIPT);
  mock.set('audioSpeechBase64', AUDIO_B64);
  mock.set('audioSpeechMime', 'audio/wav');
  mock.set('audioSpeechVisemes', [
    { viseme: 'aa', start_ms: 0, end_ms: 180 },
    { viseme: 'PP', start_ms: 180, end_ms: 420 },
  ]);
  mock.set('llmStreamScript', [{ text: REPLY }, { finish: 'stop' }]);

  await page.goto(new URL('/#/chat', page.url()).toString());
  const composer = page.getByRole('textbox', { name: 'Message input' });
  await composer.waitFor({ state: 'visible', timeout: 30_000 });
  let threadId = await page.evaluate(
    () => window.__OPENHUMAN_STORE__?.getState?.().thread?.selectedThreadId ?? null,
  );
  if (!threadId) {
    await page.getByRole('button', { name: /new conversation/i }).click();
    threadId = await page.waitForFunction(
      () => window.__OPENHUMAN_STORE__?.getState?.().thread?.selectedThreadId,
      null,
      { timeout: 15_000 },
    ).then(handle => handle.jsonValue());
  }
  await page.waitForFunction(
    id => window.__OPENHUMAN_STORE__?.getState?.().thread?.selectedThreadId === id,
    threadId,
    { timeout: 10_000 },
  );
  if (!threadId) throw new Error('new chat thread was not selected');

  await page.evaluate(wav => {
    const raw = Uint8Array.from(atob(wav), char => char.charCodeAt(0));
    Object.defineProperty(navigator, 'mediaDevices', {
      configurable: true,
      value: { getUserMedia: async () => ({ getTracks: () => [{ stop() {} }] }) },
    });
    window.MediaRecorder = class {
      static isTypeSupported() { return true; }
      constructor() { this.state = 'inactive'; this.mimeType = 'audio/wav'; this.listeners = {}; }
      addEventListener(type, fn) { this.listeners[type] = fn; }
      start() { this.state = 'recording'; }
      stop() {
        this.state = 'inactive';
        this.listeners.dataavailable?.({ data: new Blob([raw], { type: 'audio/wav' }) });
        this.listeners.stop?.();
      }
    };
  }, AUDIO_B64);

  await page.getByTestId('voice-note-record').click();
  await page.getByTestId('voice-note-recording').waitFor({ state: 'visible' });
  await page.getByTestId('voice-note-stop').click();
  await page.getByTestId('voice-note-preview').waitFor({ state: 'visible' });
  await page.getByTestId('voice-note-send').click();
  await page.waitForTimeout(3_000);
  await page.evaluate(id => window.__OPENHUMAN_STORE__?.dispatch({ type: 'thread/setSelectedThread', payload: id }), threadId);
  log(`selected thread ${threadId}; user player count=${await page.getByTestId('voice-note-player').count()}`);
  const transcriptToggle = page.getByTestId('voice-note-transcript-toggle').first();
  if (await transcriptToggle.count()) await transcriptToggle.click();
  const state = await page.evaluate(id => {
    const state = window.__OPENHUMAN_STORE__?.getState?.();
    const messages = state?.thread?.messagesByThreadId?.[id] ?? [];
    return messages.map(message => ({ id: message.id, sender: message.sender, content: message.content, extraMetadata: message.extraMetadata }));
  }, threadId);
  log(`selected thread messages: ${JSON.stringify(state)}`);
  await page.getByText(TRANSCRIPT, { exact: false }).waitFor({ timeout: 30_000 });
  await page.getByText(REPLY, { exact: false }).waitFor({ timeout: 90_000 });

  await page.getByTestId('voice-note-player').nth(1).waitFor({ state: 'visible', timeout: 90_000 });
  const requests = mock.requests().map(request => request.url);
  for (const expected of ['/openai/v1/audio/transcriptions', '/openai/v1/audio/speech']) {
    if (!requests.includes(expected)) throw new Error(`mock request missing ${expected}: ${requests.join(', ')}`);
  }
  await page.reload();
  await page.getByRole('textbox', { name: 'Message input' }).waitFor({ state: 'visible', timeout: 30_000 });
  await page.evaluate(id => window.__OPENHUMAN_STORE__?.dispatch({ type: 'thread/setSelectedThread', payload: id }), threadId);
  await page.getByTestId('voice-note-player').nth(1).waitFor({ state: 'visible', timeout: 30_000 });
  await page.getByText(REPLY, { exact: false }).waitFor({ timeout: 10_000 });
  await screenshot('voice-note-reloaded');

  await page.evaluate(() => {
    const NativeAudio = window.Audio;
    window.__E2E_AUDIO_ELEMENTS = [];
    window.Audio = function (source) {
      const audio = new NativeAudio(source);
      window.__E2E_AUDIO_ELEMENTS.push(audio);
      return audio;
    };
    window.Audio.prototype = NativeAudio.prototype;
  });
  const toggles = page.getByTestId('voice-note-toggle');
  if (await toggles.count() !== 2) throw new Error(`expected two voice-note players, found ${await toggles.count()}`);
  for (let index = 0; index < 2; index += 1) {
    await toggles.nth(index).click();
    try { await page.waitForFunction(i => {
      const audio = window.__E2E_AUDIO_ELEMENTS?.[i];
      return audio && audio.readyState >= 2 && Number.isFinite(audio.duration) && audio.duration > 0;
    }, index, { timeout: 10_000 }); } catch (error) {
      log(`audio ${index} state: ${JSON.stringify(await page.evaluate(i => { const a = window.__E2E_AUDIO_ELEMENTS?.[i]; return a ? { src: a.src, readyState: a.readyState, networkState: a.networkState, duration: a.duration, error: a.error?.code } : null; }, index))}`);
      throw error;
    }
    await page.waitForFunction(i => window.__E2E_AUDIO_ELEMENTS?.[i]?.paused === false, index, { timeout: 5_000 });
    await toggles.nth(index).click();
    log(`reloaded voice note ${index + 1} decoded and played`);
  }
  log(`recorded and sent voice note; transcription and speech mock routes called; thread ${threadId} reloaded`);
}
