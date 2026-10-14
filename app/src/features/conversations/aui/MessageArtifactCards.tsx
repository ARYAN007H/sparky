'use client';

import { useAuiState } from '@assistant-ui/react';
import { useMemo } from 'react';

import { useAppSelector, useAppDispatch } from '../../../store/hooks';
import { openArtifact } from '../../../store/chatRuntimeSlice';
import { ArtifactCardAdapter } from './ArtifactCardAdapter';

/**
 * Renders artifact cards inside the exact assistant message that produced
 * them. Matches the current message's `metadata.custom.requestId` against
 * each artifact's `ownerRequestId` (tagged by `chatRuntimeSlice` when the
 * pending/ready socket event lands while that turn is live).
 *
 * Only artifacts WITHOUT an owning tool call (`!toolCallId`) are shown
 * here — ones WITH a tool call id already render inline next to their
 * tool call via `MediaAndDocumentCalls`.
 */
export function MessageArtifactCards() {
  // requestId lives at different paths for live vs settled messages:
  // - live streaming message: metadata.custom.requestId
  // - settled message: metadata.custom.extraMetadata.requestId
  const meta = useAuiState(s => s.message.metadata?.custom as Record<string, unknown> | undefined);
  const requestId = typeof meta?.requestId === 'string' ? meta.requestId
    : typeof (meta?.extraMetadata as Record<string, unknown> | undefined)?.requestId === 'string'
      ? (meta?.extraMetadata as Record<string, unknown> | undefined)?.requestId as string
      : undefined;
  const threadId = useAppSelector(s => s.thread.selectedThreadId as string | undefined);
  const dispatch = useAppDispatch();

  const artifactsByThread = useAppSelector(s => s.chatRuntime.artifactsByThread);
  const cards = useMemo(() => {
    if (!threadId || !requestId) return [];
    const list = artifactsByThread[threadId] ?? [];
    return list.filter(
      a => !a.toolCallId && a.ownerRequestId === requestId && (a.status === 'in_progress' || a.status === 'ready')
    );
  }, [artifactsByThread, threadId, requestId]);

  if (cards.length === 0) return null;

  return (
    <div className="mt-3 flex flex-col gap-2" data-testid="message-artifact-cards">
      {cards.map(artifact => (
        <ArtifactCardAdapter
          key={artifact.artifactId}
          artifact={artifact}
          onOpenPreview={id => dispatch(openArtifact(id))}
        />
      ))}
    </div>
  );
}

export default MessageArtifactCards;
