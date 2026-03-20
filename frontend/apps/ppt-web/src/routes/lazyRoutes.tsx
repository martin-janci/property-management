/**
 * Lazy-loaded Route Components
 * Epic 130: Performance Optimization
 *
 * Implements code splitting for feature pages to reduce initial bundle size.
 * Each feature module is loaded on-demand when the user navigates to it.
 */

import { lazy } from 'react';

// Documents feature (Epic 39)
export const DocumentsPage = lazy(() =>
  import('../features/documents').then((m) => ({ default: m.DocumentsPage }))
);
export const DocumentUploadPage = lazy(() =>
  import('../features/documents').then((m) => ({ default: m.DocumentUploadPage }))
);
export const DocumentDetailPage = lazy(() =>
  import('../features/documents').then((m) => ({ default: m.DocumentDetailPage }))
);

// News feature (Epic 59)
export const NewsListPage = lazy(() =>
  import('../features/news').then((m) => ({ default: m.NewsListPage }))
);
export const ArticleDetailPage = lazy(() =>
  import('../features/news').then((m) => ({ default: m.ArticleDetailPage }))
);

// Emergency feature (Epic 62)
export const EmergencyContactDirectoryPage = lazy(() =>
  import('../features/emergency').then((m) => ({ default: m.EmergencyContactDirectoryPage }))
);

// Disputes feature (Epic 77)
export const DisputesPage = lazy(() =>
  import('../features/disputes').then((m) => ({ default: m.DisputesPage }))
);
export const FileDisputePage = lazy(() =>
  import('../features/disputes').then((m) => ({ default: m.FileDisputePage }))
);

// Outages feature (UC-12)
export const OutagesPage = lazy(() =>
  import('../features/outages').then((m) => ({ default: m.OutagesPage }))
);
export const CreateOutagePage = lazy(() =>
  import('../features/outages').then((m) => ({ default: m.CreateOutagePage }))
);
export const ViewOutagePage = lazy(() =>
  import('../features/outages').then((m) => ({ default: m.ViewOutagePage }))
);
export const EditOutagePage = lazy(() =>
  import('../features/outages').then((m) => ({ default: m.EditOutagePage }))
);

// Settings features (Epic 60, 63)
export const AccessibilitySettingsPage = lazy(() =>
  import('../features/settings').then((m) => ({ default: m.AccessibilitySettingsPage }))
);
export const PrivacySettingsPage = lazy(() =>
  import('../features/privacy').then((m) => ({ default: m.PrivacySettingsPage }))
);

// Auth pages
export const LoginPage = lazy(() =>
  import('../pages/LoginPage').then((m) => ({ default: m.LoginPage }))
);

// Announcements feature (UC-06)
export const AnnouncementsPage = lazy(() =>
  import('../features/announcements').then((m) => ({ default: m.AnnouncementsPage }))
);
export const CreateAnnouncementPage = lazy(() =>
  import('../features/announcements').then((m) => ({ default: m.CreateAnnouncementPage }))
);
export const ViewAnnouncementPage = lazy(() =>
  import('../features/announcements').then((m) => ({ default: m.ViewAnnouncementPage }))
);
export const EditAnnouncementPage = lazy(() =>
  import('../features/announcements').then((m) => ({ default: m.EditAnnouncementPage }))
);

// Messaging feature (UC-07)
export const MessagesPage = lazy(() =>
  import('../features/messaging').then((m) => ({ default: m.MessagesPage }))
);
export const ThreadDetailPage = lazy(() =>
  import('../features/messaging').then((m) => ({ default: m.ThreadDetailPage }))
);
export const NewMessagePage = lazy(() =>
  import('../features/messaging').then((m) => ({ default: m.NewMessagePage }))
);

// Faults feature (UC-03)
export const FaultsPage = lazy(() =>
  import('../features/faults').then((m) => ({ default: m.FaultsPage }))
);
export const FaultDetailPage = lazy(() =>
  import('../features/faults').then((m) => ({ default: m.FaultDetailPage }))
);
export const CreateFaultPage = lazy(() =>
  import('../features/faults').then((m) => ({ default: m.CreateFaultPage }))
);
export const EditFaultPage = lazy(() =>
  import('../features/faults').then((m) => ({ default: m.EditFaultPage }))
);

// Community feature (Epic 42)
export const FeedPage = lazy(() =>
  import('../features/community').then((m) => ({ default: m.FeedPage }))
);
export const GroupsPage = lazy(() =>
  import('../features/community').then((m) => ({ default: m.GroupsPage }))
);
export const GroupDetailPage = lazy(() =>
  import('../features/community').then((m) => ({ default: m.GroupDetailPage }))
);
export const CreateGroupPage = lazy(() =>
  import('../features/community').then((m) => ({ default: m.CreateGroupPage }))
);
export const EventsPage = lazy(() =>
  import('../features/community').then((m) => ({ default: m.EventsPage }))
);
export const MarketplacePage = lazy(() =>
  import('../features/community').then((m) => ({ default: m.MarketplacePage }))
);

// Financial feature (Epic 52)
export const FinancialDashboardPage = lazy(() =>
  import('../features/financial').then((m) => ({ default: m.FinancialDashboardPage }))
);
export const InvoiceManagementPage = lazy(() =>
  import('../features/financial').then((m) => ({ default: m.InvoiceManagementPage }))
);
export const PaymentManagementPage = lazy(() =>
  import('../features/financial').then((m) => ({ default: m.PaymentManagementPage }))
);
export const BudgetManagementPage = lazy(() =>
  import('../features/financial').then((m) => ({ default: m.BudgetManagementPage }))
);

// Command Palette (Epic 129) - loaded eagerly since it's used globally
// CommandPaletteDialog and CommandPaletteProvider are NOT lazy loaded
// because they need to be available immediately for keyboard shortcuts
