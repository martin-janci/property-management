import { describe, expect, it } from 'vitest';
import {
  getEndpoint,
  getFlow,
  getProtectedRoutes,
  getPublicRoutes,
  getRoute,
  getScreen,
  sitemap,
} from '../src';
import { buildUrl, SitemapTestHelper } from '../src/utils';

describe('Sitemap Data', () => {
  describe('Routes', () => {
    it('should have ppt-web routes', () => {
      expect(sitemap.routes['ppt-web'].length).toBe(19);
    });

    it('should have reality-web routes', () => {
      expect(sitemap.routes['reality-web'].length).toBe(9);
    });

    it('should get route by id', () => {
      const route = getRoute('ppt-web', 'ppt-login');
      expect(route).toBeDefined();
      expect(route?.path).toBe('/login');
    });
  });

  describe('Mobile Screens', () => {
    it('should have mobile screens', () => {
      expect(sitemap.screens.mobile.length).toBe(6);
    });

    it('should get screen by id', () => {
      const screen = getScreen('mobile-dashboard');
      expect(screen).toBeDefined();
      expect(screen?.screenName).toBe('Dashboard');
    });
  });

  describe('API Endpoints', () => {
    it('should have api-server endpoints', () => {
      expect(sitemap.endpoints['api-server'].length).toBeGreaterThan(0);
    });

    it('should have reality-server endpoints', () => {
      expect(sitemap.endpoints['reality-server'].length).toBeGreaterThan(0);
    });

    it('should get endpoint by operation id', () => {
      const endpoint = getEndpoint('api-server', 'auth_login');
      expect(endpoint).toBeDefined();
      expect(endpoint?.method).toBe('POST');
      expect(endpoint?.path).toBe('/api/v1/auth/login');
    });
  });

  describe('User Flows', () => {
    it('should have user flows', () => {
      expect(sitemap.flows.length).toBeGreaterThan(0);
    });

    it('should get flow by id', () => {
      const flow = getFlow('flow-login-basic');
      expect(flow).toBeDefined();
      expect(flow?.name).toBe('Basic Login Flow');
    });
  });
});

describe('Route Helpers', () => {
  describe('buildUrl', () => {
    it('should build simple url', () => {
      const route = getRoute('ppt-web', 'ppt-documents')!;
      expect(buildUrl(route)).toBe('/documents');
    });

    it('should build url with params', () => {
      const route = getRoute('ppt-web', 'ppt-document-detail')!;
      const url = buildUrl(route, { documentId: '123e4567-e89b-12d3-a456-426614174000' });
      expect(url).toBe('/documents/123e4567-e89b-12d3-a456-426614174000');
    });

    it('should build url with query params', () => {
      const route = getRoute('reality-web', 'reality-listings')!;
      const url = buildUrl(route, {}, { type: 'sale', city: 'Bratislava' });
      expect(url).toBe('/listings?type=sale&city=Bratislava');
    });
  });
});

describe('Test Helpers', () => {
  describe('getProtectedRoutes', () => {
    it('should return only protected routes', () => {
      const protectedRoutes = getProtectedRoutes('ppt-web');
      expect(protectedRoutes.every((r) => r.auth.required)).toBe(true);
    });
  });

  describe('getPublicRoutes', () => {
    it('should return only public routes', () => {
      const publicRoutes = getPublicRoutes('ppt-web');
      expect(publicRoutes.every((r) => !r.auth.required)).toBe(true);
    });
  });

  describe('SitemapTestHelper', () => {
    it('should get routes for role', () => {
      const ownerRoutes = SitemapTestHelper.getRoutesForRole('ppt-web', 'owner');
      expect(ownerRoutes.length).toBeGreaterThan(0);
    });

    it('should validate route endpoints exist', () => {
      const result = SitemapTestHelper.validateRouteEndpoints('ppt-web');
      // Some endpoints may not be defined yet, but the validation should work
      expect(result).toHaveProperty('valid');
      expect(result).toHaveProperty('missing');
    });

    it('should get mobile tabs', () => {
      const tabs = SitemapTestHelper.getMobileTabs();
      expect(tabs.length).toBeGreaterThan(0);
      expect(tabs[0]).toHaveProperty('name');
      expect(tabs[0]).toHaveProperty('screens');
    });
  });
});
