'use client';

import { X } from 'lucide-react';
import { useMemo } from 'react';

import { Button } from '../../../components/ui';
import { useT } from '../../../lib/i18n/I18nContext';
import { useAppSelector } from '../../../store/hooks';
import type { ArtifactSnapshot } from '../../../store/chatRuntimeSlice';

interface ArtifactPreviewPanelProps {
  artifactId: string;
  onClose: () => void;
}

export function ArtifactPreviewPanel({ artifactId, onClose }: ArtifactPreviewPanelProps) {
  const { t } = useT();
  const artifactsByThread = useAppSelector(state => state.chatRuntime.artifactsByThread);
  
  const artifact = useMemo<ArtifactSnapshot | undefined>(() => {
    for (const threadArtifacts of Object.values(artifactsByThread)) {
      const found = threadArtifacts.find(a => a.artifactId === artifactId);
      if (found) return found;
    }
    return undefined;
  }, [artifactsByThread, artifactId]);

  if (!artifact) {
    return (
      <div className="fixed inset-y-0 right-0 z-50 w-[480px] border-l bg-background shadow-xl">
        <div className="flex items-center justify-between border-b px-4 py-3">
          <span className="text-sm font-medium">{t('chat.artifact.preview')}</span>
          <Button variant="ghost" size="xs" onClick={onClose}><X className="h-4 w-4" /></Button>
        </div>
        <div className="p-4 text-sm text-muted-foreground">{t('chat.artifact.notFound')}</div>
      </div>
    );
  }

  return (
    <div className="fixed inset-y-0 right-0 z-50 w-[480px] border-l bg-background shadow-xl flex flex-col">
      <div className="flex items-center justify-between border-b px-4 py-3 shrink-0">
        <div className="flex flex-col gap-0.5">
          <span className="text-sm font-medium">{artifact.title}</span>
          <span className="text-xs text-muted-foreground">{artifact.kind}</span>
        </div>
        <Button variant="ghost" size="xs" onClick={onClose}><X className="h-4 w-4" /></Button>
      </div>
      <div className="flex-1 overflow-y-auto p-4">
        <div className="rounded-lg border bg-muted/30 p-4">
          <p className="text-sm text-muted-foreground">
            Artifact ID: {artifact.artifactId}
          </p>
          <p className="text-sm text-muted-foreground">
            Status: {artifact.status}
          </p>
          {artifact.sizeBytes != null && (
            <p className="text-sm text-muted-foreground">
              Size: {formatBytes(artifact.sizeBytes)}
            </p>
          )}
          {artifact.path && (
            <p className="text-sm text-muted-foreground break-all mt-2">
              Path: {artifact.path}
            </p>
          )}
        </div>
      </div>
    </div>
  );
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export default ArtifactPreviewPanel;
