# Story 79.3: Error Handling and Toast Notifications

Status: done

## Story

As a **ppt-web user**,
I want to **receive clear feedback when actions succeed or fail**,
So that **I understand the result of my interactions and can take appropriate action**.

## Acceptance Criteria

1. **AC-1: Success Toasts**
   - Given I complete an action successfully (create, update, delete)
   - When the API responds with success
   - Then a success toast appears with appropriate message
   - And the toast auto-dismisses after 5 seconds
   - And I can manually dismiss it earlier

2. **AC-2: Error Toasts**
   - Given an action fails
   - When the API returns an error
   - Then an error toast appears with the error message
   - And the toast remains until dismissed (for errors)
   - And I can copy error details for support

3. **AC-3: Network Error Handling**
   - Given I lose network connectivity
   - When I try to perform an action
   - Then I see an offline indicator in the header
   - And queued action message is shown
   - And actions are retried when online

4. **AC-4: Validation Errors**
   - Given I submit invalid data
   - When the API returns validation errors
   - Then form fields show inline error messages
   - And a summary toast lists all validation issues
   - And focus moves to first invalid field

5. **AC-5: Rate Limit Handling**
   - Given I am rate limited
   - When API returns 429 status
   - Then I see a clear message about rate limiting
   - And the retry-after time is displayed
   - And automatic retry is scheduled

## Tasks / Subtasks

- [ ] Task 1: Create Toast Notification System (AC: 1, 2)
  - [ ] 1.1 Create `/frontend/apps/ppt-web/src/components/Toast/ToastProvider.tsx`
  - [ ] 1.2 Create ToastContext with `addToast`, `removeToast` methods
  - [ ] 1.3 Create Toast component with success, error, warning, info variants
  - [ ] 1.4 Implement auto-dismiss with configurable duration
  - [ ] 1.5 Add dismiss on click and close button
  - [ ] 1.6 Support stacking up to 3 toasts

- [ ] Task 2: Create Toast Component Styles (AC: 1, 2)
  - [ ] 2.1 Create `/frontend/apps/ppt-web/src/components/Toast/Toast.tsx`
  - [ ] 2.2 Add icons for each toast type (check, x, warning, info)
  - [ ] 2.3 Add slide-in/slide-out animations
  - [ ] 2.4 Add copy button for error messages
  - [ ] 2.5 Ensure accessible (role="alert", aria-live)

- [ ] Task 3: Create API Error Handler (AC: 2, 4)
  - [ ] 3.1 Create `/frontend/apps/ppt-web/src/lib/errorHandler.ts`
  - [ ] 3.2 Parse backend ErrorResponse format
  - [ ] 3.3 Map error codes to user-friendly messages
  - [ ] 3.4 Handle validation error arrays with field paths
  - [ ] 3.5 Extract requestId for error reporting

- [ ] Task 4: Implement Network Status Detection (AC: 3)
  - [ ] 4.1 Create `/frontend/apps/ppt-web/src/hooks/useNetworkStatus.ts`
  - [ ] 4.2 Add online/offline event listeners
  - [ ] 4.3 Create NetworkStatusProvider context
  - [ ] 4.4 Create offline indicator component for header
  - [ ] 4.5 Queue mutations when offline

- [ ] Task 5: Implement Rate Limit Handling (AC: 5)
  - [ ] 5.1 Add 429 handler to axios response interceptor
  - [ ] 5.2 Parse Retry-After header
  - [ ] 5.3 Show countdown toast
  - [ ] 5.4 Automatically retry after delay
  - [ ] 5.5 Disable action buttons during rate limit

- [ ] Task 6: Wire Toast System Throughout App (AC: 1, 2, 3, 4, 5)
  - [ ] 6.1 Add ToastProvider to App.tsx root
  - [ ] 6.2 Create `useToast` hook for easy access
  - [ ] 6.3 Add success toasts to all mutation onSuccess callbacks
  - [ ] 6.4 Add error handler to global axios interceptor
  - [ ] 6.5 Update all forms to show inline validation errors

## Dev Notes

### Architecture Requirements
- Centralized toast management via React Context
- Consistent error message formatting across app
- Accessible announcements for screen readers (ARIA)
- Queue management for offline mutations

### Technical Specifications
- Toast duration: 5 seconds for success/info, persistent for errors
- Maximum visible toasts: 3 (older ones hidden, shown when space)
- Position: Top-right corner, below header
- Animation: Slide in from right, fade out

### Backend Error Response Format
```typescript
interface ErrorResponse {
  requestId: string;
  error: string;        // Error code (e.g., "VALIDATION_ERROR")
  message: string;      // Human-readable message
  details?: {           // For validation errors
    field: string;
    message: string;
  }[];
}
```

### Toast Type Definitions
```typescript
type ToastType = 'success' | 'error' | 'warning' | 'info';

interface Toast {
  id: string;
  type: ToastType;
  title: string;
  message?: string;
  duration?: number;    // 0 = persistent
  action?: {
    label: string;
    onClick: () => void;
  };
}
```

### File List (to create/modify)

**Create:**
- `/frontend/apps/ppt-web/src/components/Toast/ToastProvider.tsx` - Toast context
- `/frontend/apps/ppt-web/src/components/Toast/Toast.tsx` - Toast component
- `/frontend/apps/ppt-web/src/components/Toast/ToastContainer.tsx` - Toast stack
- `/frontend/apps/ppt-web/src/components/Toast/index.ts` - Exports
- `/frontend/apps/ppt-web/src/hooks/useToast.ts` - Toast hook
- `/frontend/apps/ppt-web/src/hooks/useNetworkStatus.ts` - Network detection
- `/frontend/apps/ppt-web/src/lib/errorHandler.ts` - Error parsing
- `/frontend/apps/ppt-web/src/components/OfflineIndicator.tsx` - Offline banner

**Modify:**
- `/frontend/apps/ppt-web/src/App.tsx` - Add providers
- `/frontend/apps/ppt-web/src/lib/api.ts` - Add error interceptor
- `/frontend/apps/ppt-web/src/components/Header.tsx` - Add offline indicator

### Accessibility Requirements
- Toast container: `role="region"`, `aria-label="Notifications"`
- Individual toasts: `role="alert"` or `role="status"`
- Success/info: `aria-live="polite"`
- Errors: `aria-live="assertive"`
- Dismiss button: Accessible label and keyboard focus

### References
- [Backend error format: backend/crates/common/src/errors.rs]
- [UC-25: Accessibility]
