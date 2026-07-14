# Story 79.2: Authentication Flow Implementation

Status: done

## Story

As a **ppt-web user**,
I want to **have a complete authentication experience with login, logout, and session management**,
So that **I can securely access the application and my data is protected**.

## Acceptance Criteria

1. **AC-1: Login Flow**
   - Given I am on the login page
   - When I enter valid email and password
   - Then I am authenticated via `/api/v1/auth/login`
   - And access/refresh tokens are stored securely
   - And I am redirected to the dashboard

2. **AC-2: Token Refresh**
   - Given my access token is about to expire (< 1 minute remaining)
   - When an API call is made
   - Then the token is automatically refreshed via `/api/v1/auth/refresh`
   - And the original request is retried with the new token

3. **AC-3: Session Expiry Handling**
   - Given my session has expired (refresh token invalid)
   - When I try to access a protected route
   - Then I am redirected to login page
   - And the intended destination is preserved as return URL

4. **AC-4: Logout**
   - Given I am logged in
   - When I click logout
   - Then `/api/v1/auth/logout` is called
   - And all tokens are cleared from storage
   - And I am redirected to login page

5. **AC-5: Protected Routes**
   - Given I am not authenticated
   - When I try to access a protected route directly
   - Then I am redirected to login
   - And after login I return to the original destination

## Tasks / Subtasks

- [x] Task 1: Create AuthContext and Provider (AC: 1, 2, 3, 4, 5)
  - [x] 1.1 Create `/frontend/apps/ppt-web/src/contexts/AuthContext.tsx`
  - [x] 1.2 Define AuthState type with `user`, `isAuthenticated`, `isLoading`
  - [x] 1.3 Implement `login`, `logout`, `refreshToken` methods
  - [x] 1.4 Add token storage using httpOnly cookies or secure localStorage
  - [x] 1.5 Create `useAuth` hook for consuming context

- [x] Task 2: Implement Login Page (AC: 1)
  - [x] 2.1 Create `/frontend/apps/ppt-web/src/pages/LoginPage.tsx`
  - [x] 2.2 Create login form with email/password fields
  - [x] 2.3 Add form validation (email format, password required)
  - [x] 2.4 Handle login errors (invalid credentials, account locked)
  - [x] 2.5 Show loading state during authentication

- [x] Task 3: Implement Token Refresh Mechanism (AC: 2)
  - [x] 3.1 Add axios response interceptor in `/frontend/apps/ppt-web/src/lib/api.ts`
  - [x] 3.2 Detect 401 responses and attempt token refresh
  - [x] 3.3 Queue requests during refresh to prevent race conditions
  - [x] 3.4 Retry failed requests after successful refresh
  - [x] 3.5 Clear auth state if refresh fails

- [x] Task 4: Implement Protected Route Wrapper (AC: 3, 5)
  - [x] 4.1 Create `/frontend/apps/ppt-web/src/components/ProtectedRoute.tsx`
  - [x] 4.2 Check `isAuthenticated` from AuthContext
  - [x] 4.3 Store current location in sessionStorage as returnUrl
  - [x] 4.4 Redirect to login if not authenticated
  - [x] 4.5 Show loading spinner while checking auth state

- [x] Task 5: Implement Logout Flow (AC: 4)
  - [x] 5.1 Create logout button in header/navigation
  - [x] 5.2 Call `/api/v1/auth/logout` on click
  - [x] 5.3 Clear all tokens from storage
  - [x] 5.4 Invalidate all React Query caches
  - [x] 5.5 Navigate to login page

- [x] Task 6: Implement Return URL Handling (AC: 1, 3, 5)
  - [x] 6.1 Capture intended URL before redirect to login
  - [x] 6.2 Store in sessionStorage as `returnUrl`
  - [x] 6.3 After successful login, redirect to returnUrl or dashboard
  - [x] 6.4 Clear returnUrl after use

## Dev Notes

### Architecture Requirements
- Use React Context for global auth state
- Implement token refresh with request queuing
- Handle concurrent refresh requests (only one refresh at a time)
- Clear sensitive data on logout

### Technical Specifications
- Access token expiry: 15 minutes
- Refresh token expiry: 7 days
- Token storage: localStorage with encryption wrapper or httpOnly cookies
- Auth endpoints:
  - POST `/api/v1/auth/login` - Returns `{ accessToken, refreshToken, user }`
  - POST `/api/v1/auth/refresh` - Returns `{ accessToken, refreshToken }`
  - POST `/api/v1/auth/logout` - Invalidates refresh token

### Token Refresh Pattern
```typescript
let isRefreshing = false;
let failedQueue: Array<{ resolve: Function; reject: Function }> = [];

const processQueue = (error: any, token: string | null) => {
  failedQueue.forEach(prom => {
    if (token) prom.resolve(token);
    else prom.reject(error);
  });
  failedQueue = [];
};

// In response interceptor for 401
if (!isRefreshing) {
  isRefreshing = true;
  try {
    const newToken = await refreshToken();
    processQueue(null, newToken);
  } catch (err) {
    processQueue(err, null);
    logout();
  } finally {
    isRefreshing = false;
  }
}
```

### File List (to create/modify)

**Create:**
- `/frontend/apps/ppt-web/src/contexts/AuthContext.tsx` - Auth context and provider
- `/frontend/apps/ppt-web/src/hooks/useAuth.ts` - Auth hook
- `/frontend/apps/ppt-web/src/pages/LoginPage.tsx` - Login page
- `/frontend/apps/ppt-web/src/components/ProtectedRoute.tsx` - Route guard

**Modify:**
- `/frontend/apps/ppt-web/src/lib/api.ts` - Add auth interceptors
- `/frontend/apps/ppt-web/src/App.tsx` - Wrap with AuthProvider, add login route
- `/frontend/apps/ppt-web/src/components/Header.tsx` - Add logout button

### Security Considerations
- Never store tokens in plain localStorage in production
- Use httpOnly cookies when possible
- Clear tokens on browser close (optional: remember me)
- Implement CSRF protection if using cookies
- Log authentication events for audit trail

### References
- [Backend: backend/servers/api-server/src/routes/auth.rs]
- [UC-14: User Account Management]
- [Backend JWT service: backend/servers/api-server/src/services/jwt.rs]
