import { configureStore } from '@reduxjs/toolkit';
import { invoke } from '@tauri-apps/api/core';
import { fireEvent, render, screen } from '@testing-library/react';
import { Provider } from 'react-redux';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import chatRuntimeReducer, {
  openArtifact,
  upsertArtifactReadyForThread,
} from '../../../store/chatRuntimeSlice';
import { ArtifactPreviewPanel } from './ArtifactPreviewPanel';

vi.mock('../../../lib/i18n/I18nContext', () => ({
  useT: () => ({ t: (k: string) => k, locale: 'en' }),
}));

const MD = '# Hello\n\nThis is **bold** text.\n';

function renderPanel(kind: string, body: string, failWith?: string) {
  // Stub the body fetch BEFORE mounting so the panel's effect resolves it.
  if (failWith) vi.mocked(invoke).mockRejectedValue(failWith);
  else
    vi.mocked(invoke).mockResolvedValue({ kind, title: 'Notes', version: 1, body, is_text: true });
  const store = configureStore({ reducer: { chatRuntime: chatRuntimeReducer } });
  store.dispatch(
    upsertArtifactReadyForThread({
      threadId: 't-1',
      artifactId: 'a-1',
      kind: 'document',
      title: 'Notes',
      path: '/tmp/n.md',
      sizeBytes: body.length,
    })
  );
  store.dispatch(openArtifact('a-1'));
  render(
    <Provider store={store}>
      <ArtifactPreviewPanel />
    </Provider>
  );
}

describe('ArtifactPreviewPanel — preview rendering by kind (#1535)', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
  });

  it('renders markdown as formatted content, not raw source', async () => {
    renderPanel('markdown', MD);

    const h1 = await screen.findByRole('heading', { level: 1 });
    expect(h1.textContent).toBe('Hello');

    // **bold** becomes <strong>, not literal asterisks
    expect(screen.getByText('bold').tagName).toBe('STRONG');

    // The raw markdown source must NOT be shown verbatim.
    expect(screen.queryByText(/# Hello/)).toBeNull();
  });

  it('renders GFM constructs (tables) via remark-gfm', async () => {
    renderPanel('markdown', '| a | b |\n| --- | --- |\n| one | two |\n');

    const table = await screen.findByRole('table');
    expect(table).toBeTruthy();
    expect(screen.getByText('one')).toBeTruthy();
  });

  it('keeps rendering HTML kind in the sandboxed iframe (unchanged)', async () => {
    renderPanel('html', '<h1>Hi</h1><p>body</p>');

    const frame = await screen.findByTitle('HTML preview');
    expect(frame.tagName).toBe('IFRAME');
    expect(frame.getAttribute('sandbox')).toBe('allow-scripts');
    expect(frame.getAttribute('srcdoc')).toContain('<h1>Hi</h1>');
  });

  it('keeps the editor read-only when the body failed to load', async () => {
    renderPanel('markdown', '', 'artifact unreadable');

    expect(await screen.findByText('artifact unreadable')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: /Code/ }));
    // Typing here would save the empty text over the artifact.
    expect((screen.getByRole('textbox') as HTMLTextAreaElement).readOnly).toBe(true);
  });

  it('still renders plain text kind as raw preformatted source', async () => {
    renderPanel('text', 'plain **not** markdown\n');

    const pre = await screen.findByText(/plain \*\*not\*\* markdown/);
    expect(pre.tagName).toBe('PRE');
    expect(screen.queryByRole('heading')).toBeNull();
  });
});
