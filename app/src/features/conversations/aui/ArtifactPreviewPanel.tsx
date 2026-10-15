'use client';

import { invoke } from '@tauri-apps/api/core';
import { Check, Clipboard, Code2, Download, Eye, FileBox, FileText, Share2, X } from 'lucide-react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import Markdown from 'react-markdown';
import remarkGfm from 'remark-gfm';

import {
  hasWindowDragBar,
  WINDOW_DRAG_BAR_HEIGHT,
} from '../../../components/layout/shell/WindowDragBar';
import { useT } from '../../../lib/i18n/I18nContext';
import { callCoreRpc } from '../../../services/coreRpcClient';
import type { ArtifactSnapshot } from '../../../store/chatRuntimeSlice';
import {
  activateArtifactTab,
  closeArtifactPanel,
  closeArtifactTab,
} from '../../../store/chatRuntimeSlice';
import { useAppDispatch, useAppSelector } from '../../../store/hooks';

// ─── Types ──────────────────────────────────────────────────────

interface ArtifactPreviewPanelProps {
  /** No props: the panel reads openArtifactIds/activeArtifactId from Redux. */
}

interface ArtifactBodyResponse {
  kind: string;
  title: string;
  version: number;
  body: string;
  is_text: boolean;
}

// ─── Constants ──────────────────────────────────────────────────

const MIN_WIDTH = 360;
const MAX_WIDTH = 1200;
const DEFAULT_WIDTH = 520;

// ─── Main panel ─────────────────────────────────────────────────

