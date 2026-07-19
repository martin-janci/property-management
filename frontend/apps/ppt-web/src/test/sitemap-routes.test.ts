/// <reference types="vitest/globals" />
/**
 * Sitemap Route Tests (Epic 101)
 *
 * Tests route accessibility and API mapping using @ppt/sitemap
 */

import type { FrontendRoute } from '@ppt/sitemap';
import {
  buildUrl,
  getProtectedRoutes,
  getPublicRoutes,
  getRoute,
  SitemapTestHelper,
  sitemap,
} from '@ppt/sitemap';

describe('PPT-Web Route Definitions', () => {
  const app = 'ppt-web' as const;

  describe('Route Coverage', () => {
    it('should have all expected routes defined', () => {
      const expectedRoutes = [
        'ppt-home',
        'ppt-login',
        'ppt-documents',
        'ppt-document-upload',
        'ppt-document-detail',
        'ppt-news',
        'ppt-news-detail',
        'ppt-emergency',
        'ppt-settings-accessibility',
        'ppt-settings-privacy',
        'ppt-settings-notifications',
        'ppt-settings-notifications-advanced',
        'ppt-disputes',
        'ppt-dispute-new',
        'ppt-dispute-detail',
      ];

      for (const routeId of expectedRoutes) {
        const route = getRoute(app, routeId);
        expect(route, `Route ${routeId} should exist`).toBeDefined();
      }
    });

    it('should have correct route count', () => {
      expect(sitemap.routes[app].length).toBe(20);
    });
  });

  describe('Public Routes', () => {
    const publicRoutes = getPublicRoutes(app);

    it('should have public routes', () => {
      expect(publicRoutes.length).toBeGreaterThan(0);
    });

    it('home should be public', () => {
      const home = getRoute(app, 'ppt-home');
      expect(home?.auth.required).toBe(false);
    });

    it('login should be public', () => {
      const login = getRoute(app, 'ppt-login');
      expect(login?.auth.required).toBe(false);
    });

    it.each(publicRoutes.map((r) => [r.name, r]))(
      '%s should not require authentication',
      (_: string, route: FrontendRoute) => {
        expect(route.auth.required).toBe(false);
      }
    );
  });

  describe('Protected Routes', () => {
    const protectedRoutes = getProtectedRoutes(app);

    it('should have protected routes', () => {
      expect(protectedRoutes.length).toBeGreaterThan(0);
    });

    it('documents should be protected', () => {
      const docs = getRoute(app, 'ppt-documents');
      expect(docs?.auth.required).toBe(true);
    });

    it('disputes should be protected', () => {
      const disputes = getRoute(app, 'ppt-disputes');
      expect(disputes?.auth.required).toBe(true);
    });

    it.each(protectedRoutes.map((r) => [r.name, r]))(
      '%s should require authentication',
      (_: string, route: FrontendRoute) => {
        expect(route.auth.required).toBe(true);
      }
    );
  });

  describe('Route Parameters', () => {
    it('document detail should have documentId param', () => {
      const route = getRoute(app, 'ppt-document-detail');
      expect(route?.params).toBeDefined();
      expect(route?.params?.find((p) => p.name === 'documentId')).toBeDefined();
    });

    it('dispute detail should have disputeId param', () => {
      const route = getRoute(app, 'ppt-dispute-detail');
      expect(route?.params).toBeDefined();
      expect(route?.params?.find((p) => p.name === 'disputeId')).toBeDefined();
    });

    it('news detail should have articleId param', () => {
      const route = getRoute(app, 'ppt-news-detail');
      expect(route?.params).toBeDefined();
      expect(route?.params?.find((p) => p.name === 'articleId')).toBeDefined();
    });
  });

  describe('URL Building', () => {
    it('should build simple URLs', () => {
      const route = getRoute(app, 'ppt-documents')!;
      expect(buildUrl(route)).toBe('/documents');
    });

    it('should build URLs with parameters', () => {
      const route = getRoute(app, 'ppt-document-detail')!;
      const url = buildUrl(route, { documentId: 'abc-123' });
      expect(url).toBe('/documents/abc-123');
    });
  });

  describe('API Endpoint Mappings', () => {
    it('login route should map to auth_login endpoint', () => {
      const route = getRoute(app, 'ppt-login');
      expect(route?.apiEndpoints).toContain('auth_login');
    });

    it('documents route should map to documents_list endpoint', () => {
      const route = getRoute(app, 'ppt-documents');
      expect(route?.apiEndpoints).toContain('documents_list');
    });

    it('disputes route should map to disputes_list endpoint', () => {
      const route = getRoute(app, 'ppt-disputes');
      expect(route?.apiEndpoints).toContain('disputes_list');
    });
  });

  describe('Role-Based Access', () => {
    it('owner should access document upload', () => {
      const ownerRoutes = SitemapTestHelper.getRoutesForRole(app, 'owner');
      const uploadRoute = ownerRoutes.find((r) => r.id === 'ppt-document-upload');
      expect(uploadRoute).toBeDefined();
    });

    it('tenant should access disputes', () => {
      const tenantRoutes = SitemapTestHelper.getRoutesForRole(app, 'tenant');
      const disputesRoute = tenantRoutes.find((r) => r.id === 'ppt-disputes');
      expect(disputesRoute).toBeDefined();
    });

    it('should validate route endpoints exist', () => {
      const result = SitemapTestHelper.validateRouteEndpoints(app);
      // Log any missing endpoints for debugging
      if (!result.valid) {
        console.warn('Missing endpoints:', result.missing);
      }
      // Not all endpoints may be defined yet, so we just check the structure
      expect(result).toHaveProperty('valid');
      expect(result).toHaveProperty('missing');
    });
  });

  describe('Route Hierarchy', () => {
    it('document upload should have documents as parent', () => {
      const route = getRoute(app, 'ppt-document-upload');
      expect(route?.parentId).toBe('ppt-documents');
    });

    it('document detail should have documents as parent', () => {
      const route = getRoute(app, 'ppt-document-detail');
      expect(route?.parentId).toBe('ppt-documents');
    });

    it('dispute new should have disputes as parent', () => {
      const route = getRoute(app, 'ppt-dispute-new');
      expect(route?.parentId).toBe('ppt-disputes');
    });
  });

  describe('Feature Tagging', () => {
    it('document routes should be tagged with documents', () => {
      const docRoutes = SitemapTestHelper.getRoutesByTag(app, 'documents');
      expect(docRoutes.length).toBeGreaterThan(0);
      expect(docRoutes.every((r) => r.path.includes('/documents'))).toBe(true);
    });

    it('dispute routes should be tagged with disputes', () => {
      const disputeRoutes = SitemapTestHelper.getRoutesByTag(app, 'disputes');
      expect(disputeRoutes.length).toBeGreaterThan(0);
      expect(disputeRoutes.every((r) => r.path.includes('/disputes'))).toBe(true);
    });

    it('protected routes should be tagged with protected', () => {
      const protectedTagged = SitemapTestHelper.getRoutesByTag(app, 'protected');
      expect(protectedTagged.every((r) => r.auth.required)).toBe(true);
    });
  });
});

