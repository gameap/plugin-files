/**
 * Type declarations for window.gameapDebug
 * @see /Users/nikita/Git/gameap/gameap-api/web/frontend/packages/gameap-debug/src/mocks/README.md
 */

interface GameAPDebugMSW {
  http: {
    get: (path: string, handler: (info: { params: Record<string, string>; request: Request }) => Promise<Response> | Response) => unknown;
    post: (path: string, handler: (info: { params: Record<string, string>; request: Request }) => Promise<Response> | Response) => unknown;
    put: (path: string, handler: (info: { params: Record<string, string>; request: Request }) => Promise<Response> | Response) => unknown;
    delete: (path: string, handler: (info: { params: Record<string, string>; request: Request }) => Promise<Response> | Response) => unknown;
    patch: (path: string, handler: (info: { params: Record<string, string>; request: Request }) => Promise<Response> | Response) => unknown;
  };
  HttpResponse: {
    json: (data: unknown, init?: ResponseInit) => Response;
    text: (data: string, init?: ResponseInit) => Response;
    error: () => Response;
  };
  delay: (ms: number) => Promise<void>;
}

interface GameAPDebugMockData {
  addServer: (server: Record<string, unknown>) => unknown;
  updateServer: (id: number, data: Record<string, unknown>) => void;
  removeServer: (id: number) => void;
  getServers: () => unknown[];
  addUser: (type: string, user: Record<string, unknown>) => void;
}

interface GameAPDebug {
  msw: GameAPDebugMSW;
  mockData: GameAPDebugMockData;
  registerMockHandlers: (handlers: unknown[]) => void;
  resetMockHandlers: () => void;
}

declare global {
  interface Window {
    gameapDebug?: GameAPDebug;
  }
}

export {};
