# Frontend Features Implementation Design

**Date:** 2026-03-20
**Features:** Announcements, Messaging, Faults, Community, Financial

## Overview

Implement 5 priority frontend features with full API integration following established patterns from Outages/Disputes features.

## Architecture Pattern

```
App.tsx Route Wrapper (data fetching, type transformation)
    └── Feature Page (presentational, receives clean props)
        └── Feature Components (forms, lists, cards)
```

**Key Patterns:**
- TanStack Query hooks from `@ppt/api-client`
- Type transformation: API types (snake_case) → UI types (camelCase)
- Query key factories for cache management
- Toast notifications for feedback
- i18n for all user-facing strings

## Feature Designs

### 1. Announcements (UC-06)

**Existing scaffolding:** Most components exist, need API wiring.

**Pages:**
- `AnnouncementsPage` - List with status/target filters
- `CreateAnnouncementPage` - Form with target selector, schedule picker
- `EditAnnouncementPage` - Edit existing draft
- `ViewAnnouncementPage` - Detail with acknowledgment stats, comments

**API Integration:**
- `useList(params)` - List with filters
- `useCreate()` - Create new
- `useUpdate()` - Edit draft
- `usePublish()` / `useArchive()` - Status changes
- `useAcknowledge()` - User acknowledgment
- `useAcknowledgmentStats(id)` - Stats for managers

**Components to complete:**
- Wire `AnnouncementForm` to mutations
- Wire `AnnouncementList` to `useList`
- Complete `AcknowledgmentStats` display
- Add route wrappers in App.tsx

### 2. Messaging (UC-04)

**Existing scaffolding:** Pages exist with local types, need API connection.

**Pages:**
- `MessagesPage` - Thread list with unread indicators
- `ThreadDetailPage` - Conversation view with message input
- `NewMessagePage` - Start new conversation

**API Integration:**
- `useThreads(params)` - List threads
- `useThread(id)` - Thread with messages
- `useUnreadCount()` - Badge count
- `useStartThread()` - New conversation
- `useSendMessage()` - Send message
- `useMarkThreadRead()` - Mark as read

**Components to update:**
- Replace local types with API types
- Add polling for thread list (30s via hook config)
- Add recipient search (use existing user search)

### 3. Faults (UC-05)

**Existing scaffolding:** Minimal - mostly empty folders.

**Pages to implement:**
- `FaultsPage` - List with status/category/priority/assignee filters
- `CreateFaultPage` - Report form with photo upload
- `FaultDetailPage` - Full detail with timeline, comments, actions

**API Integration:**
- `useFaults(query)` - List faults
- `useFault(id)` - Fault details with timeline
- `useCreateFault()` - Report new fault
- `useTriageFault(id)` - Set priority/category
- `useAssignFault(id)` - Assign to worker
- `useResolveFault(id)` - Mark resolved
- `useAddComment(id)` - Add comment
- `useAddAttachment(id)` - Upload photo
- `useAiSuggestion(id)` - AI category suggestion

**Components to create:**
- `FaultForm` - Create/edit form
- `FaultList` - Filterable list with pagination
- `FaultCard` - Summary card
- `FaultTimeline` - Event history
- `FaultComments` - Comment thread
- `PhotoUpload` - Image upload component
- `AiSuggestionBadge` - Display AI suggestions

### 4. Community (UC-08)

**Existing scaffolding:** Rich UI components, need API connection.

**Focus areas:** Groups, Posts, Events (defer marketplace to basic view)

**Pages:**
- `CommunityPage` - Dashboard with recent activity
- `GroupsPage` - List of groups
- `GroupDetailPage` - Group with posts
- `EventsPage` - Upcoming events
- `EventDetailPage` - Event with RSVP
- `MarketplacePage` - Item listings (basic)

**API Integration:**
- Groups: `useGroups`, `useGroup`, `useCreateGroup`, `useJoinGroup`
- Posts: `usePosts`, `useCreatePost`, `useLikePost`, `useCreatePostComment`
- Events: `useEvents`, `useEvent`, `useRsvpEvent`, `useCancelRsvp`
- Items: `useMarketplaceItems`, `useMarketplaceItem`

**Components to wire:**
- `GroupList`, `GroupForm` - Groups CRUD
- `PostCard`, `CreatePostForm` - Post feed
- `EventCard`, `EventForm` - Events
- `ItemCard` - Marketplace listing

### 5. Financial (UC-09)

**Existing scaffolding:** Good component structure, need API wiring.

**Pages:**
- `FinancialDashboardPage` - Overview with metrics
- `InvoicesPage` - List with status filters
- `InvoiceDetailPage` - Invoice with line items
- `PaymentsPage` - Payment history
- `ARAgingPage` - Accounts receivable report

**API Integration:**
- `listInvoices(params)` - Invoice list
- `getInvoice(id)` - Invoice detail
- `recordPayment()` - Record payment
- `getAccountsReceivableReport()` - AR aging
- `getFinancialDashboard()` - Dashboard metrics

**Components to wire:**
- `InvoiceList`, `InvoiceDetail` - Invoice management
- `PaymentForm` - Payment recording
- `ARAgingTable` - AR report display
- `BudgetOverview` - Budget summary

## Implementation Order

1. **Announcements** - Most scaffolding exists, quick wins
2. **Faults** - High user impact, clear workflow
3. **Messaging** - Core communication feature
4. **Financial** - Complex but well-defined
5. **Community** - Largest scope, do core features

## File Structure

Each feature follows:
```
features/{feature}/
├── components/
│   ├── {Feature}Form.tsx
│   ├── {Feature}List.tsx
│   ├── {Feature}Card.tsx
│   └── ...
├── pages/
│   ├── {Feature}sPage.tsx
│   ├── Create{Feature}Page.tsx
│   ├── View{Feature}Page.tsx
│   └── Edit{Feature}Page.tsx
└── index.ts (barrel exports)
```

## Success Criteria

- [ ] All 5 features build without errors (`pnpm build`)
- [ ] List views show data from API
- [ ] Create/edit forms submit to API
- [ ] Status changes work (publish, archive, resolve, etc.)
- [ ] Error handling with toast notifications
- [ ] Loading states displayed
- [ ] Each feature committed separately
