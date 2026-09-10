import type { APIRequestContext } from '@playwright/test';
import { env } from './env';

export interface LoginCredentials {
  login: string;
  password: string;
}

// A personal access token cannot be used here: /api/admin/plugins/* is guarded
// by TokenAdminGuardMiddleware, which answers 403 pat_not_allowed_on_admin_route
// for every token session. Only the PASETO from /api/auth/login works.
export async function loginViaAPI(
  request: APIRequestContext,
  credentials: LoginCredentials = {
    login: env.adminUser,
    password: env.adminPassword,
  },
): Promise<string> {
  const response = await request.post(`${env.apiBaseUrl}/api/auth/login`, {
    data: credentials,
  });

  if (!response.ok()) {
    throw new Error(
      `login failed: ${response.status()} ${await response.text()}`,
    );
  }

  const body = (await response.json()) as {
    token?: string;
    two_factor_required?: boolean;
  };

  if (!body.token) {
    const hint = body.two_factor_required
      ? ' (two-factor challenge; set AUTH_REQUIRE_MFA_FOR_ADMINS=false)'
      : '';

    throw new Error(`login response missing token${hint}`);
  }

  return body.token;
}

export function authHeader(token: string): Record<string, string> {
  return { Authorization: `Bearer ${token}` };
}
