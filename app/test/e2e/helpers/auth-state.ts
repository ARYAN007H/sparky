/** Auth state returned by `openhuman.auth_get_state`. */
export interface AuthStateResponse {
  isAuthenticated: boolean;
  userId?: string | null;
  credential?: 'session' | 'api-key' | 'local';
}

/** Read the auth DTO from the core RPC response. */
export function authStateValue(state: { result?: unknown }): AuthStateResponse {
  const raw = (state.result ?? {}) as { value?: AuthStateResponse } & AuthStateResponse;
  return raw.value ?? raw;
}
