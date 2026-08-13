// Shared API plumbing. `axios` imported anywhere under src/api is the panel's
// configured instance (externalized to window.axios): it carries the bearer
// token, credentials and route-change cancellation. Note it is an *instance*,
// so `axios.isAxiosError` does not exist — errors are unwrapped structurally.

export const PLUGIN_ID = 'files';
export const BASE = `/api/plugins/${PLUGIN_ID}`;

export interface ApiErrorBody {
  code?: string;
  message?: string;
}

interface ErrorShape {
  response?: {
    status?: number;
    data?: ApiErrorBody;
  };
  message?: string;
}

export function apiErrorBody(error: unknown): ApiErrorBody | null {
  return (error as ErrorShape | null)?.response?.data ?? null;
}

export function apiErrorMessage(error: unknown, fallback: string): string {
  const body = apiErrorBody(error);
  if (body?.message) {
    return body.message;
  }
  const message = (error as ErrorShape | null)?.message;
  return message || fallback;
}

export function showError(text: string): void {
  if (window.$message) {
    window.$message.error(text);
  } else if (window.$dialog) {
    window.$dialog.error({ title: 'Error', content: text });
  } else {
    console.error(text);
  }
}

export function showApiError(error: unknown, fallback: string): void {
  showError(apiErrorMessage(error, fallback));
}
