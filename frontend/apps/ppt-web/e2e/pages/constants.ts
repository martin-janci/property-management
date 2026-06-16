/**
 * Shared constants for ppt-web page objects.
 *
 * Centralizes the app name and sitemap route ids so no spec or page object
 * repeats the literal strings.
 */

import type { SitemapApp } from '@ppt/e2e';

/** This app, as a sitemap-backed app. */
export const PPT_WEB_APP: SitemapApp = 'ppt-web';

/** Sitemap route ids for ppt-web (source of truth: @ppt/sitemap). */
export const PPT_LOGIN_ROUTE_ID = 'ppt-login';
export const PPT_HOME_ROUTE_ID = 'ppt-home';
export const PPT_DOCUMENTS_ROUTE_ID = 'ppt-documents';
export const PPT_DOCUMENT_DETAIL_ROUTE_ID = 'ppt-document-detail';
export const PPT_DOCUMENT_UPLOAD_ROUTE_ID = 'ppt-document-upload';
export const PPT_NEWS_ROUTE_ID = 'ppt-news';
export const PPT_EMERGENCY_ROUTE_ID = 'ppt-emergency';
export const PPT_DISPUTES_ROUTE_ID = 'ppt-disputes';
export const PPT_DISPUTE_NEW_ROUTE_ID = 'ppt-dispute-new';