export function ArtifactPreviewPanel(_props: ArtifactPreviewPanelProps) {
  const { t } = useT();
  const dispatch = useAppDispatch();
  const [panelWidth, setPanelWidth] = useState(DEFAULT_WIDTH);
  const dragRef = useRef(false);
  const startXRef = useRef(0);
  const startWidthRef = useRef(0);

  // All artifacts for the current thread — we show them as tabs
  const artifactsByThread = useAppSelector(s => s.chatRuntime.artifactsByThread);
  const allArtifacts = useMemo(() => {
    const list: ArtifactSnapshot[] = [];
    for (const arr of Object.values(artifactsByThread)) {
      for (const a of arr) {
        if (a.status === 'ready') list.push(a);
      }
    }
    return list.sort((a, b) => (b.updatedAt ?? 0) - (a.updatedAt ?? 0));
  }, [artifactsByThread]);

  // Active tab and open-tab list come from Redux — single source of truth.
  const openArtifactIds = useAppSelector(s => s.chatRuntime.openArtifactIds);
  const activeId = useAppSelector(s => s.chatRuntime.activeArtifactId);
  const activeArtifact = useMemo(
    () => allArtifacts.find(a => a.artifactId === activeId),
    [allArtifacts, activeId]
  );

  // ── Resize ──
  const handleMouseDown = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      dragRef.current = true;
      startXRef.current = e.clientX;
      startWidthRef.current = panelWidth;
      document.body.style.cursor = 'ew-resize';
      document.body.style.userSelect = 'none';
    },
    [panelWidth]
  );

  useEffect(() => {
    const onMove = (e: MouseEvent) => {
      if (!dragRef.current) return;
      const delta = startXRef.current - e.clientX;
      setPanelWidth(Math.min(MAX_WIDTH, Math.max(MIN_WIDTH, startWidthRef.current + delta)));
    };
    const onUp = () => {
      if (!dragRef.current) return;
      dragRef.current = false;
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
    };
    window.addEventListener('mousemove', onMove);
    window.addEventListener('mouseup', onUp);
    return () => {
      window.removeEventListener('mousemove', onMove);
      window.removeEventListener('mouseup', onUp);
    };
  }, []);

  // Tabs = the artifacts currently open in the panel, in open order.
  const visibleTabs = useMemo(
    () =>
      openArtifactIds
        .map(id => allArtifacts.find(a => a.artifactId === id))
        .filter((a): a is ArtifactSnapshot => a !== undefined),
    [openArtifactIds, allArtifacts]
  );
  const handleCloseTab = useCallback(
    (id: string) => {
      dispatch(closeArtifactTab(id));
    },
    [dispatch]
  );

  const showTabs = visibleTabs.length > 1;

  return (
    <div
      className="relative flex flex-col border-l bg-background shadow-xl h-full"
      style={{
        width: panelWidth,
        minWidth: MIN_WIDTH,
        maxWidth: MAX_WIDTH,
        // The panel header is at the top of the window, under the window-drag
        // band, which would otherwise swallow clicks on the tabs and the X.
        paddingTop: hasWindowDragBar() ? WINDOW_DRAG_BAR_HEIGHT : undefined,
      }}>
      {/* Drag handle */}
      <div
        className="absolute left-0 top-0 bottom-0 w-1.5 cursor-ew-resize hover:bg-primary/20 active:bg-primary/30 z-20"
        onMouseDown={handleMouseDown}
      />

      {/* Header */}
      <div className="flex items-center justify-between border-b shrink-0">
        <div className="flex items-center gap-1 min-w-0 flex-1 overflow-x-auto">
          {showTabs ? (
            visibleTabs.map(a => (
              <div
                key={a.artifactId}
                className={`group flex items-center gap-1 border-b-2 px-2 py-2.5 transition-colors shrink-0 min-w-0 ${
                  a.artifactId === activeId ? 'border-primary' : 'border-transparent'
                }`}>
                <button
                  onClick={() => dispatch(activateArtifactTab(a.artifactId))}
                  className={`flex min-w-0 items-center gap-1.5 text-xs font-medium transition-colors ${
                    a.artifactId === activeId
                      ? 'text-foreground'
                      : 'text-muted-foreground hover:text-foreground'
                  }`}>
                  <FileText className="h-3.5 w-3.5 shrink-0" />
                  <span className="truncate max-w-[110px]">{a.title}</span>
                </button>
                <button
                  onClick={e => {
                    e.stopPropagation();
                    handleCloseTab(a.artifactId);
                  }}
                  className="ml-0.5 rounded p-0.5 text-muted-foreground/50 hover:bg-accent hover:text-foreground shrink-0"
                  title="Close artifact">
                  <X className="h-3 w-3" />
                </button>
              </div>
            ))
          ) : (
            <div className="flex items-center gap-2 px-4 py-2.5 min-w-0">
              <FileText className="h-4 w-4 shrink-0 text-muted-foreground" />
              <span className="text-sm font-medium truncate">
                {activeArtifact?.title || t('chat.artifact.preview')}
              </span>
            </div>
          )}
        </div>
        <button
          onClick={() => dispatch(closeArtifactPanel())}
          title="Close panel"
          className="mr-2 shrink-0 inline-flex items-center justify-center rounded p-1 text-muted-foreground hover:bg-accent hover:text-foreground">
          <X className="h-4 w-4" />
        </button>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-hidden">
        {activeArtifact ? (
          <ArtifactContentPanel artifact={activeArtifact} />
        ) : (
          <div className="flex items-center justify-center h-full p-4 text-sm text-muted-foreground">
            {t('chat.artifact.notFound')}
          </div>
        )}
      </div>
    </div>
  );
}

// ─── Content panel (fetches and renders) ────────────────────────

