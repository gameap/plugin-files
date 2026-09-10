export type NodeOs = 'linux' | 'windows';

function required(name: string): string {
  const value = process.env[name];
  if (!value) {
    throw new Error(`${name} must be set`);
  }

  return value;
}

export const env = {
  baseUrl: process.env.E2E_BASE_URL ?? 'http://127.0.0.1:8025',
  apiBaseUrl: process.env.E2E_API_BASE_URL ?? 'http://127.0.0.1:8025',
  adminUser: process.env.E2E_ADMIN_USER ?? 'admin',
  get adminPassword(): string {
    return required('E2E_ADMIN_PASSWORD');
  },
  get nodeOs(): NodeOs {
    const value = required('E2E_NODE_OS');
    if (value !== 'linux' && value !== 'windows') {
      throw new Error(`E2E_NODE_OS must be linux or windows, got ${value}`);
    }

    return value;
  },
  get nodeHost(): string {
    return required('E2E_NODE_HOST');
  },
  get nodeWorkPath(): string {
    return required('E2E_NODE_WORK_PATH');
  },
  nodeContainer: process.env.E2E_NODE_CONTAINER ?? 'gameap-node',
  get wasmPath(): string {
    return required('E2E_WASM_PATH');
  },
  runId: process.env.E2E_RUN_ID ?? 'local',
  stateDir: process.env.E2E_STATE_DIR ?? 'test-results/state',
};

// Everything the suite creates is named from the run id so a spec can re-derive
// it without depending on state left behind by an earlier spec. The username
// has to satisfy the plugin's ^[a-zA-Z][a-zA-Z0-9_]{2,31}$.
const slug = env.runId.replace(/[^a-zA-Z0-9]/g, '').slice(-12) || 'local';

export const names = {
  gameCode: `e2eftp${slug}`.slice(0, 16).toLowerCase(),
  gameName: `E2E FTP ${slug}`,
  modName: 'Default',
  serverName: `e2e-ftp-${slug}`,
  ftpUser: `e2e_${slug}`,
  ftpPassword: `E2e-ftp-${slug}!`,
};

export const PLUGIN_ID = 'files';

export const REQUIRED_PERMISSIONS = [
  'files',
  'listen_events',
  'manage_servers',
  'node_commands',
] as const;

// Ports the install spec configures, deliberately different from the plugin's
// 30000-30100 default so the setup form is proven to be read.
export const ports = {
  ftp: 21,
  passiveMin: 30000,
  passiveMax: 30009,
  sftp: 2222,
  ftpChanged: 2121,
  sftpChanged: 2223,
};
