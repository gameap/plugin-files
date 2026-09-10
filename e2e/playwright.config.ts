import { defineConfig, devices } from '@playwright/test';

// The suite is one ordered scenario against a live panel and a live node: the
// plugin is installed, gameap-files is installed on the node, users are created
// and then deleted. Re-running a failed test would replay a mutation against an
// already-mutated system, so there are no retries and no parallelism; specs run
// in the alphabetical order of their filenames. For the same reason the run
// stops at the first failure: everything after it would be asserting against a
// state the scenario never reached.
export default defineConfig({
  testDir: './specs',
  fullyParallel: false,
  workers: 1,
  retries: 0,
  maxFailures: 1,
  forbidOnly: !!process.env.CI,
  // A node install downloads an installer and a release binary over the public
  // internet and then registers a service; 10 minutes is the per-test ceiling
  // and stays below the plugin's own INSTALLING_TIMEOUT_SECS of 900.
  timeout: 10 * 60_000,
  expect: { timeout: 15_000 },
  reporter: process.env.CI
    ? [['list'], ['html', { open: 'never' }], ['github']]
    : [['list'], ['html', { open: 'never' }]],
  use: {
    baseURL: process.env.E2E_BASE_URL ?? 'http://127.0.0.1:8025',
    actionTimeout: 30_000,
    navigationTimeout: 60_000,
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
    video: 'retain-on-failure',
  },
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],
});
