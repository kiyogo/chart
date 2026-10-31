import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './e2e',
  fullyParallel: false,
  timeout: 30000,
  use: {
    baseURL: 'http://127.0.0.1:8081',
    trace: 'on-first-retry',
  },
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
    {
      name: 'mobile',
      use: { ...devices['Pixel 5'] },
    },
  ],
  webServer: {
    command: '..\\backend\\target\\release\\chatw.exe',
    cwd: '..\\backend',
    url: 'http://127.0.0.1:8081',
    reuseExistingServer: true,
    timeout: 10000,
  },
});
