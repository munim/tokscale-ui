import {
  ipc,
  onSessionTitlesUpdated as subscribeSessionTitlesUpdated,
  type CompanionPreferences,
  type SessionDetail,
  type SessionTitlesUpdatedEvent,
  type SessionsPayload,
  type SessionsQuery,
  type SyncMode,
  type SyncStatus,
  type UnlistenFn,
} from '$lib/contracts/ipc';

export function getSessionsView(query: SessionsQuery): Promise<SessionsPayload> {
  return ipc.getSessionsView(query);
}

export function getSessionDetail(client: string, sessionId: string): Promise<SessionDetail> {
  return ipc.getSessionDetail(client, sessionId);
}

export function triggerSync(mode: SyncMode): Promise<SyncStatus> {
  return ipc.triggerSync(mode);
}

export function getPreferences(): Promise<CompanionPreferences> {
  return ipc.getPreferences();
}

export function updatePreferences(
  patch: Partial<CompanionPreferences>,
): Promise<CompanionPreferences> {
  return ipc.updatePreferences(patch);
}

export function onSessionTitlesUpdated(
  handler: (payload: SessionTitlesUpdatedEvent) => void,
): Promise<UnlistenFn> {
  return subscribeSessionTitlesUpdated(handler);
}
