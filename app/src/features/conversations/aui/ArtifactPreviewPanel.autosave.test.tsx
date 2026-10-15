import { act, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { callCoreRpc } from '../../../services/coreRpcClient';
import { AUTOSAVE_DELAY_MS, useArtifactAutosave } from './ArtifactPreviewPanel';

vi.mock('../../../services/coreRpcClient', () => ({
  callCoreRpc: vi.fn().mockResolvedValue({ artifact_id: 'a-1', version: 2, saved: true }),
}));
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
vi.mock('../../../lib/i18n/I18nContext', () => ({
  useT: () => ({ t: (k: string) => k, locale: 'en' }),
}));

const rpc = vi.mocked(callCoreRpc);

describe('useArtifactAutosave', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    rpc.mockClear();
  });
  afterEach(() => vi.useRealTimers());

  it('saves once, 5 s after the last edit', async () => {
    const { result } = renderHook(() => useArtifactAutosave('a-1', '# v1', 1, true));

    act(() => result.current.schedule('# v1 a'));
    await act(async () => vi.advanceTimersByTime(AUTOSAVE_DELAY_MS - 1_000));
    // Typing again restarts the wait.
    act(() => result.current.schedule('# v1 ab'));
    await act(async () => vi.advanceTimersByTime(AUTOSAVE_DELAY_MS - 1));
    expect(rpc).not.toHaveBeenCalled();
    expect(result.current.status).toBe('pending');

    await act(async () => vi.advanceTimersByTime(1));
    expect(rpc).toHaveBeenCalledTimes(1);
    expect(rpc).toHaveBeenCalledWith({
      method: 'openhuman.ai_save_artifact_version',
      params: { artifact_id: 'a-1', body: '# v1 ab' },
    });
    expect(result.current.version).toBe(2);
    expect(result.current.status).toBe('saved');
  });

  it('flush saves a pending edit at once (before a download)', async () => {
    const { result } = renderHook(() => useArtifactAutosave('a-1', '# v1', 1, true));
    act(() => result.current.schedule('# edited'));
    await act(async () => result.current.flush());
    expect(rpc).toHaveBeenCalledTimes(1);
    // Nothing left to save when the timer would have fired.
    await act(async () => vi.advanceTimersByTime(AUTOSAVE_DELAY_MS));
    expect(rpc).toHaveBeenCalledTimes(1);
  });

  it('saves a pending edit when the panel closes', async () => {
    const { result, unmount } = renderHook(() => useArtifactAutosave('a-1', '# v1', 1, true));
    act(() => result.current.schedule('# edited'));
    await act(async () => unmount());
    expect(rpc).toHaveBeenCalledTimes(1);
  });

  it('never saves a read-only artifact', async () => {
    const { result } = renderHook(() => useArtifactAutosave('a-1', 'text', 1, false));
    act(() => result.current.schedule('changed'));
    await act(async () => vi.advanceTimersByTime(AUTOSAVE_DELAY_MS * 2));
    expect(rpc).not.toHaveBeenCalled();
  });
});
