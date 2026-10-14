'use client';

import { X, GripVertical } from 'lucide-react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';

import { Button } from '../../../components/ui';
import { useT } from '../../../lib/i18n/I18nContext';
import { useAppSelector } from '../../../store/hooks';
import type { ArtifactSnapshot } from '../../../store/chatRuntimeSlice';

interface ArtifactPreviewPanelProps {
  artifactId: string;
  onClose: () => void;
}

const MIN_WIDTH = 320;
const MAX_WIDTH = 960;
const DEFAULT_WIDTH = 480;

export function ArtifactPreviewPanel({ artifactId, onClose }: ArtifactPreviewPanelProps) {
  const { t } = useT();
  const [panelWidth, setPanelWidth] = useState(DEFAULT_WIDTH);
  const dragRef = useRef(false);
  const startXRef = useRef(0);
  const startWidthRef = useRef(0);

  const artifactsByThread = useAppSelector(state => state.chatRuntime.artifactsByThread);

  const artifact = useMemo<ArtifactSnapshot | undefined>(() => {
    for (const threadArtifacts of Object.values(artifactsByThread)) {
      const found = threadArtifacts.find(a => a.artifactId === artifactId);
      if (found) return found;
    }
    return undefined;
  }, [artifactsByThread, artifactId]);

  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    dragRef.current = true;
    startXRef.current = e.clientX;
    startWidthRef.current = panelWidth;
    document.body.style.cursor = 'ew-resize';
    document.body.style.userSelect = 'none';
  }, [panelWidth]);

  useEffect(() => {
    const handleMouseMove = (e: MouseEvent) => {
      if (!dragRef.current) return;
      const delta = startXRef.current - e.clientX;
      const newWidth = Math.min(MAX_WIDTH, Math.max(MIN_WIDTH, startWidthRef.current + delta));
      setPanelWidth(newWidth);
    };
    const handleMouseUp = () => {
      if (!dragRef.current) return;
      dragRef.current = false;
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
    };
    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
    return () => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };
  }, []);

  return (
    <div
      className="relative flex flex-col border-l bg-background shadow-xl h-full"
      style={{ width: panelWidth, minWidth: MIN_WIDTH, maxWidth: MAX_WIDTH }}
    >
      {/* Drag handle */}
      <div
        className="absolute left-0 top-0 bottom-0 w-1.5 cursor-ew-resize hover:bg-primary/20 active:bg-primary/30 z-10"
        onMouseDown={handleMouseDown}
      />

      {/* Header */}
      <div className="flex items-center justify-between border-b px-4 py-3 shrink-0 pl-5">
        <div className="flex items-center gap-2 min-w-0">
          <div className="flex flex-col gap-0.5 min-w-0">
            <span className="text-sm font-medium truncate">
              {artifact?.title || t('chat.artifact.preview')}
            </span>
            {artifact && (
              <span className="text-xs text-muted-foreground capitalize">{artifact.kind}</span>
            )}
          </div>
        </div>
        <Button variant="ghost" size="xs" onClick={onClose} className="shrink-0">
          <X className="h-4 w-4" />
        </Button>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto p-4">
        {!artifact ? (
          <p className="text-sm text-muted-foreground">{t('chat.artifact.notFound')}</p>
        ) : artifact.status === 'ready' ? (
          <ArtifactContent artifact={artifact} />
        ) : (
          <div className="flex items-center justify-center h-full">
            <p className="text-sm text-muted-foreground">
              {artifact.status === 'in_progress' ? t('conversations.tools.working') : t('chat.artifact.failed')}
            </p>
          </div>
        )}
      </div>
    </div>
  );
}

/** Renders artifact content based on kind. */
function ArtifactContent({ artifact }: { artifact: ArtifactSnapshot }) {
  // For now show metadata. In a full implementation, this would:
  // 1. Fetch the artifact body from the versioned store or file
  // 2. Render HTML in an iframe, text/markdown in a <pre>, etc.
  const fileUrl = artifact.path ? `/api/files/${artifact.path}` : null;

  return (
    <div className="space-y-4">
      <div className="rounded-lg border bg-muted/30 p-4 space-y-2">
        <MetaRow label="ID" value={artifact.artifactId} />
        <MetaRow label="Status" value={artifact.status} />
        <MetaRow label="Kind" value={artifact.kind} />
        {artifact.sizeBytes != null && (
          <MetaRow label="Size" value={formatBytes(artifact.sizeBytes)} />
        )}
        {artifact.path && <MetaRow label="File" value={artifact.path} mono />}
      </div>

      {/* Content area - placeholder for now */}
      {artifact.kind === 'html' && <HtmlPreview html={artifact as never} />}
      {(artifact.kind === 'markdown' || artifact.kind === 'text' || artifact.kind === 'document') && (
        <TextPreview artifact={artifact} />
      )}
    </div>
  );
}

function MetaRow({ label, value, mono }: { label: string; value: string; mono?: boolean }) {
  return (
    <div className="flex items-start gap-2 text-sm">
      <span className="text-muted-foreground shrink-0 w-12">{label}:</span>
      <span className={`break-all ${mono ? 'font-mono text-xs' : ''}`}>{value}</span>
    </div>
  );
}

function HtmlPreview({ html }: { html: string }) {
  return (
    <div className="rounded-lg border overflow-hidden">
      <iframe
        srcDoc={html}
        className="w-full h-[400px]"
        sandbox="allow-same-origin"
        title="HTML Preview"
      />
    </div>
  );
}

function TextPreview({ artifact }: { artifact: ArtifactSnapshot }) {
  return (
    <div className="rounded-lg border">
      <div className="border-b bg-muted/30 px-3 py-2 text-xs font-medium text-muted-foreground">
        Content preview
      </div>
      <pre className="p-4 text-sm whitespace-pre-wrap break-words max-h-[500px] overflow-y-auto font-mono">
        {/* Placeholder: would fetch actual content from versioned store */}
        {artifact.path ? `Load content from: ${artifact.path}` : 'No content available'}
      </pre>
    </div>
  );
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export default ArtifactPreviewPanel;
