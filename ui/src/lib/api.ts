import { invoke, Channel } from '@tauri-apps/api/core';

export function apiError(error: unknown): Error & { status?: number } {
  if (error instanceof Error) return error;
  const detail = error as { status?: number; message?: string } | null;
  return Object.assign(new Error(detail?.message || String(error || 'Request failed')), {
    status: detail?.status,
  });
}

export async function api<T>(method: 'GET' | 'POST' | 'PUT' | 'DELETE', path: string, body?: object): Promise<T> {
  try {
    return await invoke<T>('api', { method, path, body: body ?? {} });
  } catch (error) {
    throw apiError(error);
  }
}

export async function chatStream(body: object, onPiece: (piece: string) => void): Promise<string> {
  const ch = new Channel<string>();
  ch.onmessage = onPiece;
  try {
    return await invoke<string>('chat_stream', { body, onPiece: ch });
  } catch (error) {
    throw apiError(error);
  }
}

export function updateCommand<T>(command: 'update_status' | 'update_check' | 'update_install'): Promise<T> {
  return invoke<T>(command);
}
export function updateState(state: string, manualError: boolean): string {
  return state === 'error' && !manualError ? '' : state;
}
