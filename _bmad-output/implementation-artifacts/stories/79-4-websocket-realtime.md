# Story 79.4: WebSocket Real-time Integration

Status: done

## Story

As a **ppt-web user**,
I want to **receive real-time updates for messages, notifications, and data changes**,
So that **I always see the latest information without manually refreshing the page**.

## Acceptance Criteria

1. **AC-1: WebSocket Connection Management**
   - Given I am authenticated in the application
   - When I open the dashboard
   - Then a WebSocket connection is established to the server
   - And the connection is authenticated with my JWT token
   - And connection status is visible in the UI

2. **AC-2: Real-time Message Updates**
   - Given I am viewing the messages page
   - When another user sends me a message
   - Then the message appears in my inbox immediately
   - And a notification sound plays (if enabled)
   - And the unread count updates in the header

3. **AC-3: Real-time Notification Sync**
   - Given I have notification preferences configured
   - When a relevant event occurs (announcement, fault update, vote)
   - Then a real-time notification appears
   - And it matches my notification preference settings

4. **AC-4: Data Invalidation on Server Push**
   - Given I am viewing a data list (faults, announcements, etc.)
   - When another user modifies the data
   - Then my view updates automatically via query invalidation
   - And a subtle indicator shows "Data updated"

5. **AC-5: Reconnection Handling**
   - Given my WebSocket connection is lost
   - When connectivity is restored
   - Then the connection automatically reconnects
   - And missed events are fetched and applied
   - And a "Reconnected" toast is shown

## Tasks / Subtasks

- [ ] Task 1: Create WebSocket Service (AC: 1, 5)
  - [ ] 1.1 Create `/frontend/apps/ppt-web/src/lib/websocket.ts`
  - [ ] 1.2 Implement WebSocket connection with auth token
  - [ ] 1.3 Add automatic reconnection with exponential backoff
  - [ ] 1.4 Create connection state machine (connecting, connected, disconnected, error)
  - [ ] 1.5 Implement heartbeat/ping-pong mechanism

- [ ] Task 2: Create WebSocket Context and Provider (AC: 1, 2, 3, 4)
  - [ ] 2.1 Create `/frontend/apps/ppt-web/src/contexts/WebSocketContext.tsx`
  - [ ] 2.2 Expose connection state and methods via context
  - [ ] 2.3 Create `useWebSocket` hook for subscribing to events
  - [ ] 2.4 Integrate with AuthContext for auth token

- [ ] Task 3: Implement Message Event Handling (AC: 2)
  - [ ] 3.1 Subscribe to `message:new` WebSocket events
  - [ ] 3.2 Update messages query cache on new message
  - [ ] 3.3 Update unread count in header
  - [ ] 3.4 Play notification sound (with user preference check)
  - [ ] 3.5 Show browser notification (if permitted)

- [ ] Task 4: Implement Notification Sync (AC: 3)
  - [ ] 4.1 Update `/frontend/packages/api-client/src/notification-preferences/sync.ts`
  - [ ] 4.2 Subscribe to `notification:*` WebSocket events
  - [ ] 4.3 Filter events based on user preferences
  - [ ] 4.4 Display real-time notification toasts

- [ ] Task 5: Implement Query Invalidation System (AC: 4)
  - [ ] 5.1 Create event-to-query-key mapping configuration
  - [ ] 5.2 Subscribe to entity change events (`entity:updated`, `entity:created`, `entity:deleted`)
  - [ ] 5.3 Invalidate relevant TanStack Query caches
  - [ ] 5.4 Add subtle "Data updated" indicator component

- [ ] Task 6: Implement Reconnection and Sync (AC: 5)
  - [ ] 6.1 Track last event timestamp for gap detection
  - [ ] 6.2 Fetch missed events on reconnection via REST endpoint
  - [ ] 6.3 Apply missed events to update local state
  - [ ] 6.4 Show reconnection status in toast

## Dev Notes

### Architecture Requirements
- WebSocket connection managed via React Context
- Integration with TanStack Query for cache invalidation
- Graceful degradation when WebSocket unavailable
- Event-driven architecture with typed event handlers

### Technical Specifications
- WebSocket URL: `import.meta.env.VITE_WS_URL || 'ws://localhost:8080/ws'`
- Auth: JWT token sent as query param or first message
- Reconnection: Exponential backoff (1s, 2s, 4s, 8s, max 30s)
- Heartbeat: Ping every 30 seconds, timeout after 10s no pong

### WebSocket Message Format
```typescript
interface WebSocketMessage {
  type: string;          // Event type (e.g., "message:new")
  payload: unknown;      // Event-specific data
  timestamp: string;     // ISO timestamp
  requestId?: string;    // For request-response correlation
}

// Event types
type WebSocketEventType =
  | 'message:new'
  | 'notification:announcement'
  | 'notification:fault'
  | 'notification:vote'
  | 'entity:updated'
  | 'entity:created'
  | 'entity:deleted'
  | 'connection:authenticated'
  | 'connection:error';
```

### Query Invalidation Mapping
```typescript
const eventToQueryKeys: Record<string, string[]> = {
  'entity:announcement': ['announcements'],
  'entity:fault': ['faults'],
  'entity:vote': ['votes'],
  'entity:document': ['documents'],
  'entity:message': ['messages'],
  'entity:neighbor': ['neighbors'],
};
```

### File List (to create/modify)

**Create:**
- `/frontend/apps/ppt-web/src/lib/websocket.ts` - WebSocket service
- `/frontend/apps/ppt-web/src/contexts/WebSocketContext.tsx` - WebSocket provider
- `/frontend/apps/ppt-web/src/hooks/useWebSocket.ts` - WebSocket hook
- `/frontend/apps/ppt-web/src/components/ConnectionStatus.tsx` - Status indicator

**Modify:**
- `/frontend/apps/ppt-web/src/App.tsx` - Add WebSocketProvider
- `/frontend/packages/api-client/src/notification-preferences/sync.ts` - Wire to WebSocket
- `/frontend/apps/ppt-web/src/components/Header.tsx` - Add connection status, unread count

### Backend WebSocket Endpoints
- `GET /ws` - WebSocket upgrade endpoint (existing in api-server)
- `GET /api/v1/events/missed?since={timestamp}` - Fetch missed events (may need implementation)

### Dependencies
- Story 79.2 (Authentication Flow) - For JWT token access
- Backend WebSocket support (already exists in api-server)

### References
- [Backend: backend/servers/api-server/src/routes/websocket.rs]
- [Source: frontend/packages/api-client/src/notification-preferences/sync.ts:37 TODO]
- [UC-23: Notifications]
