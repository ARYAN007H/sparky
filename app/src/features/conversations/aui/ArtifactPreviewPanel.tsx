'use client';

import {
  Check,
  Clipboard,
  Code2,
  Download,
  Eye,
  FileBox,
  FileText,
  Save,
  X,
} from 'lucide-react';
import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';

import { Button } from '../../../components/ui';
import { useT } from '../../../lib/i18n/I18nContext';
import { useAppSelector } from '../../../store/hooks';
import type { ArtifactSnapshot } from '../../../store/chatRuntimeSlice';
import { invoke } from '@tauri-apps/api/core';

// ─── Types ──────────────────────────────────────────────────────

interface ArtifactPreviewPanelProps {
  /** If set, panel opens scoped to this single artifact. */
  artifactId: string;
  onClose: () => void;
}

interface ArtifactBodyResponse {
  kind: string;
  title: string;
  version: number;
  body: string;
}

// ─── Constants ──────────────────────────────────────────────────

const MIN_WIDTH = 360;
const MAX_WIDTH = 1200;
const DEFAULT_WIDTH = 520;

// ─── Main panel ─────────────────────────────────────────────────

export function ArtifactPreviewPanel({ artifactId, onClose }: ArtifactPreviewPanelProps) {
  const { t } = useT();
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

  // Active tab — start with the requested artifact, or the first one
  const [activeId, setActiveId] = useState(artifactId);
  const activeArtifact = useMemo(
    () => allArtifacts.find(a => a.artifactId === activeId),
    [allArtifacts, activeId],
  );

  // Keep activeId in sync if the parent passes a new artifactId
  useEffect(() => { setActiveId(artifactId); }, [artifactId]);

  // ── Resize ──
  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    dragRef.current = true;
    startXRef.current = e.clientX;
    startWidthRef.current = panelWidth;
    document.body.style.cursor = 'ew-resize';
    document.body.style.userSelect = 'none';
  }, [panelWidth]);

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
    return () => { window.removeEventListener('mousemove', onMove); window.removeEventListener('mouseup', onUp); };
  }, []);

  const showTabs = allArtifacts.length > 1;

  return (
    <div
      className="relative flex flex-col border-l bg-background shadow-xl h-full"
      style={{ width: panelWidth, minWidth: MIN_WIDTH, maxWidth: MAX_WIDTH }}
    >
      {/* Drag handle */}
      <div
        className="absolute left-0 top-0 bottom-0 w-1.5 cursor-ew-resize hover:bg-primary/20 active:bg-primary/30 z-20"
        onMouseDown={handleMouseDown}
      />

      {/* Header */}
      <div className="flex items-center justify-between border-b shrink-0">
        <div className="flex items-center gap-1 min-w-0 flex-1 overflow-x-auto">
          {showTabs ? (
            allArtifacts.map(a => (
              <button
                key={a.artifactId}
                onClick={() => setActiveId(a.artifactId)}
                className={`flex items-center gap-1.5 px-3 py-2.5 text-xs font-medium border-b-2 transition-colors shrink-0 ${
                  a.artifactId === activeId
                    ? 'border-primary text-foreground'
                    : 'border-transparent text-muted-foreground hover:text-foreground'
                }`}
              >
                <FileText className="h-3.5 w-3.5" />
                <span className="truncate max-w-[120px]">{a.title}</span>
              </button>
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
        <Button variant="ghost" size="xs" onClick={onClose} className="mr-2 shrink-0">
          <X className="h-4 w-4" />
        </Button>
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
    return () => { cancelled = true; };
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

  // Binary / file-reference kinds → FileViewer (metadata + download)
  if (kind === 'file' || kind === 'image') {
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

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center justify-between border-b px-3 py-1.5 shrink-0 bg-muted/10">
        <div className="flex items-center gap-2">
          <FileBox className="h-4 w-4 text-muted-foreground" />
          <span className="text-xs font-medium text-muted-foreground capitalize">{artifact.kind}</span>
          <span className="text-[11px] text-muted-foreground/60 border-l pl-2">v{version}</span>
        </div>
      </div>

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
            <a
              href={`file://${filePath}`}
              download={fileName}
              className="inline-flex items-center gap-2 rounded-md bg-primary px-4 py-2 text-xs font-medium text-primary-foreground hover:bg-primary/90"
            >
              <Download className="h-3.5 w-3.5" />
              Download
            </a>
          )}
        </div>
      </div>
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
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [copied, setCopied] = useState(false);

  // Sync when body changes (switching artifact tabs)
  useEffect(() => {
    setEditedBody(body);
    setSaved(false);
  }, [body]);

  const handleSave = useCallback(async () => {
    setSaving(true);
    try {
      await invoke('save_artifact_body', {
        args: {
          artifact_id: artifact.artifactId,
          body: editedBody,
          note: null,
        },
      });
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } catch (err) {
      console.error('[ArtifactPreviewPanel] save failed:', err);
    }
    setSaving(false);
  }, [artifact.artifactId, editedBody]);

  const handleCopy = useCallback(() => {
    void navigator.clipboard.writeText(editedBody);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }, [editedBody]);

  const isHtml = kind === 'html';

  return (
    <div className="flex flex-col h-full">
      {/* Toolbar */}
      <div className="flex items-center justify-between border-b px-3 py-1.5 shrink-0 bg-muted/10">
        <div className="flex items-center gap-2">
          <span className="text-xs font-medium text-muted-foreground capitalize">{kind}</span>
          <span className="text-[11px] text-muted-foreground/60 border-l pl-2">v{version}</span>
        </div>
        <div className="flex items-center gap-0.5">
          <div className="flex items-center rounded border mr-2">
            <button
              onClick={() => setTab('preview')}
              className={`flex items-center gap-1 px-2 py-0.5 text-xs ${
                tab === 'preview'
                  ? 'bg-primary/10 text-primary'
                  : 'text-muted-foreground hover:text-foreground'
              }`}
            >
              <Eye className="h-3 w-3" />
              Preview
            </button>
            <button
              onClick={() => setTab('code')}
              className={`flex items-center gap-1 px-2 py-0.5 text-xs ${
                tab === 'code'
                  ? 'bg-primary/10 text-primary'
                  : 'text-muted-foreground hover:text-foreground'
              }`}
            >
              <Code2 className="h-3 w-3" />
              Code
            </button>
          </div>

          <Button variant="ghost" size="xs" onClick={handleCopy} title="Copy to clipboard">
            {copied ? <Check className="h-3.5 w-3.5 text-green-500" /> : <Clipboard className="h-3.5 w-3.5" />}
          </Button>
          <Button variant="ghost" size="xs" onClick={handleSave} disabled={saving} title="Save as new version">
            {saving ? (
              <div className="h-3.5 w-3.5 animate-spin rounded-full border-2 border-primary border-t-transparent" />
            ) : saved ? (
              <Check className="h-3.5 w-3.5 text-green-500" />
            ) : (
              <Save className="h-3.5 w-3.5" />
            )}
          </Button>
        </div>
      </div>

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
          onChange={e => setEditedBody(e.target.value)}
          className="flex-1 w-full resize-none border-0 bg-background p-4 text-sm font-mono leading-relaxed outline-none focus:ring-0"
          spellCheck={false}
        />
      )}
    </div>
  );
}

export default ArtifactPreviewPanel;