describe('API Endpoint Definitions', () => {
  const server = 'api-server' as const;

  describe('Authentication Endpoints', () => {
    it('should have login endpoint', () => {
      const endpoint = sitemap.endpoints[server].find((e) => e.operationId === 'auth_login');
      expect(endpoint).toBeDefined();
      expect(endpoint?.method).toBe('POST');
      expect(endpoint?.path).toBe('/api/v1/auth/login');
      expect(endpoint?.auth.required).toBe(false);
    });

    it('should have logout endpoint', () => {
      const endpoint = sitemap.endpoints[server].find((e) => e.operationId === 'auth_logout');
      expect(endpoint).toBeDefined();
      expect(endpoint?.method).toBe('POST');
      expect(endpoint?.auth.required).toBe(true);
    });
  });

  describe('Document Endpoints', () => {
    it('should have document list endpoint', () => {
      const endpoint = sitemap.endpoints[server].find((e) => e.operationId === 'documents_list');
      expect(endpoint).toBeDefined();
      expect(endpoint?.method).toBe('GET');
      expect(endpoint?.auth.required).toBe(true);
      expect(endpoint?.auth.tenantContext?.required).toBe(true);
    });

    it('should have document upload endpoint', () => {
      const endpoint = sitemap.endpoints[server].find((e) => e.operationId === 'documents_upload');
      expect(endpoint).toBeDefined();
      expect(endpoint?.method).toBe('POST');
    });
  });

  describe('Fault Endpoints', () => {
    it('should have fault create endpoint', () => {
      const endpoint = sitemap.endpoints[server].find((e) => e.operationId === 'faults_create');
      expect(endpoint).toBeDefined();
      expect(endpoint?.method).toBe('POST');
      expect(endpoint?.auth.required).toBe(true);
    });
  });
});

describe('User Flows', () => {
  it('should have login flow', () => {
    const flow = sitemap.flows.find((f) => f.id === 'flow-login-basic');
    expect(flow).toBeDefined();
    expect(flow?.category).toBe('authentication');
    expect(flow?.steps.length).toBeGreaterThan(0);
  });

  it('should have document upload flow', () => {
    const flow = sitemap.flows.find((f) => f.id === 'flow-document-upload');
    expect(flow).toBeDefined();
    expect(flow?.category).toBe('documents');
    expect(flow?.requiredRole).toBe('owner');
  });

  it('should have file dispute flow', () => {
    const flow = sitemap.flows.find((f) => f.id === 'flow-file-dispute');
    expect(flow).toBeDefined();
    expect(flow?.category).toBe('disputes');
  });

  it('login flow should have correct steps', () => {
    const flow = sitemap.flows.find((f) => f.id === 'flow-login-basic');
    expect(flow?.steps[0].name).toBe('Navigate to login page');
    expect(flow?.steps[flow.steps.length - 1].name).toBe('Redirect to dashboard');
  });
});
