import { useEffect } from 'react';

import { listArtifactsForThread } from '../../../services/artifactDownloadService';
import { upsertArtifactReadyForThread } from '../../../store/chatRuntimeSlice';
import { useAppDispatch } from '../../../store/hooks';

/**
 * Load a thread's ready artifacts from the core into
 * `chatRuntime.artifactsByThread` when the thread is opened (#3024), so the
 * artifact cards, the preview panel and the `@` mention list have them after
 * a restart or on a thread the app has not seen yet.
 *
 * Where each card renders comes from the thread's artifact rows, not from
 * here (`services/api/artifactRows.ts`).
 */
export function useThreadArtifactsHydration(threadId: string | null | undefined): void {
  const dispatch = useAppDispatch();
  const normalizedThreadId = threadId?.trim() ?? '';

  useEffect(() => {
    if (!normalizedThreadId) return;
    let cancelled = false;
    void listArtifactsForThread(normalizedThreadId).then(outcome => {
      if (cancelled) return;
      if (!outcome.ok) {
        console.warn('[artifact] failed to hydrate thread artifacts', {
          threadId: normalizedThreadId,
          error: outcome.error,
        });
        return;
      }
      if (outcome.artifacts.length === 0) return;
      console.debug('[artifact] hydrating thread artifacts', {
        threadId: normalizedThreadId,
        count: outcome.artifacts.length,
      });
      for (const artifact of outcome.artifacts) {
        dispatch(
          upsertArtifactReadyForThread({
            threadId: normalizedThreadId,
            artifactId: artifact.artifactId,
            kind: artifact.kind,
            title: artifact.title,
            path: artifact.path,
            sizeBytes: artifact.sizeBytes,
          })
        );
      }
    });
    return () => {
      cancelled = true;
    };
  }, [dispatch, normalizedThreadId]);
}