function ArtifactContentPanel({ artifact }: { artifact: ArtifactSnapshot }) {
  const [bodyResult, setBodyResult] = useState<ArtifactBodyResponse | null>(null);
  useEffect(() => {
    // Clear when switching artifacts
    setBodyResult(null);
    setLoading(true);
    setError(null);
  }, [artifact.artifactId]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!artifact.artifactId) {
      setLoading(false);
      return;
    }
    let cancelled = false;
    setLoading(true);
    setError(null);

    const fetchBody = async () => {
      try {
        const result = await invoke<ArtifactBodyResponse>('read_artifact_body', {
          artifactId: artifact.artifactId,
        });
        if (!cancelled) setBodyResult(result);
      } catch (err) {
        if (!cancelled) setError(String(err));
      }
      if (!cancelled) setLoading(false);
    };

    fetchBody();
    return () => {
      cancelled = true;
    };
  }, [artifact.artifactId]);

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full p-8">
        <div className="h-5 w-5 animate-spin rounded-full border-2 border-primary border-t-transparent" />
      </div>
    );
  }

  const kind = bodyResult?.kind || artifact.kind;
  const body = bodyResult?.body || '';
  const version = bodyResult?.version || 1;
  const isText = bodyResult?.is_text ?? false;

  // File kind whose body is actual readable text → editable viewer.
  // Truly binary files (body stays a path) → FileViewer (metadata + download).
  if (kind === 'file' || kind === 'image') {
    if (isText) {
      return (
        <EditableTextViewer
          artifact={artifact}
          body={body}
          version={version}
          kind={kind}
          error={error}
        />
      );
    }
    return <FileViewer artifact={artifact} body={body} version={version} error={error} />;
  }

  // Text-based kinds → EditableTextViewer (preview + code editor + save)
  return (
    <EditableTextViewer
      artifact={artifact}
      body={body}
      version={version}
      kind={kind}
      error={error}
    />
  );
}

// ─── File viewer (binary / file-reference kinds) ────────────────

