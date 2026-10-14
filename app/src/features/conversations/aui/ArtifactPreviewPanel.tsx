'use client';

import {
  Check,
  Clipboard,
  Download,
  FileText,
  GripVertical,
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

interface VersionEntry {
  version: number;
  body: string;
  createdAt: string;
  note?: string;
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

      {/* Toolbar */}
      {activeArtifact && (
        <ArtifactToolbar artifact={activeArtifact} allArtifacts={allArtifacts} />
      )}

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        {activeArtifact ? (
          <ArtifactContent artifact={activeArtifact} />
        ) : (
          <div className="flex items-center justify-center h-full p-4 text-sm text-muted-foreground">
            {t('chat.artifact.notFound')}
          </div>
        )}
      </div>
    </div>
  );
}

// ─── Toolbar (version, copy, download) ──────────────────────────

function ArtifactToolbar({
  artifact,
  allArtifacts,
}: {
  artifact: ArtifactSnapshot;
  allArtifacts: ArtifactSnapshot[];
}) {
  const { t } = useT();
  const [copied, setCopied] = useState(false);

  // Find the current index among same-titled artifacts for version hint
  const sameTitle = allArtifacts.filter(a => a.title === artifact.title);
  const versionIndex = sameTitle.indexOf(artifact);

  const handleCopy = useCallback(() => {
    // In a full implementation, copy the artifact body
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }, []);

  const handleDownload = useCallback(() => {
    // In a full implementation, trigger file download via the file URL
  }, []);

  return (
    <div className="flex items-center justify-between border-b px-3 py-1.5 shrink-0 bg-muted/10">
      <div className="flex items-center gap-2">
        <span className="text-xs font-medium text-muted-foreground capitalize">{artifact.kind}</span>
        {versionIndex >= 0 && (
          <span className="text-[11px] text-muted-foreground/60 border-l pl-2">
            v{versionIndex + 1}
          </span>
        )}
      </div>
      <div className="flex items-center gap-1">
        <Button variant="ghost" size="xs" onClick={handleCopy} title={t('chat.artifact.copy')}>
          {copied ? <Check className="h-3.5 w-3.5 text-green-500" /> : <Clipboard className="h-3.5 w-3.5" />}
        </Button>
        <Button variant="ghost" size="xs" onClick={handleDownload} title={t('chat.artifact.download')}>
          <Download className="h-3.5 w-3.5" />
        </Button>
      </div>
    </div>
  );
}

// ─── Content renderer ──────────────────────────────────────────

function ArtifactContent({ artifact }: { artifact: ArtifactSnapshot }) {
  const [content, setContent] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [artifactKind, setArtifactKind] = useState<string>(artifact.kind);
  const [version, setVersion] = useState(1);

  // Fetch artifact body from versioned store via Tauri IPC
  useEffect(() => {
    if (!artifact.artifactId) {
      setLoading(false);
      return;
    }

    let cancelled = false;
    setLoading(true);

    const fetchBody = async () => {
      try {
        const result = await invoke<{ kind: string; title: string; version: number; body: string }>(
          'read_artifact_body',
          { artifactId: artifact.artifactId },
        );
        if (cancelled) return;
        setContent(result.body);
        setArtifactKind(result.kind);
        setVersion(result.version);
      } catch (err) {
        // Fallback: show a placeholder
        if (cancelled) return;
        setContent(`# ${artifact.title}\n\n*Kind: ${artifact.kind}\nPath: ${artifact.path || '—'}*`);
      }
      if (!cancelled) setLoading(false);
    };

    fetchBody();
    return () => { cancelled = true; };
  }, [artifact.artifactId, artifact.title, artifact.kind, artifact.path]);

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full p-8">
        <div className="h-5 w-5 animate-spin rounded-full border-2 border-primary border-t-transparent" />
      </div>
    );
  }

  const kind = artifactKind || artifact.kind;

  switch (kind) {
    case 'html':
      return <HtmlViewer content={content || ''} />;
    case 'markdown':
    case 'text':
    case 'document':
    default:
      return <TextViewer content={content || `# ${artifact.title}\n\n*Kind: ${artifact.kind}\nPath: ${artifact.path}*`} language={kind} />;
  }
}

// ─── HTML viewer ──────────────────────────────────────────────

function HtmlViewer({ content }: { content: string }) {
  return (
    <iframe
      srcDoc={content}
      className="w-full h-full border-0"
      sandbox="allow-scripts allow-same-origin"
      title="HTML preview"
    />
  );
}

// ─── Text / Markdown viewer ───────────────────────────────────

function TextViewer({ content, language }: { content: string; language: string }) {
  return (
    <div className="h-full flex flex-col">
      <div className="border-b bg-muted/20 px-3 py-1.5 text-[11px] font-medium text-muted-foreground flex items-center gap-2">
        <FileText className="h-3 w-3" />
        <span className="uppercase">{language}</span>
      </div>
      <pre className="flex-1 overflow-auto p-4 text-sm font-mono leading-relaxed whitespace-pre-wrap break-words">
        {content}
      </pre>
    </div>
  );
}

export default ArtifactPreviewPanel;