function FileViewer({
  artifact,
  body,
  version,
  error,
}: {
  artifact: ArtifactSnapshot;
  body: string;
  version: number;
  error: string | null;
}) {
  const filePath = body || artifact.path || '';
  const fileName = filePath.split('/').pop() || filePath;
  // Share is hidden until the backend serves viewable files inline (links download today); see useArtifactShare.
  // const sharing = useArtifactShare(artifact.artifactId);

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center justify-between border-b px-3 py-1.5 shrink-0 bg-muted/10">
        <div className="flex items-center gap-2">
          <FileBox className="h-4 w-4 text-muted-foreground" />
          <span className="text-xs font-medium text-muted-foreground capitalize">
            {artifact.kind}
          </span>
          <span className="text-[11px] text-muted-foreground/60 border-l pl-2">v{version}</span>
        </div>
        {/* <ShareButton state={sharing.state} onShare={() => void sharing.share()} /> */}
      </div>
      {/* <ShareStatus state={sharing.state} onCopy={sharing.copyAgain} /> */}

      <div className="flex-1 overflow-auto p-6">
        <div className="flex flex-col items-center justify-center gap-4 text-center">
          <FileBox className="h-12 w-12 text-muted-foreground/40" />
          <div>
            <p className="text-sm font-medium break-all">{fileName}</p>
            <p className="text-xs text-muted-foreground mt-1 break-all">{filePath}</p>
            <p className="text-xs text-muted-foreground mt-2">
              Binary {artifact.kind} — preview not available
            </p>
            {error && <p className="text-xs text-destructive mt-2">{error}</p>}
          </div>
          {filePath && (
            <button
              onClick={() => void saveArtifactAs(artifact)}
              className="inline-flex items-center gap-2 rounded-md bg-primary px-4 py-2 text-xs font-medium text-primary-foreground hover:bg-primary/90">
              <Download className="h-3.5 w-3.5" />
              Download
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

// ─── Download and autosave ──────────────────────────────────────

/** Versioned kinds whose record holds the content, so an edit is a version. */
const EDITABLE_KINDS = new Set(['text', 'markdown', 'html']);

/** Wait after the last keystroke before saving the edit as a new version. */
export const AUTOSAVE_DELAY_MS = 5_000;

/** The artifact's file name: the one it has in the files folder. */
function downloadName(artifact: ArtifactSnapshot): string {
  const fromPath = artifact.path?.split(/[\\/]/).pop();
  return fromPath || artifact.title;
}

/** Let the user pick where to save the artifact's file (OS save dialog). */
async function saveArtifactAs(artifact: ArtifactSnapshot): Promise<void> {
  try {
    await invoke<string | null>('save_artifact_as', {
      artifactId: artifact.artifactId,
      filename: downloadName(artifact),
    });
  } catch (err) {
    console.error('[ArtifactPreviewPanel] download failed:', err);
  }
}

type AutosaveStatus = 'saved' | 'pending' | 'saving' | 'error';

/**
 * Save edits as new operator versions, AUTOSAVE_DELAY_MS after the last
 * change. A pending edit is saved at once when the artifact changes, the
 * panel closes, or a download needs the newest file. The core appends nothing
 * when the text matches the newest version.
 */
export function useArtifactAutosave(
  artifactId: string,
  body: string,
  initialVersion: number,
  enabled: boolean
) {
  const [status, setStatus] = useState<AutosaveStatus>('saved');
  const [version, setVersion] = useState(initialVersion);
  const pending = useRef<string | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    setVersion(initialVersion);
    setStatus('saved');
  }, [artifactId, body, initialVersion]);

  const save = useCallback(async () => {
    if (timer.current) clearTimeout(timer.current);
    timer.current = null;
    const next = pending.current;
    pending.current = null;
    if (next === null) return;
    setStatus('saving');
    try {
      const result = await callCoreRpc<{ version: number }>({
        method: 'openhuman.ai_save_artifact_version',
        params: { artifact_id: artifactId, body: next },
      });
      if (typeof result?.version === 'number') setVersion(result.version);
      setStatus(pending.current === null ? 'saved' : 'pending');
    } catch (err) {
      console.error('[ArtifactPreviewPanel] autosave failed:', err);
      setStatus('error');
    }
  }, [artifactId]);

  const schedule = useCallback(
    (next: string) => {
      if (!enabled) return;
      pending.current = next;
      setStatus('pending');
      if (timer.current) clearTimeout(timer.current);
      timer.current = setTimeout(() => void save(), AUTOSAVE_DELAY_MS);
    },
    [enabled, save]
  );

  // Save a pending edit when switching artifacts or closing the panel.
  useEffect(() => () => void save(), [save]);

  return useMemo(
    () => ({ status, version, schedule, flush: save }),
    [status, version, schedule, save]
  );
}

function AutosaveStatus({ status }: { status: AutosaveStatus }) {
  const label = {
    saved: 'Saved',
    pending: 'Unsaved changes',
    saving: 'Saving…',
    error: 'Not saved',
  }[status];
  return (
    <span
      className={`text-[11px] border-l pl-2 ${
        status === 'error' ? 'text-destructive' : 'text-muted-foreground/60'
      }`}>
      {label}
    </span>
  );
}

// ─── Share ──────────────────────────────────────────────────────

interface ShareResult {
  url: string;
  version: number;
  public: boolean;
  expires_at_millis?: number | null;
  reused: boolean;
}

type ShareState =
  | { status: 'idle' }
  | { status: 'sharing' }
  | { status: 'shared'; url: string; version: number; expiresAt: number | null; copied: boolean }
  | { status: 'error'; message: string };

/**
 * Share the artifact's newest version through cloud file storage
 * (`ai.share_artifact`, which uploads it and returns a link — reused while it
 * still works) and copy the link. `beforeShare` runs first, e.g. to save a
 * pending edit so the newest text is what is shared.
 */
export function useArtifactShare(artifactId: string, beforeShare?: () => Promise<void>) {
  const [state, setState] = useState<ShareState>({ status: 'idle' });

  useEffect(() => {
    setState({ status: 'idle' });
  }, [artifactId]);

  const copy = useCallback(async (url: string) => {
    try {
      await navigator.clipboard.writeText(url);
      return true;
    } catch {
      return false;
    }
  }, []);

  const share = useCallback(async () => {
    setState({ status: 'sharing' });
    try {
      await beforeShare?.();
      const result = await callCoreRpc<ShareResult>({
        method: 'openhuman.ai_share_artifact',
        params: { artifact_id: artifactId },
      });
      const copied = await copy(result.url);
      setState({
        status: 'shared',
        url: result.url,
        version: result.version,
        expiresAt: result.expires_at_millis ?? null,
        copied,
      });
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      console.error('[ArtifactPreviewPanel] share failed:', err);
      setState({ status: 'error', message });
    }
  }, [artifactId, beforeShare, copy]);

  const copyAgain = useCallback(async () => {
    if (state.status !== 'shared') return false;
    const copied = await copy(state.url);
    setState({ ...state, copied });
    return copied;
  }, [copy, state]);

  return { state, share, copyAgain };
}

export function ShareButton({ state, onShare }: { state: ShareState; onShare: () => void }) {
  return (
    <button
      onClick={onShare}
      disabled={state.status === 'sharing'}
      title="Share: upload and copy a link"
      className="inline-flex items-center justify-center rounded p-1 text-muted-foreground hover:bg-accent hover:text-foreground disabled:opacity-50">
      {state.status === 'sharing' ? (
        <div className="h-3.5 w-3.5 animate-spin rounded-full border-2 border-primary border-t-transparent" />
      ) : (
        <Share2 className="h-3.5 w-3.5" />
      )}
    </button>
  );
}

/** How long the Copy button says "Copied" after a click. */
const COPIED_FEEDBACK_MS = 2000;

export function ShareStatus({
  state,
  onCopy,
}: {
  state: ShareState;
  onCopy: () => Promise<boolean>;
}) {
  // The button's own feedback: only for a click on it, not for the copy the
  // share already made (the status text reports that one).
  const [justCopied, setJustCopied] = useState(false);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  useEffect(
    () => () => {
      if (timer.current) clearTimeout(timer.current);
    },
    []
  );
  const handleCopy = useCallback(async () => {
    const copied = await onCopy();
    if (!copied) return;
    setJustCopied(true);
    if (timer.current) clearTimeout(timer.current);
    timer.current = setTimeout(() => setJustCopied(false), COPIED_FEEDBACK_MS);
  }, [onCopy]);

  if (state.status === 'error') {
    return (
      <div
        data-testid="artifact-share-status"
        className="border-b bg-destructive/10 px-3 py-1.5 text-[11px] text-destructive">
        Not shared: {state.message}
      </div>
    );
  }
  if (state.status !== 'shared') return null;
  const expires = state.expiresAt
    ? ` · expires ${new Date(state.expiresAt).toLocaleDateString()}`
    : '';
  return (
    <div
      data-testid="artifact-share-status"
      className="flex items-center gap-2 border-b bg-muted/20 px-3 py-1.5 text-[11px] text-muted-foreground">
      <span className="shrink-0">
        {state.copied ? 'Link copied' : 'Link ready'} · v{state.version}
        {expires}
      </span>
      <span className="min-w-0 flex-1 truncate text-foreground/70" title={state.url}>
        {state.url}
      </span>
      <button
        onClick={() => void handleCopy()}
        title="Copy the full link"
        data-testid="artifact-share-copy"
        className="inline-flex shrink-0 items-center gap-1 rounded border px-1.5 py-0.5 text-[11px] text-foreground hover:bg-accent">
        {justCopied ? (
          <Check className="h-3 w-3 text-green-500" />
        ) : (
          <Clipboard className="h-3 w-3" />
        )}
        {justCopied ? 'Copied' : 'Copy'}
      </button>
    </div>
  );
}

// ─── Editable text viewer (markdown / text / html) ──────────────

function EditableTextViewer({
  artifact,
  body,
  version,
  kind,
  error,
}: {
  artifact: ArtifactSnapshot;
  body: string;
  version: number;
  kind: string;
  error: string | null;
}) {
  const [tab, setTab] = useState<'preview' | 'code'>('preview');
  const [editedBody, setEditedBody] = useState(body);
  const [copied, setCopied] = useState(false);
  // Only kinds whose record holds the content itself can take a new version;
  // a `file` record holds a path, and its text here is read off that file.
  const editable = EDITABLE_KINDS.has(kind);
  const autosave = useArtifactAutosave(artifact.artifactId, body, version, editable);

  // Sync when body changes (switching artifact tabs)
  useEffect(() => {
    setEditedBody(body);
  }, [body]);

  const onEdit = useCallback(
    (next: string) => {
      setEditedBody(next);
      autosave.schedule(next);
    },
    [autosave]
  );

  const handleCopy = useCallback(() => {
    void navigator.clipboard.writeText(editedBody);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }, [editedBody]);

  const handleDownload = useCallback(async () => {
    // The file is the newest saved version: save a pending edit first.
    await autosave.flush();
    await saveArtifactAs(artifact);
  }, [artifact, autosave]);

  // Share the newest saved version: save a pending edit first.
  // Share is hidden until the backend serves viewable files inline (links download today); see useArtifactShare.
  // const sharing = useArtifactShare(artifact.artifactId, autosave.flush);

  const isHtml = kind === 'html';
  const isMarkdown = kind === 'markdown';

  return (
    <div className="flex flex-col h-full">
      {/* Toolbar */}
      <div className="flex items-center justify-between border-b px-3 py-1.5 shrink-0 bg-muted/10">
        <div className="flex items-center gap-2">
          <span className="text-xs font-medium text-muted-foreground capitalize">{kind}</span>
          <span className="text-[11px] text-muted-foreground/60 border-l pl-2">
            v{autosave.version}
          </span>
          {editable && <AutosaveStatus status={autosave.status} />}
        </div>
        <div className="flex items-center gap-0.5">
          <div className="flex items-center rounded border mr-2">
            <button
              onClick={() => setTab('preview')}
              className={`flex items-center gap-1 px-2 py-0.5 text-xs ${
                tab === 'preview'
                  ? 'bg-primary/10 text-primary'
                  : 'text-muted-foreground hover:text-foreground'
              }`}>
              <Eye className="h-3 w-3" />
              Preview
            </button>
            <button
              onClick={() => setTab('code')}
              className={`flex items-center gap-1 px-2 py-0.5 text-xs ${
                tab === 'code'
                  ? 'bg-primary/10 text-primary'
                  : 'text-muted-foreground hover:text-foreground'
              }`}>
              <Code2 className="h-3 w-3" />
              Code
            </button>
          </div>

          <button
            onClick={handleCopy}
            title="Copy to clipboard"
            className="inline-flex items-center justify-center rounded p-1 text-muted-foreground hover:bg-accent hover:text-foreground">
            {copied ? (
              <Check className="h-3.5 w-3.5 text-green-500" />
            ) : (
              <Clipboard className="h-3.5 w-3.5" />
            )}
          </button>
          {/* <ShareButton state={sharing.state} onShare={() => void sharing.share()} /> */}
          <button
            onClick={() => void handleDownload()}
            title="Download"
            className="inline-flex items-center justify-center rounded p-1 text-muted-foreground hover:bg-accent hover:text-foreground">
            <Download className="h-3.5 w-3.5" />
          </button>
        </div>
      </div>

      {/* <ShareStatus state={sharing.state} onCopy={sharing.copyAgain} /> */}

      {/* Error banner */}
      {error && (
        <div className="border-b bg-destructive/10 px-3 py-1.5 text-[11px] text-destructive">
          {error}
        </div>
      )}

      {/* Content area */}
      {tab === 'preview' ? (
        isHtml ? (
          <iframe
            srcDoc={editedBody}
            className="w-full flex-1 border-0"
            sandbox="allow-scripts allow-same-origin"
            title="HTML preview"
          />
        ) : isMarkdown ? (
          // Markdown renders as formatted content (headings, lists, tables,
          // emphasis) just like HTML renders in the iframe above — not as a
          // raw source dump.
          <div className="flex-1 overflow-auto">
            <div className="p-4 prose prose-sm dark:prose-invert max-w-none">
              <Markdown remarkPlugins={[remarkGfm]}>{editedBody}</Markdown>
            </div>
          </div>
        ) : (
          <div className="flex-1 overflow-auto">
            <pre className="p-4 text-sm font-mono leading-relaxed whitespace-pre-wrap break-words">
              {editedBody}
            </pre>
          </div>
        )
      ) : (
        <textarea
          value={editedBody}
          onChange={e => onEdit(e.target.value)}
          readOnly={!editable}
          className="flex-1 w-full resize-none border-0 bg-background p-4 text-sm font-mono leading-relaxed outline-none focus:ring-0"
          spellCheck={false}
        />
      )}
    </div>
  );
}

export default ArtifactPreviewPanel;
