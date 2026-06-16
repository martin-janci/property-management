# Functional Requirements Specification

> **Parent:** See `docs/CLAUDE.md` for documentation overview.

This document decomposes all 508 use cases into concrete system functions with defined inputs, outputs, and business rules.

---

## FR Taxonomy — Reconciliation (two numbering schemes)

This project uses **two distinct FR numbering schemes**. They are complementary,
not contradictory — pick the right one for the task:

| Scheme | Where | Granularity | Source of truth for |
|--------|-------|-------------|---------------------|
| **FR-XX.Y** (this document) | `docs/functional-requirements.md` | 51 categories → **256 sub-FRs** decomposing **508 UCs** | Product spec: inputs / outputs / business rules / acceptance detail |
| **FR1–FR101** | `_bmad-output/epics.md` (capability areas CA-01…CA-15) | **101 capability-level FRs** | Epic / story **delivery tracking** |

The 101 epic FRs are a coarse capability rollup; each maps to one or more
`FR-XX.Y` entries here (e.g. epic FR37–44 "Voting" ⊇ the `FR-04.*` voting
functions). For **delivery status** of the 101 FRs against the codebase, see
[`docs/EPIC_STORY_STATUS.md`](./EPIC_STORY_STATUS.md) §5–§6. **This document**
remains the source of truth for the fine-grained 256-sub-FR / 508-UC acceptance
detail.

---

## Document Conventions

- **FR-XX.Y**: Functional Requirement ID (maps to UC-XX.Y)
- **BR-XX.Y.Z**: Business Rule ID
- **Inputs**: Required parameters marked with `*`
- **Authorization**: Roles that can execute the function

---

## FR-01: Notifications

### FR-01.1: Enable Push Notifications

**Function:** `registerPushDevice()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| userId | UUID | Yes | Authenticated user ID |
| deviceToken | string | Yes | FCM/APNs device token |
| platform | enum | Yes | `ios` \| `android` \| `web` |
| deviceName | string | No | User-friendly device name |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| deviceId | UUID | Registered device identifier |
| registeredAt | datetime | Registration timestamp |

**Business Rules:**
- BR-01.1.1: User must be authenticated
- BR-01.1.2: Device token must be valid for specified platform
- BR-01.1.3: Maximum 5 devices per user
- BR-01.1.4: Duplicate device tokens update existing registration

**API Endpoint:** `POST /api/v1/notifications/devices`

**Authorization:** Owner, Tenant, Manager, Technical Manager

---

### FR-01.2: Disable Push Notifications

**Function:** `unregisterPushDevice()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| userId | UUID | Yes | Authenticated user ID |
| deviceId | UUID | Yes | Device to unregister |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| success | boolean | Whether unregistration succeeded |

**Business Rules:**
- BR-01.2.1: User can only unregister their own devices
- BR-01.2.2: Device must exist

**API Endpoint:** `DELETE /api/v1/notifications/devices/{deviceId}`

**Authorization:** Owner, Tenant, Manager, Technical Manager

---

### FR-01.3: Configure Notification Preferences

**Function:** `updateNotificationPreferences()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| userId | UUID | Yes | Authenticated user ID |
| preferences | object | Yes | Notification preference settings |
| preferences.announcements | boolean | No | Receive announcement notifications |
| preferences.faults | boolean | No | Receive fault update notifications |
| preferences.votes | boolean | No | Receive voting notifications |
| preferences.messages | boolean | No | Receive message notifications |
| preferences.payments | boolean | No | Receive payment reminders |
| preferences.emergencies | boolean | No | Receive emergency alerts (cannot be disabled) |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| preferences | object | Updated preference settings |
| updatedAt | datetime | Last update timestamp |

**Business Rules:**
- BR-01.3.1: Emergency notifications cannot be disabled
- BR-01.3.2: Default all preferences to true for new users
- BR-01.3.3: Preferences are per-user, not per-device

**API Endpoint:** `PUT /api/v1/notifications/preferences`

**Authorization:** Owner, Tenant, Manager

---

### FR-01.4: View Notification History

**Function:** `getNotificationHistory()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| userId | UUID | Yes | Authenticated user ID |
| page | integer | No | Page number (default: 1) |
| limit | integer | No | Items per page (default: 20, max: 100) |
| unreadOnly | boolean | No | Filter to unread only |
| type | string | No | Filter by notification type |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| items | array | List of notifications |
| items[].id | UUID | Notification ID |
| items[].type | string | Notification type |
| items[].title | string | Notification title |
| items[].body | string | Notification body |
| items[].isRead | boolean | Read status |
| items[].createdAt | datetime | Creation timestamp |
| items[].link | string | Deep link to related content |
| pagination | object | Pagination metadata |
| unreadCount | integer | Total unread count |

**Business Rules:**
- BR-01.4.1: Return only user's own notifications
- BR-01.4.2: Sort by createdAt descending (newest first)
- BR-01.4.3: Retain notifications for 90 days

**API Endpoint:** `GET /api/v1/notifications`

**Authorization:** Owner, Tenant, Manager

---

### FR-01.5: Mark Notification as Read

**Function:** `markNotificationRead()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| userId | UUID | Yes | Authenticated user ID |
| notificationId | UUID | Yes | Notification to mark |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| success | boolean | Whether operation succeeded |
| unreadCount | integer | Updated unread count |

**Business Rules:**
- BR-01.5.1: User can only mark their own notifications
- BR-01.5.2: Already-read notifications return success without change

**API Endpoint:** `POST /api/v1/notifications/{notificationId}/read`

**Authorization:** Owner, Tenant, Manager

---

### FR-01.6: Mark All Notifications as Read

**Function:** `markAllNotificationsRead()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| userId | UUID | Yes | Authenticated user ID |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| markedCount | integer | Number of notifications marked |
| unreadCount | integer | Should be 0 |

**Business Rules:**
- BR-01.6.1: Marks all unread notifications for the user
- BR-01.6.2: Atomic operation (all or none)

**API Endpoint:** `POST /api/v1/notifications/read-all`

**Authorization:** Owner, Tenant, Manager

---

## FR-02: Announcements

### FR-02.1: Create Announcement

**Function:** `createAnnouncement()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Target building ID |
| title | string | Yes | Announcement title (max 200 chars) |
| content | string | Yes | Announcement body (max 10000 chars) |
| status | enum | No | `draft` \| `published` (default: draft) |
| visibility | enum | No | `all` \| `owners` \| `tenants` (default: all) |
| attachments | array | No | File attachment IDs |
| notifyUsers | boolean | No | Send push notification (default: true) |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| id | UUID | Created announcement ID |
| createdAt | datetime | Creation timestamp |
| authorId | UUID | Creating user ID |

**Business Rules:**
- BR-02.1.1: Only managers can create announcements
- BR-02.1.2: Building must belong to tenant
- BR-02.1.3: Title required, content required
- BR-02.1.4: Max 10 attachments per announcement
- BR-02.1.5: Attachments max 10MB each

**API Endpoint:** `POST /api/v1/buildings/{buildingId}/announcements`

**Authorization:** Manager

---

### FR-02.2: Create Owners' Meeting Announcement

**Function:** `createMeetingAnnouncement()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Target building ID |
| title | string | Yes | Meeting title |
| content | string | Yes | Meeting details/agenda |
| meetingDate | datetime | Yes | Meeting date and time |
| location | string | Yes | Meeting location |
| isVirtual | boolean | No | Virtual meeting flag |
| virtualMeetingUrl | string | No | Video conference URL |
| attachments | array | No | Agenda documents |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| id | UUID | Created announcement ID |
| calendarEventId | UUID | Associated calendar event |

**Business Rules:**
- BR-02.2.1: Meeting date must be in the future
- BR-02.2.2: Virtual meetings require URL
- BR-02.2.3: Automatically creates calendar event
- BR-02.2.4: Sends notification to all owners

**API Endpoint:** `POST /api/v1/buildings/{buildingId}/announcements/meeting`

**Authorization:** Manager

---

### FR-02.3: Search Announcements

**Function:** `searchAnnouncements()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| query | string | Yes | Search query (min 2 chars) |
| page | integer | No | Page number |
| limit | integer | No | Items per page |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| items | array | Matching announcements |
| pagination | object | Pagination metadata |

**Business Rules:**
- BR-02.3.1: Full-text search on title and content
- BR-02.3.2: Respect visibility settings for user role
- BR-02.3.3: Only return published announcements for non-managers

**API Endpoint:** `GET /api/v1/buildings/{buildingId}/announcements/search`

**Authorization:** Owner, Tenant, Manager

---

### FR-02.4: Filter Announcements by Status

**Function:** `listAnnouncements()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| status | enum | No | `published` \| `draft` \| `archived` |
| page | integer | No | Page number |
| limit | integer | No | Items per page |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| items | array | Filtered announcements |
| items[].id | UUID | Announcement ID |
| items[].title | string | Title |
| items[].status | string | Current status |
| items[].isPinned | boolean | Pin status |
| items[].createdAt | datetime | Creation date |
| items[].authorName | string | Author display name |
| pagination | object | Pagination metadata |

**Business Rules:**
- BR-02.4.1: Non-managers only see published announcements
- BR-02.4.2: Pinned announcements appear first
- BR-02.4.3: Sort by createdAt descending within groups

**API Endpoint:** `GET /api/v1/buildings/{buildingId}/announcements`

**Authorization:** Owner, Tenant, Manager

---

### FR-02.5: Filter Own Announcements

**Function:** `listMyAnnouncements()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| authorId | UUID | Yes | Current user ID |
| status | enum | No | Filter by status |
| page | integer | No | Page number |
| limit | integer | No | Items per page |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| items | array | User's announcements |
| pagination | object | Pagination metadata |

**Business Rules:**
- BR-02.5.1: Only return announcements created by user
- BR-02.5.2: Include all statuses (draft, published, archived)

**API Endpoint:** `GET /api/v1/announcements/mine`

**Authorization:** Manager

---

### FR-02.6: View Announcement Detail

**Function:** `getAnnouncement()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| announcementId | UUID | Yes | Announcement ID |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| id | UUID | Announcement ID |
| title | string | Title |
| content | string | Full content |
| status | string | Current status |
| visibility | string | Visibility setting |
| isPinned | boolean | Pin status |
| author | object | Author details |
| attachments | array | Attached files |
| commentCount | integer | Number of comments |
| createdAt | datetime | Creation timestamp |
| updatedAt | datetime | Last update timestamp |
| scheduledAt | datetime | Scheduled publication time |

**Business Rules:**
- BR-02.6.1: Check visibility against user role
- BR-02.6.2: Draft announcements only visible to author
- BR-02.6.3: Increment view count

**API Endpoint:** `GET /api/v1/announcements/{announcementId}`

**Authorization:** Owner, Tenant, Manager

---

### FR-02.7: Comment on Announcement

**Function:** `addAnnouncementComment()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| announcementId | UUID | Yes | Announcement ID |
| content | string | Yes | Comment text (max 2000 chars) |
| parentId | UUID | No | Parent comment ID for replies |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| id | UUID | Created comment ID |
| content | string | Comment content |
| authorId | UUID | Commenter ID |
| authorName | string | Commenter name |
| createdAt | datetime | Creation timestamp |

**Business Rules:**
- BR-02.7.1: Only published announcements can be commented
- BR-02.7.2: Max nesting depth: 2 levels
- BR-02.7.3: Notify announcement author of new comments

**API Endpoint:** `POST /api/v1/announcements/{announcementId}/comments`

**Authorization:** Owner, Tenant, Manager

---

### FR-02.8: View Announcement Comments

**Function:** `getAnnouncementComments()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| announcementId | UUID | Yes | Announcement ID |
| page | integer | No | Page number |
| limit | integer | No | Items per page |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| items | array | Comments with replies nested |
| pagination | object | Pagination metadata |

**Business Rules:**
- BR-02.8.1: Sort by createdAt ascending (oldest first)
- BR-02.8.2: Include nested replies

**API Endpoint:** `GET /api/v1/announcements/{announcementId}/comments`

**Authorization:** Owner, Tenant, Manager

---

### FR-02.9: Edit Announcement

**Function:** `updateAnnouncement()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| announcementId | UUID | Yes | Announcement ID |
| title | string | No | Updated title |
| content | string | No | Updated content |
| visibility | enum | No | Updated visibility |
| attachments | array | No | Updated attachments |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| id | UUID | Announcement ID |
| updatedAt | datetime | Update timestamp |

**Business Rules:**
- BR-02.9.1: Only author or admin can edit
- BR-02.9.2: Cannot change status via this endpoint
- BR-02.9.3: Editing published announcement sends update notification

**API Endpoint:** `PUT /api/v1/announcements/{announcementId}`

**Authorization:** Manager

---

### FR-02.10: Delete Announcement

**Function:** `deleteAnnouncement()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| announcementId | UUID | Yes | Announcement ID |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| success | boolean | Whether deletion succeeded |

**Business Rules:**
- BR-02.10.1: Only author or admin can delete
- BR-02.10.2: Soft delete - marks as deleted
- BR-02.10.3: Deletes associated comments

**API Endpoint:** `DELETE /api/v1/announcements/{announcementId}`

**Authorization:** Manager

---

### FR-02.11: Archive Announcement

**Function:** `archiveAnnouncement()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| announcementId | UUID | Yes | Announcement ID |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| id | UUID | Announcement ID |
| status | string | "archived" |
| archivedAt | datetime | Archive timestamp |

**Business Rules:**
- BR-02.11.1: Only published announcements can be archived
- BR-02.11.2: Archived announcements remain searchable
- BR-02.11.3: Auto-archive announcements older than 1 year

**API Endpoint:** `POST /api/v1/announcements/{announcementId}/archive`

**Authorization:** Manager

---

### FR-02.12: Pin Announcement to Top

**Function:** `pinAnnouncement()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| announcementId | UUID | Yes | Announcement ID |
| isPinned | boolean | Yes | Pin or unpin |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| id | UUID | Announcement ID |
| isPinned | boolean | Updated pin status |

**Business Rules:**
- BR-02.12.1: Maximum 3 pinned announcements per building
- BR-02.12.2: Only published announcements can be pinned

**API Endpoint:** `POST /api/v1/announcements/{announcementId}/pin`

**Authorization:** Manager

---

### FR-02.13: Schedule Announcement Publication

**Function:** `scheduleAnnouncement()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| announcementId | UUID | Yes | Announcement ID |
| scheduledAt | datetime | Yes | Publication date/time |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| id | UUID | Announcement ID |
| status | string | "scheduled" |
| scheduledAt | datetime | Scheduled time |

**Business Rules:**
- BR-02.13.1: Only draft announcements can be scheduled
- BR-02.13.2: Scheduled time must be in future
- BR-02.13.3: System publishes automatically at scheduled time

**API Endpoint:** `POST /api/v1/announcements/{announcementId}/schedule`

**Authorization:** Manager

---

## FR-03: Faults

### FR-03.1: Report Fault

**Function:** `createFault()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| unitId | UUID | No | Specific unit (if applicable) |
| title | string | Yes | Fault title (max 200 chars) |
| description | string | Yes | Detailed description (max 5000 chars) |
| category | enum | Yes | `plumbing` \| `electrical` \| `hvac` \| `structural` \| `elevator` \| `common_area` \| `other` |
| location | string | Yes | Location in building |
| photos | array | No | Photo attachment IDs (max 5) |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| id | UUID | Created fault ID |
| referenceNumber | string | Human-readable reference (e.g., F-2024-0001) |
| status | string | "new" |
| createdAt | datetime | Creation timestamp |

**Business Rules:**
- BR-03.1.1: Reference number auto-generated per tenant
- BR-03.1.2: Initial status is "new"
- BR-03.1.3: Notify building managers
- BR-03.1.4: Photos max 10MB each

**API Endpoint:** `POST /api/v1/buildings/{buildingId}/faults`

**Authorization:** Owner, Tenant, Manager

---

### FR-03.2: Search Faults

**Function:** `searchFaults()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| query | string | Yes | Search query |
| page | integer | No | Page number |
| limit | integer | No | Items per page |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| items | array | Matching faults |
| pagination | object | Pagination metadata |

**Business Rules:**
- BR-03.2.1: Search title, description, reference number
- BR-03.2.2: Owners/tenants see only own unit faults + common area
- BR-03.2.3: Managers see all building faults

**API Endpoint:** `GET /api/v1/buildings/{buildingId}/faults/search`

**Authorization:** Owner, Tenant, Manager

---

### FR-03.3: Filter Faults by Status

**Function:** `listFaults()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| status | enum | No | `new` \| `in_progress` \| `resolved` \| `closed` |
| priority | enum | No | `low` \| `medium` \| `high` \| `critical` |
| category | enum | No | Fault category filter |
| page | integer | No | Page number |
| limit | integer | No | Items per page |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| items | array | Filtered faults |
| items[].id | UUID | Fault ID |
| items[].referenceNumber | string | Reference number |
| items[].title | string | Title |
| items[].status | string | Current status |
| items[].priority | string | Priority level |
| items[].createdAt | datetime | Creation date |
| items[].assignedTo | object | Assigned technician |
| pagination | object | Pagination metadata |

**Business Rules:**
- BR-03.3.1: Sort by priority (critical first), then by createdAt
- BR-03.3.2: Apply visibility rules based on user role

**API Endpoint:** `GET /api/v1/buildings/{buildingId}/faults`

**Authorization:** Owner, Tenant, Manager

---

### FR-03.4: Filter Own Reported Faults

**Function:** `listMyFaults()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| reporterId | UUID | Yes | Current user ID |
| status | enum | No | Status filter |
| page | integer | No | Page number |
| limit | integer | No | Items per page |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| items | array | User's reported faults |
| pagination | object | Pagination metadata |

**Business Rules:**
- BR-03.4.1: Only return faults reported by user
- BR-03.4.2: Include all buildings user has access to

**API Endpoint:** `GET /api/v1/faults/mine`

**Authorization:** Owner, Tenant

---

### FR-03.5: View Fault Detail

**Function:** `getFault()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| faultId | UUID | Yes | Fault ID |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| id | UUID | Fault ID |
| referenceNumber | string | Reference number |
| title | string | Title |
| description | string | Full description |
| category | string | Category |
| location | string | Location |
| status | string | Current status |
| priority | string | Priority level |
| reporter | object | Reporter details |
| assignedTo | object | Assigned technician |
| photos | array | Attached photos |
| communications | array | Communication history |
| statusHistory | array | Status change history |
| createdAt | datetime | Creation timestamp |
| updatedAt | datetime | Last update timestamp |
| resolvedAt | datetime | Resolution timestamp |

**Business Rules:**
- BR-03.5.1: Check user access to fault
- BR-03.5.2: Include full communication history

**API Endpoint:** `GET /api/v1/faults/{faultId}`

**Authorization:** Owner, Tenant, Manager

---

### FR-03.6: Update Fault Status

**Function:** `updateFaultStatus()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| faultId | UUID | Yes | Fault ID |
| status | enum | Yes | `new` \| `in_progress` \| `resolved` \| `closed` |
| note | string | No | Status change note |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| id | UUID | Fault ID |
| status | string | New status |
| updatedAt | datetime | Update timestamp |

**Business Rules:**
- BR-03.6.1: Valid transitions: new→in_progress, in_progress→resolved, resolved→closed
- BR-03.6.2: Notify reporter of status change
- BR-03.6.3: Record status change in history

**API Endpoint:** `POST /api/v1/faults/{faultId}/status`

**Authorization:** Manager, Technical Manager

---

### FR-03.7: Communicate on Fault

**Function:** `addFaultCommunication()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| faultId | UUID | Yes | Fault ID |
| message | string | Yes | Communication message (max 2000 chars) |
| attachments | array | No | File attachments |
| isInternal | boolean | No | Internal note (manager only) |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| id | UUID | Communication ID |
| message | string | Message content |
| authorId | UUID | Author ID |
| authorName | string | Author name |
| createdAt | datetime | Timestamp |

**Business Rules:**
- BR-03.7.1: Internal notes only visible to managers
- BR-03.7.2: Notify other participants
- BR-03.7.3: Max 5 attachments per message

**API Endpoint:** `POST /api/v1/faults/{faultId}/communications`

**Authorization:** Owner, Tenant, Manager

---

### FR-03.8: Assign Fault to Technical Manager

**Function:** `assignFault()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| faultId | UUID | Yes | Fault ID |
| assigneeId | UUID | Yes | Technical manager ID |
| note | string | No | Assignment note |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| id | UUID | Fault ID |
| assignedTo | object | Assigned technician details |
| assignedAt | datetime | Assignment timestamp |

**Business Rules:**
- BR-03.8.1: Assignee must be technical manager in same tenant
- BR-03.8.2: Notify assignee of new assignment
- BR-03.8.3: Auto-change status to in_progress if currently new

**API Endpoint:** `POST /api/v1/faults/{faultId}/assign`

**Authorization:** Manager

---

### FR-03.9: Set Fault Priority

**Function:** `setFaultPriority()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| faultId | UUID | Yes | Fault ID |
| priority | enum | Yes | `low` \| `medium` \| `high` \| `critical` |
| reason | string | No | Priority change reason |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| id | UUID | Fault ID |
| priority | string | New priority |

**Business Rules:**
- BR-03.9.1: Critical priority triggers immediate notification to all managers
- BR-03.9.2: Record priority change in history

**API Endpoint:** `POST /api/v1/faults/{faultId}/priority`

**Authorization:** Manager, Technical Manager

---

### FR-03.10: Close Fault

**Function:** `closeFault()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| faultId | UUID | Yes | Fault ID |
| resolution | string | Yes | Resolution summary |
| closureNote | string | No | Additional closure notes |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| id | UUID | Fault ID |
| status | string | "closed" |
| closedAt | datetime | Closure timestamp |
| resolutionTime | integer | Hours from creation to closure |

**Business Rules:**
- BR-03.10.1: Must be in resolved status to close
- BR-03.10.2: Notify reporter of closure
- BR-03.10.3: Calculate resolution metrics

**API Endpoint:** `POST /api/v1/faults/{faultId}/close`

**Authorization:** Manager, Technical Manager

---

### FR-03.11: Reopen Fault

**Function:** `reopenFault()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| faultId | UUID | Yes | Fault ID |
| reason | string | Yes | Reason for reopening |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| id | UUID | Fault ID |
| status | string | "in_progress" |
| reopenedAt | datetime | Reopen timestamp |
| reopenCount | integer | Number of times reopened |

**Business Rules:**
- BR-03.11.1: Only closed faults can be reopened
- BR-03.11.2: Cannot reopen faults older than 90 days
- BR-03.11.3: Escalate after 3rd reopen

**API Endpoint:** `POST /api/v1/faults/{faultId}/reopen`

**Authorization:** Owner, Tenant, Manager

---

### FR-03.12: Escalate Fault

**Function:** `escalateFault()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| faultId | UUID | Yes | Fault ID |
| reason | string | Yes | Escalation reason |
| escalateTo | UUID | No | Specific manager to escalate to |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| id | UUID | Fault ID |
| escalatedAt | datetime | Escalation timestamp |
| escalatedTo | object | Escalation target |

**Business Rules:**
- BR-03.12.1: Escalation notifies organization admin
- BR-03.12.2: Automatically increases priority
- BR-03.12.3: Record escalation in history

**API Endpoint:** `POST /api/v1/faults/{faultId}/escalate`

**Authorization:** Manager, Technical Manager

---

### FR-03.13: Add Photo to Existing Fault

**Function:** `addFaultPhoto()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| faultId | UUID | Yes | Fault ID |
| photo | file | Yes | Photo file |
| caption | string | No | Photo caption |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| id | UUID | Photo ID |
| url | string | Photo URL |
| thumbnailUrl | string | Thumbnail URL |
| uploadedAt | datetime | Upload timestamp |

**Business Rules:**
- BR-03.13.1: Max 10 photos per fault total
- BR-03.13.2: Max 10MB per photo
- BR-03.13.3: Accepted formats: jpg, png, heic

**API Endpoint:** `POST /api/v1/faults/{faultId}/photos`

**Authorization:** Owner, Tenant, Manager, Technical Manager

---

### FR-03.14: Request Fault Update

**Function:** `requestFaultUpdate()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| faultId | UUID | Yes | Fault ID |
| message | string | No | Optional message |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| requestedAt | datetime | Request timestamp |

**Business Rules:**
- BR-03.14.1: Only reporter can request update
- BR-03.14.2: Rate limit: 1 request per 24 hours
- BR-03.14.3: Notify assigned manager

**API Endpoint:** `POST /api/v1/faults/{faultId}/request-update`

**Authorization:** Owner, Tenant

---

## FR-04: Voting and Polls

### FR-04.1: Search Votes

**Function:** `searchVotes()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| query | string | Yes | Search query |
| page | integer | No | Page number |
| limit | integer | No | Items per page |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| items | array | Matching votes |
| pagination | object | Pagination metadata |

**Business Rules:**
- BR-04.1.1: Search question text
- BR-04.1.2: Only show votes user is eligible for

**API Endpoint:** `GET /api/v1/buildings/{buildingId}/votes/search`

**Authorization:** Owner, Manager

---

### FR-04.2: Filter Votes by Status

**Function:** `listVotes()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| status | enum | No | `draft` \| `active` \| `completed` \| `cancelled` |
| page | integer | No | Page number |
| limit | integer | No | Items per page |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| items | array | Filtered votes |
| items[].id | UUID | Vote ID |
| items[].question | string | Vote question |
| items[].status | string | Current status |
| items[].startDate | datetime | Voting start |
| items[].endDate | datetime | Voting deadline |
| items[].participationRate | number | Percentage of eligible voters who voted |
| pagination | object | Pagination metadata |

**Business Rules:**
- BR-04.2.1: Owners see active and completed votes
- BR-04.2.2: Managers see all including drafts

**API Endpoint:** `GET /api/v1/buildings/{buildingId}/votes`

**Authorization:** Owner, Manager

---

### FR-04.3: View Vote Detail

**Function:** `getVote()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| voteId | UUID | Yes | Vote ID |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| id | UUID | Vote ID |
| question | string | Vote question |
| description | string | Detailed description |
| options | array | Voting options |
| status | string | Current status |
| startDate | datetime | Voting start |
| endDate | datetime | Voting deadline |
| allowChangeVote | boolean | Can voters change vote |
| showResultsBeforeEnd | boolean | Show live results |
| results | object | Results (if allowed/completed) |
| userVote | object | Current user's vote (if cast) |
| eligibleVoters | integer | Number of eligible voters |
| votesCount | integer | Number of votes cast |

**Business Rules:**
- BR-04.3.1: Results only shown if showResultsBeforeEnd or status=completed
- BR-04.3.2: User's own vote always shown to them

**API Endpoint:** `GET /api/v1/votes/{voteId}`

**Authorization:** Owner, Manager

---

### FR-04.4: Cast Vote

**Function:** `castVote()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| voteId | UUID | Yes | Vote ID |
| optionId | UUID | Yes | Selected option ID |
| voterId | UUID | Yes | Voter ID (owner or delegate) |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| voteId | UUID | Vote ID |
| optionId | UUID | Selected option |
| votedAt | datetime | Vote timestamp |
| isDelegate | boolean | Whether voting as delegate |

**Business Rules:**
- BR-04.4.1: Only owners or their delegates can vote
- BR-04.4.2: Must be within voting period
- BR-04.4.3: One vote per unit
- BR-04.4.4: Cannot vote if already voted (unless allowChangeVote)

**API Endpoint:** `POST /api/v1/votes/{voteId}/cast`

**Authorization:** Owner

---

### FR-04.5: View Vote Results

**Function:** `getVoteResults()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| voteId | UUID | Yes | Vote ID |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| voteId | UUID | Vote ID |
| status | string | Vote status |
| options | array | Options with vote counts |
| options[].id | UUID | Option ID |
| options[].text | string | Option text |
| options[].count | integer | Number of votes |
| options[].percentage | number | Percentage of votes |
| totalVotes | integer | Total votes cast |
| participationRate | number | Participation percentage |
| outcome | string | Winning option (if applicable) |

**Business Rules:**
- BR-04.5.1: Full results only after vote completed
- BR-04.5.2: Partial results if showResultsBeforeEnd enabled

**API Endpoint:** `GET /api/v1/votes/{voteId}/results`

**Authorization:** Owner, Manager

---

### FR-04.6: Comment on Vote

**Function:** `addVoteComment()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| voteId | UUID | Yes | Vote ID |
| content | string | Yes | Comment text (max 2000 chars) |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| id | UUID | Comment ID |
| content | string | Comment content |
| authorId | UUID | Commenter ID |
| authorName | string | Commenter name |
| createdAt | datetime | Creation timestamp |

**Business Rules:**
- BR-04.6.1: Comments visible to all eligible voters
- BR-04.6.2: Comments can be added during and after voting

**API Endpoint:** `POST /api/v1/votes/{voteId}/comments`

**Authorization:** Owner, Manager

---

### FR-04.7: Create Vote

**Function:** `createVote()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| question | string | Yes | Vote question (max 500 chars) |
| description | string | No | Detailed description |
| options | array | Yes | Voting options (min 2, max 10) |
| options[].text | string | Yes | Option text |
| startDate | datetime | Yes | Voting start date/time |
| endDate | datetime | Yes | Voting end date/time |
| allowChangeVote | boolean | No | Allow vote changes (default: false) |
| showResultsBeforeEnd | boolean | No | Show live results (default: false) |
| requiredParticipation | number | No | Minimum participation % for validity |
| attachments | array | No | Supporting documents |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| id | UUID | Created vote ID |
| status | string | "draft" |
| createdAt | datetime | Creation timestamp |

**Business Rules:**
- BR-04.7.1: Start date must be in future
- BR-04.7.2: End date must be after start date
- BR-04.7.3: Minimum 2 options required
- BR-04.7.4: Initial status is draft

**API Endpoint:** `POST /api/v1/buildings/{buildingId}/votes`

**Authorization:** Manager

---

### FR-04.8: Edit Vote

**Function:** `updateVote()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| voteId | UUID | Yes | Vote ID |
| question | string | No | Updated question |
| description | string | No | Updated description |
| options | array | No | Updated options |
| startDate | datetime | No | Updated start date |
| endDate | datetime | No | Updated end date |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| id | UUID | Vote ID |
| updatedAt | datetime | Update timestamp |

**Business Rules:**
- BR-04.8.1: Cannot edit active or completed votes
- BR-04.8.2: Only draft votes can be edited

**API Endpoint:** `PUT /api/v1/votes/{voteId}`

**Authorization:** Manager

---

### FR-04.9: Cancel Vote

**Function:** `cancelVote()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| voteId | UUID | Yes | Vote ID |
| reason | string | Yes | Cancellation reason |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| id | UUID | Vote ID |
| status | string | "cancelled" |
| cancelledAt | datetime | Cancellation timestamp |

**Business Rules:**
- BR-04.9.1: Can cancel draft or active votes
- BR-04.9.2: Cannot cancel completed votes
- BR-04.9.3: Notify all eligible voters

**API Endpoint:** `POST /api/v1/votes/{voteId}/cancel`

**Authorization:** Manager

---

### FR-04.10: Extend Voting Deadline

**Function:** `extendVoteDeadline()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| voteId | UUID | Yes | Vote ID |
| newEndDate | datetime | Yes | New end date |
| reason | string | No | Extension reason |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| id | UUID | Vote ID |
| endDate | datetime | Updated end date |

**Business Rules:**
- BR-04.10.1: New date must be after current end date
- BR-04.10.2: Can only extend active votes
- BR-04.10.3: Notify all eligible voters

**API Endpoint:** `POST /api/v1/votes/{voteId}/extend`

**Authorization:** Manager

---

### FR-04.11: Delegate Vote (Proxy Voting)

**Function:** `delegateVote()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| voteId | UUID | Yes | Vote ID |
| delegatorId | UUID | Yes | Owner delegating vote |
| delegateId | UUID | Yes | Person receiving delegation |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| delegationId | UUID | Delegation ID |
| voteId | UUID | Vote ID |
| delegatorId | UUID | Delegator |
| delegateId | UUID | Delegate |
| createdAt | datetime | Delegation timestamp |

**Business Rules:**
- BR-04.11.1: Delegate must be an owner in same building
- BR-04.11.2: Cannot delegate to self
- BR-04.11.3: No circular delegations
- BR-04.11.4: Delegation valid only for specific vote

**API Endpoint:** `POST /api/v1/votes/{voteId}/delegate`

**Authorization:** Owner

---

### FR-04.12: Send Vote Reminder

**Function:** `sendVoteReminder()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| voteId | UUID | Yes | Vote ID |
| message | string | No | Custom reminder message |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| sentCount | integer | Number of reminders sent |
| sentAt | datetime | Send timestamp |

**Business Rules:**
- BR-04.12.1: Only send to owners who haven't voted
- BR-04.12.2: Rate limit: 1 reminder per vote per 24 hours

**API Endpoint:** `POST /api/v1/votes/{voteId}/remind`

**Authorization:** Manager

---

### FR-04.13: Export Vote Results

**Function:** `exportVoteResults()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| voteId | UUID | Yes | Vote ID |
| format | enum | Yes | `pdf` \| `xlsx` |
| includeVoterList | boolean | No | Include who voted (not how) |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| downloadUrl | string | Temporary download URL |
| expiresAt | datetime | URL expiration |

**Business Rules:**
- BR-04.13.1: Only export completed votes
- BR-04.13.2: Voter list shows who voted, not their choice (secret ballot)

**API Endpoint:** `GET /api/v1/votes/{voteId}/export`

**Authorization:** Manager

---

### FR-04.14: Change Vote

**Function:** `changeVote()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| voteId | UUID | Yes | Vote ID |
| newOptionId | UUID | Yes | New selected option |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| voteId | UUID | Vote ID |
| optionId | UUID | New selected option |
| changedAt | datetime | Change timestamp |
| changeCount | integer | Number of times changed |

**Business Rules:**
- BR-04.14.1: Only allowed if allowChangeVote is true
- BR-04.14.2: Must be within voting period
- BR-04.14.3: Track change count for audit

**API Endpoint:** `PUT /api/v1/votes/{voteId}/cast`

**Authorization:** Owner

---

## FR-05: Messages

### FR-05.1: Create New Message

**Function:** `createConversation()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| recipientIds | array | Yes | Recipient user IDs |
| subject | string | No | Conversation subject |
| message | string | Yes | Initial message text |
| attachments | array | No | File attachments |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| conversationId | UUID | Created conversation ID |
| messageId | UUID | Initial message ID |
| createdAt | datetime | Creation timestamp |

**Business Rules:**
- BR-05.1.1: Recipients must be in same building or organization
- BR-05.1.2: Max 10 recipients for group conversations
- BR-05.1.3: Notify recipients via push notification

**API Endpoint:** `POST /api/v1/messages/conversations`

**Authorization:** Owner, Tenant, Manager

---

### FR-05.2: Search Conversations

**Function:** `searchConversations()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| userId | UUID | Yes | Current user ID |
| query | string | Yes | Search query |
| page | integer | No | Page number |
| limit | integer | No | Items per page |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| items | array | Matching conversations |
| pagination | object | Pagination metadata |

**Business Rules:**
- BR-05.2.1: Search participant names and message content
- BR-05.2.2: Only return user's own conversations

**API Endpoint:** `GET /api/v1/messages/conversations/search`

**Authorization:** Owner, Tenant, Manager

---

### FR-05.3: View Conversation List

**Function:** `listConversations()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| userId | UUID | Yes | Current user ID |
| filter | enum | No | `all` \| `unread` \| `archived` |
| page | integer | No | Page number |
| limit | integer | No | Items per page |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| items | array | Conversations |
| items[].id | UUID | Conversation ID |
| items[].participants | array | Participant names |
| items[].lastMessage | object | Last message preview |
| items[].unreadCount | integer | Unread message count |
| items[].updatedAt | datetime | Last activity |
| pagination | object | Pagination metadata |
| totalUnread | integer | Total unread across all conversations |

**Business Rules:**
- BR-05.3.1: Sort by updatedAt descending
- BR-05.3.2: Truncate last message to 100 chars

**API Endpoint:** `GET /api/v1/messages/conversations`

**Authorization:** Owner, Tenant, Manager

---

### FR-05.4: View Conversation Detail

**Function:** `getConversation()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| conversationId | UUID | Yes | Conversation ID |
| page | integer | No | Page number for messages |
| limit | integer | No | Messages per page |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| id | UUID | Conversation ID |
| participants | array | All participants |
| messages | array | Messages in conversation |
| messages[].id | UUID | Message ID |
| messages[].content | string | Message text |
| messages[].senderId | UUID | Sender ID |
| messages[].senderName | string | Sender name |
| messages[].attachments | array | Message attachments |
| messages[].readBy | array | Users who read message |
| messages[].createdAt | datetime | Message timestamp |
| pagination | object | Message pagination |

**Business Rules:**
- BR-05.4.1: Mark conversation as read on view
- BR-05.4.2: Sort messages by createdAt ascending

**API Endpoint:** `GET /api/v1/messages/conversations/{conversationId}`

**Authorization:** Owner, Tenant, Manager

---

### FR-05.5: Send Message

**Function:** `sendMessage()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| conversationId | UUID | Yes | Conversation ID |
| content | string | Yes | Message text (max 5000 chars) |
| attachments | array | No | File attachments (max 5) |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| id | UUID | Message ID |
| content | string | Message content |
| attachments | array | Attachment info |
| createdAt | datetime | Send timestamp |

**Business Rules:**
- BR-05.5.1: Must be participant in conversation
- BR-05.5.2: Notify other participants
- BR-05.5.3: Queue if offline (offline support)

**API Endpoint:** `POST /api/v1/messages/conversations/{conversationId}/messages`

**Authorization:** Owner, Tenant, Manager

---

### FR-05.6: Delete Message

**Function:** `deleteMessage()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| conversationId | UUID | Yes | Conversation ID |
| messageId | UUID | Yes | Message ID |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| success | boolean | Whether deletion succeeded |

**Business Rules:**
- BR-05.6.1: Only sender can delete
- BR-05.6.2: Delete within 24 hours only
- BR-05.6.3: Shows "Message deleted" placeholder

**API Endpoint:** `DELETE /api/v1/messages/conversations/{conversationId}/messages/{messageId}`

**Authorization:** Owner, Tenant, Manager

---

### FR-05.7: Delete Conversation

**Function:** `deleteConversation()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| conversationId | UUID | Yes | Conversation ID |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| success | boolean | Whether deletion succeeded |

**Business Rules:**
- BR-05.7.1: Only deletes for requesting user
- BR-05.7.2: Other participants still see conversation
- BR-05.7.3: Soft delete - can be restored

**API Endpoint:** `DELETE /api/v1/messages/conversations/{conversationId}`

**Authorization:** Owner, Tenant, Manager

---

### FR-05.8: Create Group Conversation

**Function:** `createGroupConversation()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| name | string | Yes | Group name |
| recipientIds | array | Yes | Participant user IDs (min 2) |
| message | string | Yes | Initial message |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| conversationId | UUID | Created group ID |
| name | string | Group name |
| participants | array | All participants |

**Business Rules:**
- BR-05.8.1: Minimum 3 total participants (creator + 2)
- BR-05.8.2: Maximum 50 participants
- BR-05.8.3: Only managers can create building-wide groups

**API Endpoint:** `POST /api/v1/messages/groups`

**Authorization:** Manager

---

### FR-05.9: Attach File to Message

**Function:** `uploadMessageAttachment()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| file | file | Yes | File to upload |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| attachmentId | UUID | Attachment ID |
| filename | string | Original filename |
| mimeType | string | File MIME type |
| size | integer | File size in bytes |
| url | string | Download URL |

**Business Rules:**
- BR-05.9.1: Max file size 25MB
- BR-05.9.2: Allowed types: images, documents, pdfs
- BR-05.9.3: Virus scan before accepting

**API Endpoint:** `POST /api/v1/messages/attachments`

**Authorization:** Owner, Tenant, Manager

---

### FR-05.10: View Read Receipt

**Function:** `getMessageReadReceipts()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| conversationId | UUID | Yes | Conversation ID |
| messageId | UUID | Yes | Message ID |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| messageId | UUID | Message ID |
| readBy | array | Users who read the message |
| readBy[].userId | UUID | User ID |
| readBy[].userName | string | User name |
| readBy[].readAt | datetime | Read timestamp |

**Business Rules:**
- BR-05.10.1: Only sender can view read receipts
- BR-05.10.2: Track read status per participant

**API Endpoint:** `GET /api/v1/messages/conversations/{conversationId}/messages/{messageId}/read`

**Authorization:** Owner, Tenant, Manager

---

### FR-05.11: Archive Conversation

**Function:** `archiveConversation()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| conversationId | UUID | Yes | Conversation ID |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| conversationId | UUID | Conversation ID |
| archivedAt | datetime | Archive timestamp |

**Business Rules:**
- BR-05.11.1: Archives for requesting user only
- BR-05.11.2: New messages unarchive automatically

**API Endpoint:** `POST /api/v1/messages/conversations/{conversationId}/archive`

**Authorization:** Owner, Tenant, Manager

---

## FR-14: User Account Management

### FR-14.1: Register Account

**Function:** `registerUser()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| invitationCode | string | Yes | Invitation code or token |
| email | string | Yes | User email address |
| password | string | Yes | Password (min 8 chars) |
| displayName | string | Yes | Display name |
| phone | string | No | Phone number |
| acceptTerms | boolean | Yes | Terms acceptance |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| userId | UUID | Created user ID |
| email | string | User email |
| accessToken | string | JWT access token |
| refreshToken | string | JWT refresh token |

**Business Rules:**
- BR-14.1.1: Invitation must be valid and not expired
- BR-14.1.2: Email must be unique
- BR-14.1.3: Password: min 8 chars, 1 uppercase, 1 lowercase, 1 number
- BR-14.1.4: Terms acceptance required

**API Endpoint:** `POST /api/v1/auth/register`

**Authorization:** Public (with invitation)

---

### FR-14.2: Login

**Function:** `login()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| email | string | Yes | User email |
| password | string | Yes | User password |
| deviceInfo | object | No | Device information |
| twoFactorCode | string | No | 2FA code if enabled |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| accessToken | string | JWT access token |
| refreshToken | string | JWT refresh token |
| expiresIn | integer | Token expiry in seconds |
| user | object | User profile data |
| tenants | array | Available tenants/organizations |
| requiresTwoFactor | boolean | Whether 2FA is needed |

**Business Rules:**
- BR-14.2.1: Lock account after 5 failed attempts for 15 minutes
- BR-14.2.2: Require 2FA if enabled
- BR-14.2.3: Log login attempt with IP and device

**API Endpoint:** `POST /api/v1/auth/login`

**Authorization:** Public

---

### FR-14.3: Logout

**Function:** `logout()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| refreshToken | string | Yes | Current refresh token |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| success | boolean | Whether logout succeeded |

**Business Rules:**
- BR-14.3.1: Invalidate refresh token
- BR-14.3.2: Log logout event

**API Endpoint:** `POST /api/v1/auth/logout`

**Authorization:** Authenticated

---

### FR-14.4: Reset Password

**Function:** `requestPasswordReset()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| email | string | Yes | User email address |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| message | string | Generic success message |

**Business Rules:**
- BR-14.4.1: Always return success (don't reveal if email exists)
- BR-14.4.2: Send reset link if email exists
- BR-14.4.3: Reset token valid for 1 hour
- BR-14.4.4: Rate limit: 3 requests per hour per email

**API Endpoint:** `POST /api/v1/auth/password-reset`

**Authorization:** Public

---

### FR-14.5: Change Password

**Function:** `changePassword()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| userId | UUID | Yes | User ID |
| currentPassword | string | Yes | Current password |
| newPassword | string | Yes | New password |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| success | boolean | Whether change succeeded |
| passwordChangedAt | datetime | Change timestamp |

**Business Rules:**
- BR-14.5.1: Verify current password
- BR-14.5.2: New password cannot match last 3 passwords
- BR-14.5.3: Invalidate all other sessions

**API Endpoint:** `POST /api/v1/auth/change-password`

**Authorization:** Authenticated

---

### FR-14.6: Update Profile Information

**Function:** `updateProfile()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| userId | UUID | Yes | User ID |
| displayName | string | No | Updated display name |
| phone | string | No | Updated phone number |
| email | string | No | Updated email (requires verification) |
| language | string | No | Preferred language |
| timezone | string | No | Preferred timezone |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| userId | UUID | User ID |
| displayName | string | Updated name |
| email | string | Email |
| emailVerified | boolean | Email verification status |
| updatedAt | datetime | Update timestamp |

**Business Rules:**
- BR-14.6.1: Email change requires re-verification
- BR-14.6.2: Phone format validation

**API Endpoint:** `PUT /api/v1/users/profile`

**Authorization:** Authenticated

---

### FR-14.7: Upload Profile Photo

**Function:** `uploadProfilePhoto()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| userId | UUID | Yes | User ID |
| photo | file | Yes | Photo file |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| avatarUrl | string | New avatar URL |
| thumbnailUrl | string | Thumbnail URL |

**Business Rules:**
- BR-14.7.1: Max file size 5MB
- BR-14.7.2: Accepted formats: jpg, png
- BR-14.7.3: Auto-resize to standard dimensions
- BR-14.7.4: Delete previous photo

**API Endpoint:** `POST /api/v1/users/profile/photo`

**Authorization:** Authenticated

---

### FR-14.8: Delete Account

**Function:** `deleteAccount()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| userId | UUID | Yes | User ID |
| password | string | Yes | Confirm with password |
| reason | string | No | Deletion reason |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| scheduledDeletionDate | datetime | When deletion will occur |
| confirmationToken | string | Token to cancel deletion |

**Business Rules:**
- BR-14.8.1: 30-day grace period before deletion
- BR-14.8.2: Cannot delete if only unit owner
- BR-14.8.3: Cannot delete with pending payments
- BR-14.8.4: Send confirmation email

**API Endpoint:** `POST /api/v1/users/delete`

**Authorization:** Authenticated

---

### FR-14.9: View Own Activity History

**Function:** `getActivityHistory()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| userId | UUID | Yes | User ID |
| page | integer | No | Page number |
| limit | integer | No | Items per page |
| activityType | string | No | Filter by type |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| items | array | Activity log entries |
| items[].id | UUID | Activity ID |
| items[].type | string | Activity type |
| items[].description | string | Activity description |
| items[].ip | string | IP address |
| items[].device | string | Device info |
| items[].createdAt | datetime | Timestamp |
| pagination | object | Pagination metadata |

**Business Rules:**
- BR-14.9.1: Only return user's own activity
- BR-14.9.2: Retain activity for 90 days

**API Endpoint:** `GET /api/v1/users/activity`

**Authorization:** Authenticated

---

### FR-14.10: Setup Multi-Factor Authentication

**Function:** `setupMfa()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| userId | UUID | Yes | User ID |
| method | enum | Yes | `totp` \| `sms` |
| phoneNumber | string | No | Phone for SMS method |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| secret | string | TOTP secret (if method=totp) |
| qrCodeUrl | string | QR code for authenticator app |
| backupCodes | array | One-time backup codes |

**Business Rules:**
- BR-14.10.1: Generate 10 backup codes
- BR-14.10.2: Require password confirmation
- BR-14.10.3: TOTP secret only shown once

**API Endpoint:** `POST /api/v1/auth/mfa/setup`

**Authorization:** Authenticated

---

### FR-14.11: Login with SSO

**Function:** `loginSso()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| provider | enum | Yes | `saml` \| `oidc` |
| idpToken | string | Yes | Identity provider token |
| organizationId | UUID | No | Target organization |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| accessToken | string | JWT access token |
| refreshToken | string | JWT refresh token |
| user | object | User profile |
| tenants | array | Available organizations |

**Business Rules:**
- BR-14.11.1: Validate IdP token
- BR-14.11.2: Auto-provision user if configured
- BR-14.11.3: Map IdP claims to user profile

**API Endpoint:** `POST /api/v1/auth/sso`

**Authorization:** Public

---

### FR-14.12: Handle Account Lockout

**Function:** `handleLockout()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| email | string | Yes | Locked user email |
| unlockMethod | enum | Yes | `wait` \| `email` \| `support` |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| lockoutStatus | string | Current lockout status |
| unlockAt | datetime | When automatic unlock occurs |
| unlockEmailSent | boolean | Whether unlock email was sent |

**Business Rules:**
- BR-14.12.1: Auto-unlock after 15 minutes
- BR-14.12.2: Email unlock token valid 1 hour
- BR-14.12.3: Support unlock requires ticket

**API Endpoint:** `POST /api/v1/auth/lockout`

**Authorization:** Public

---

## FR-27: Multi-tenancy & Organizations

### FR-27.1: Create Organization

**Function:** `createOrganization()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| name | string | Yes | Organization name |
| type | enum | Yes | `housing_coop` \| `management_company` |
| legalName | string | Yes | Legal registered name |
| taxId | string | No | Tax identification number |
| registrationNumber | string | No | Business registration number |
| address | object | Yes | Organization address |
| contactEmail | string | Yes | Primary contact email |
| contactPhone | string | No | Primary contact phone |
| subscriptionTier | enum | Yes | `free` \| `basic` \| `professional` \| `enterprise` |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| id | UUID | Created organization ID |
| tenantId | UUID | Tenant identifier |
| name | string | Organization name |
| status | string | "active" |
| createdAt | datetime | Creation timestamp |

**Business Rules:**
- BR-27.1.1: Only Super Administrator can create
- BR-27.1.2: Organization name must be unique per region
- BR-27.1.3: Tax ID format validated by country
- BR-27.1.4: Initial status is "active"

**API Endpoint:** `POST /api/v1/organizations`

**Authorization:** Super Administrator

---

### FR-27.2: Edit Organization

**Function:** `updateOrganization()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| organizationId | UUID | Yes | Organization ID |
| name | string | No | Updated name |
| legalName | string | No | Updated legal name |
| taxId | string | No | Updated tax ID |
| address | object | No | Updated address |
| contactEmail | string | No | Updated contact email |
| contactPhone | string | No | Updated phone |
| settings | object | No | Organization settings |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| id | UUID | Organization ID |
| updatedAt | datetime | Update timestamp |

**Business Rules:**
- BR-27.2.1: Super Admin or Organization Admin can edit
- BR-27.2.2: Cannot change type after creation
- BR-27.2.3: Log all changes for audit

**API Endpoint:** `PUT /api/v1/organizations/{organizationId}`

**Authorization:** Super Administrator, Organization Admin

---

### FR-27.3: Delete Organization

**Function:** `deleteOrganization()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| organizationId | UUID | Yes | Organization ID |
| confirmationCode | string | Yes | Deletion confirmation code |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| scheduledDeletionDate | datetime | When deletion will occur |
| dataExportUrl | string | URL to download data export |

**Business Rules:**
- BR-27.3.1: Only Super Administrator can delete
- BR-27.3.2: Cannot delete with active buildings
- BR-27.3.3: Cannot delete with unpaid invoices
- BR-27.3.4: 30-day grace period
- BR-27.3.5: Export all data before deletion

**API Endpoint:** `DELETE /api/v1/organizations/{organizationId}`

**Authorization:** Super Administrator

---

### FR-27.4: View Organization List

**Function:** `listOrganizations()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| page | integer | No | Page number |
| limit | integer | No | Items per page |
| status | enum | No | `active` \| `suspended` \| `pending` |
| type | enum | No | Organization type filter |
| search | string | No | Search by name |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| items | array | Organizations |
| items[].id | UUID | Organization ID |
| items[].name | string | Name |
| items[].type | string | Type |
| items[].status | string | Status |
| items[].buildingCount | integer | Number of buildings |
| items[].userCount | integer | Number of users |
| items[].createdAt | datetime | Creation date |
| pagination | object | Pagination metadata |

**Business Rules:**
- BR-27.4.1: Only Super Administrator can list all
- BR-27.4.2: Organization Admin sees only their org

**API Endpoint:** `GET /api/v1/organizations`

**Authorization:** Super Administrator

---

### FR-27.5: Assign Building to Organization

**Function:** `assignBuildingToOrganization()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| organizationId | UUID | Yes | Organization ID |
| buildingId | UUID | Yes | Building ID |
| effectiveDate | date | No | When assignment takes effect |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| organizationId | UUID | Organization ID |
| buildingId | UUID | Building ID |
| assignedAt | datetime | Assignment timestamp |

**Business Rules:**
- BR-27.5.1: Building can only belong to one organization
- BR-27.5.2: Transfer requires unassigning first
- BR-27.5.3: Notify building residents

**API Endpoint:** `POST /api/v1/organizations/{organizationId}/buildings`

**Authorization:** Super Administrator, Organization Admin

---

### FR-27.6: Remove Building from Organization

**Function:** `removeBuildingFromOrganization()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| organizationId | UUID | Yes | Organization ID |
| buildingId | UUID | Yes | Building ID |
| reason | string | No | Removal reason |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| success | boolean | Whether removal succeeded |
| removedAt | datetime | Removal timestamp |

**Business Rules:**
- BR-27.6.1: Retain building data for organization
- BR-27.6.2: Notify building managers

**API Endpoint:** `DELETE /api/v1/organizations/{organizationId}/buildings/{buildingId}`

**Authorization:** Super Administrator, Organization Admin

---

### FR-27.7: Switch Organization Context

**Function:** `switchOrganization()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| userId | UUID | Yes | User ID |
| targetOrganizationId | UUID | Yes | Target organization ID |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| accessToken | string | New access token with tenant context |
| tenantId | UUID | New tenant ID |
| organization | object | Organization details |
| role | string | User's role in this organization |

**Business Rules:**
- BR-27.7.1: User must have access to target organization
- BR-27.7.2: Issue new token with tenant context
- BR-27.7.3: Log context switch

**API Endpoint:** `POST /api/v1/auth/switch-organization`

**Authorization:** Manager, Organization Admin

---

### FR-27.8: View Organization Statistics

**Function:** `getOrganizationStatistics()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| organizationId | UUID | Yes | Organization ID |
| period | enum | No | `week` \| `month` \| `quarter` \| `year` |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| buildingCount | integer | Total buildings |
| unitCount | integer | Total units |
| userCount | integer | Registered users |
| activeUsers | integer | Users active in period |
| faultStats | object | Fault statistics |
| votingStats | object | Voting statistics |
| financialStats | object | Financial summary |

**Business Rules:**
- BR-27.8.1: Calculate metrics for organization scope only
- BR-27.8.2: Cache statistics for performance

**API Endpoint:** `GET /api/v1/organizations/{organizationId}/statistics`

**Authorization:** Organization Admin, Manager

---

### FR-27.9: Configure Organization Settings

**Function:** `updateOrganizationSettings()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| organizationId | UUID | Yes | Organization ID |
| settings | object | Yes | Settings to update |
| settings.defaultLanguage | string | No | Default language |
| settings.timezone | string | No | Organization timezone |
| settings.dateFormat | string | No | Date format preference |
| settings.currencyCode | string | No | Currency code |
| settings.fiscalYearStart | date | No | Fiscal year start month/day |
| settings.features | object | No | Feature toggles |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| organizationId | UUID | Organization ID |
| settings | object | Updated settings |
| updatedAt | datetime | Update timestamp |

**Business Rules:**
- BR-27.9.1: Validate timezone against IANA database
- BR-27.9.2: Features limited by subscription tier

**API Endpoint:** `PUT /api/v1/organizations/{organizationId}/settings`

**Authorization:** Organization Admin

---

### FR-27.10: Manage Organization Branding

**Function:** `updateOrganizationBranding()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| organizationId | UUID | Yes | Organization ID |
| logo | file | No | Organization logo |
| primaryColor | string | No | Primary brand color (hex) |
| secondaryColor | string | No | Secondary brand color (hex) |
| emailHeaderLogo | file | No | Logo for email headers |
| favicon | file | No | Browser favicon |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| organizationId | UUID | Organization ID |
| logoUrl | string | Logo URL |
| primaryColor | string | Primary color |
| secondaryColor | string | Secondary color |

**Business Rules:**
- BR-27.10.1: Available on Professional tier and above
- BR-27.10.2: Logo max 2MB, PNG/SVG only
- BR-27.10.3: Colors must have sufficient contrast

**API Endpoint:** `PUT /api/v1/organizations/{organizationId}/branding`

**Authorization:** Organization Admin

---

## FR-06: Neighbors

### FR-06.1: View Neighbors List

**Function:** `listNeighbors()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| page | integer | No | Page number |
| limit | integer | No | Items per page |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| items | array | List of neighbors |
| items[].id | UUID | User ID |
| items[].displayName | string | Display name |
| items[].unitNumber | string | Unit number |
| items[].floor | integer | Floor number |
| items[].entrance | string | Entrance/section |
| items[].role | string | Owner/Tenant |
| items[].avatarUrl | string | Profile photo URL |
| items[].registeredAt | datetime | Registration date |
| pagination | object | Pagination metadata |

**Business Rules:**
- BR-06.1.1: Only show users in same building
- BR-06.1.2: Respect privacy settings
- BR-06.1.3: Sort by unit number

**API Endpoint:** `GET /api/v1/buildings/{buildingId}/neighbors`

**Authorization:** Owner, Tenant, Manager

---

### FR-06.2: Invite Neighbor

**Function:** `inviteNeighbor()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| unitId | UUID | Yes | Target unit ID |
| email | string | Yes | Invitation email |
| role | enum | Yes | `owner` \| `tenant` |
| displayName | string | No | Suggested display name |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| invitationId | UUID | Invitation ID |
| invitationCode | string | Registration code |
| expiresAt | datetime | Expiration date |

**Business Rules:**
- BR-06.2.1: Invitation valid for 30 days
- BR-06.2.2: One active invitation per unit/role
- BR-06.2.3: Send email with registration link

**API Endpoint:** `POST /api/v1/buildings/{buildingId}/neighbors/invite`

**Authorization:** Manager

---

### FR-06.3: Search Neighbors

**Function:** `searchNeighbors()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| query | string | Yes | Search query (min 2 chars) |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| items | array | Matching neighbors |
| pagination | object | Pagination metadata |

**Business Rules:**
- BR-06.3.1: Search by name and unit number
- BR-06.3.2: Respect privacy settings

**API Endpoint:** `GET /api/v1/buildings/{buildingId}/neighbors/search`

**Authorization:** Owner, Tenant, Manager

---

### FR-06.4: Filter Neighbors by Entrance

**Function:** `listNeighborsByEntrance()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| entrance | string | Yes | Entrance/section identifier |
| page | integer | No | Page number |
| limit | integer | No | Items per page |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| items | array | Neighbors in entrance |
| pagination | object | Pagination metadata |

**Business Rules:**
- BR-06.4.1: Only return neighbors in specified entrance
- BR-06.4.2: Respect privacy settings

**API Endpoint:** `GET /api/v1/buildings/{buildingId}/neighbors?entrance={entrance}`

**Authorization:** Owner, Tenant, Manager

---

### FR-06.5: Contact Neighbor

**Function:** `contactNeighbor()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| neighborId | UUID | Yes | Neighbor user ID |
| message | string | Yes | Initial message |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| conversationId | UUID | Created conversation ID |
| messageId | UUID | Message ID |

**Business Rules:**
- BR-06.5.1: Create new conversation or use existing
- BR-06.5.2: Must be in same building
- BR-06.5.3: Neighbor must allow contact

**API Endpoint:** `POST /api/v1/neighbors/{neighborId}/contact`

**Authorization:** Owner, Tenant

---

### FR-06.6: Edit Neighbor Information

**Function:** `updateNeighbor()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| neighborId | UUID | Yes | Neighbor user ID |
| unitId | UUID | No | Updated unit assignment |
| role | enum | No | Updated role |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| neighborId | UUID | User ID |
| updatedAt | datetime | Update timestamp |

**Business Rules:**
- BR-06.6.1: Only managers can edit
- BR-06.6.2: Cannot change email/password
- BR-06.6.3: Log changes for audit

**API Endpoint:** `PUT /api/v1/neighbors/{neighborId}`

**Authorization:** Manager

---

### FR-06.7: Remove Neighbor from Building

**Function:** `removeNeighbor()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| neighborId | UUID | Yes | Neighbor user ID |
| reason | string | No | Removal reason |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| success | boolean | Whether removal succeeded |

**Business Rules:**
- BR-06.7.1: Soft delete - user can be reassigned
- BR-06.7.2: Notify user of removal
- BR-06.7.3: Preserve user account

**API Endpoint:** `DELETE /api/v1/neighbors/{neighborId}`

**Authorization:** Manager

---

### FR-06.8: Resend Invitation

**Function:** `resendInvitation()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| invitationId | UUID | Yes | Invitation ID |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| invitationId | UUID | Invitation ID |
| resentAt | datetime | Resend timestamp |
| expiresAt | datetime | New expiration |

**Business Rules:**
- BR-06.8.1: Extend expiration by 30 days
- BR-06.8.2: Rate limit: 3 resends per invitation
- BR-06.8.3: Send new email

**API Endpoint:** `POST /api/v1/invitations/{invitationId}/resend`

**Authorization:** Manager

---

### FR-06.9: Cancel Pending Invitation

**Function:** `cancelInvitation()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| invitationId | UUID | Yes | Invitation ID |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| success | boolean | Whether cancellation succeeded |

**Business Rules:**
- BR-06.9.1: Only pending invitations can be cancelled
- BR-06.9.2: Invalidate invitation code

**API Endpoint:** `DELETE /api/v1/invitations/{invitationId}`

**Authorization:** Manager

---

### FR-06.10: View Invitation Status

**Function:** `listInvitations()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| status | enum | No | `pending` \| `accepted` \| `expired` |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| items | array | Invitations |
| items[].id | UUID | Invitation ID |
| items[].email | string | Invited email |
| items[].unitNumber | string | Target unit |
| items[].role | string | Invited role |
| items[].status | string | Current status |
| items[].createdAt | datetime | Creation date |
| items[].expiresAt | datetime | Expiration date |

**Business Rules:**
- BR-06.10.1: Show all statuses if not filtered
- BR-06.10.2: Sort by createdAt descending

**API Endpoint:** `GET /api/v1/buildings/{buildingId}/invitations`

**Authorization:** Manager

---

## FR-07: Contacts

### FR-07.1: View Manager Directory

**Function:** `listManagerContacts()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| items | array | Manager contacts |
| items[].id | UUID | Contact ID |
| items[].displayName | string | Name |
| items[].role | string | Manager/Technical Manager |
| items[].email | string | Email address |
| items[].phone | string | Phone number |
| items[].avatarUrl | string | Profile photo |
| items[].isPrimary | boolean | Primary contact flag |
| items[].availability | string | Availability hours |

**Business Rules:**
- BR-07.1.1: Show only building-assigned managers
- BR-07.1.2: Primary contact shown first

**API Endpoint:** `GET /api/v1/buildings/{buildingId}/contacts`

**Authorization:** Owner, Tenant

---

### FR-07.2: View Manager Profile

**Function:** `getManagerProfile()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| contactId | UUID | Yes | Contact ID |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| id | UUID | Contact ID |
| displayName | string | Full name |
| role | string | Role |
| email | string | Email |
| phone | string | Phone |
| bio | string | Biography |
| avatarUrl | string | Photo URL |
| buildings | array | Managed buildings |
| availability | object | Availability schedule |

**Business Rules:**
- BR-07.2.1: User must have access to at least one managed building

**API Endpoint:** `GET /api/v1/contacts/{contactId}`

**Authorization:** Owner, Tenant

---

### FR-07.3: Contact Manager

**Function:** `contactManager()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| contactId | UUID | Yes | Contact ID |
| method | enum | Yes | `message` \| `email` |
| subject | string | No | Message subject |
| message | string | Yes | Message content |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| conversationId | UUID | Conversation ID (if message) |
| emailSent | boolean | Whether email was sent |

**Business Rules:**
- BR-07.3.1: Create conversation for message method
- BR-07.3.2: Send email for email method
- BR-07.3.3: User must be in manager's building

**API Endpoint:** `POST /api/v1/contacts/{contactId}/message`

**Authorization:** Owner, Tenant

---

### FR-07.4: Add Manager Contact

**Function:** `createManagerContact()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| userId | UUID | Yes | User to add as contact |
| buildingIds | array | Yes | Buildings to assign |
| role | enum | Yes | `manager` \| `technical_manager` |
| phone | string | No | Contact phone |
| availability | string | No | Availability description |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| contactId | UUID | Created contact ID |
| createdAt | datetime | Creation timestamp |

**Business Rules:**
- BR-07.4.1: User must exist in organization
- BR-07.4.2: User must have manager role
- BR-07.4.3: Notify user of assignment

**API Endpoint:** `POST /api/v1/contacts`

**Authorization:** Manager, System Administrator

---

### FR-07.5: Edit Manager Contact

**Function:** `updateManagerContact()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| contactId | UUID | Yes | Contact ID |
| phone | string | No | Updated phone |
| availability | string | No | Updated availability |
| buildingIds | array | No | Updated buildings |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| contactId | UUID | Contact ID |
| updatedAt | datetime | Update timestamp |

**Business Rules:**
- BR-07.5.1: Cannot change user or role
- BR-07.5.2: Log changes

**API Endpoint:** `PUT /api/v1/contacts/{contactId}`

**Authorization:** Manager, System Administrator

---

### FR-07.6: Remove Manager Contact

**Function:** `removeManagerContact()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| contactId | UUID | Yes | Contact ID |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| success | boolean | Whether removal succeeded |

**Business Rules:**
- BR-07.6.1: Cannot remove last manager from building
- BR-07.6.2: Notify affected buildings

**API Endpoint:** `DELETE /api/v1/contacts/{contactId}`

**Authorization:** Manager, System Administrator

---

### FR-07.7: Set Primary Contact for Building

**Function:** `setPrimaryContact()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| contactId | UUID | Yes | Contact to make primary |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| buildingId | UUID | Building ID |
| primaryContactId | UUID | New primary contact |

**Business Rules:**
- BR-07.7.1: Contact must be assigned to building
- BR-07.7.2: One primary contact per building

**API Endpoint:** `POST /api/v1/buildings/{buildingId}/primary-contact`

**Authorization:** Manager

---

## FR-08: Documents

### FR-08.1: Search Documents

**Function:** `searchDocuments()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| query | string | Yes | Search query |
| page | integer | No | Page number |
| limit | integer | No | Items per page |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| items | array | Matching documents |
| items[].id | UUID | Document ID |
| items[].name | string | File name |
| items[].mimeType | string | File type |
| items[].folderId | UUID | Parent folder |
| items[].folderPath | string | Full path |
| items[].size | integer | File size |
| items[].uploadedAt | datetime | Upload date |
| pagination | object | Pagination metadata |

**Business Rules:**
- BR-08.1.1: Full-text search on name and OCR content
- BR-08.1.2: Respect document permissions

**API Endpoint:** `GET /api/v1/buildings/{buildingId}/documents/search`

**Authorization:** Owner, Tenant, Manager

---

### FR-08.2: Browse Document Folders

**Function:** `listDocumentFolder()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| folderId | UUID | No | Folder ID (root if empty) |
| page | integer | No | Page number |
| limit | integer | No | Items per page |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| currentFolder | object | Current folder info |
| breadcrumbs | array | Path from root |
| folders | array | Subfolders |
| documents | array | Documents in folder |
| pagination | object | Pagination metadata |

**Business Rules:**
- BR-08.2.1: Sort folders first, then documents
- BR-08.2.2: Sort alphabetically within type
- BR-08.2.3: Respect folder permissions

**API Endpoint:** `GET /api/v1/buildings/{buildingId}/documents/folders/{folderId}`

**Authorization:** Owner, Tenant, Manager

---

### FR-08.3: Download Document

**Function:** `downloadDocument()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| documentId | UUID | Yes | Document ID |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| downloadUrl | string | Temporary signed URL |
| expiresAt | datetime | URL expiration |
| filename | string | Original filename |

**Business Rules:**
- BR-08.3.1: Check user permissions
- BR-08.3.2: URL valid for 1 hour
- BR-08.3.3: Log download for audit

**API Endpoint:** `GET /api/v1/documents/{documentId}/download`

**Authorization:** Owner, Tenant, Manager

---

### FR-08.4: Forward Document

**Function:** `forwardDocument()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| documentId | UUID | Yes | Document ID |
| recipientIds | array | Yes | Recipient user IDs |
| message | string | No | Accompanying message |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| forwardedCount | integer | Number of recipients |
| messageIds | array | Created message IDs |

**Business Rules:**
- BR-08.4.1: Recipients must have document access
- BR-08.4.2: Create message with document link
- BR-08.4.3: Notify recipients

**API Endpoint:** `POST /api/v1/documents/{documentId}/forward`

**Authorization:** Owner, Tenant, Manager

---

### FR-08.5: View Document Version History

**Function:** `getDocumentVersions()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| documentId | UUID | Yes | Document ID |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| items | array | Document versions |
| items[].version | integer | Version number |
| items[].uploadedBy | object | Uploader info |
| items[].uploadedAt | datetime | Upload timestamp |
| items[].size | integer | File size |
| items[].changeNote | string | Version note |

**Business Rules:**
- BR-08.5.1: Sort by version descending
- BR-08.5.2: Include current version

**API Endpoint:** `GET /api/v1/documents/{documentId}/versions`

**Authorization:** Owner, Tenant, Manager

---

### FR-08.6: Upload Document

**Function:** `uploadDocument()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| folderId | UUID | No | Target folder ID |
| file | file | Yes | File to upload |
| name | string | No | Custom file name |
| description | string | No | Document description |
| visibility | enum | No | `all` \| `owners` \| `managers` |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| documentId | UUID | Created document ID |
| name | string | Final file name |
| size | integer | File size |
| uploadedAt | datetime | Upload timestamp |

**Business Rules:**
- BR-08.6.1: Max file size 50MB
- BR-08.6.2: Virus scan before accepting
- BR-08.6.3: Auto-OCR for PDFs and images
- BR-08.6.4: Default visibility: all

**API Endpoint:** `POST /api/v1/buildings/{buildingId}/documents`

**Authorization:** Manager

---

### FR-08.7: Create Document Folder

**Function:** `createDocumentFolder()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| parentFolderId | UUID | No | Parent folder ID |
| name | string | Yes | Folder name |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| folderId | UUID | Created folder ID |
| name | string | Folder name |
| path | string | Full folder path |

**Business Rules:**
- BR-08.7.1: Folder name unique within parent
- BR-08.7.2: Max nesting depth: 5 levels

**API Endpoint:** `POST /api/v1/buildings/{buildingId}/documents/folders`

**Authorization:** Manager

---

### FR-08.8: Delete Document

**Function:** `deleteDocument()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| documentId | UUID | Yes | Document ID |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| success | boolean | Whether deletion succeeded |

**Business Rules:**
- BR-08.8.1: Soft delete with 30-day retention
- BR-08.8.2: Log deletion for audit

**API Endpoint:** `DELETE /api/v1/documents/{documentId}`

**Authorization:** Manager

---

### FR-08.9: Move Document to Folder

**Function:** `moveDocument()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| documentId | UUID | Yes | Document ID |
| targetFolderId | UUID | Yes | Destination folder ID |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| documentId | UUID | Document ID |
| newPath | string | New folder path |

**Business Rules:**
- BR-08.9.1: Target folder must exist
- BR-08.9.2: Same building only

**API Endpoint:** `POST /api/v1/documents/{documentId}/move`

**Authorization:** Manager

---

### FR-08.10: Rename Document

**Function:** `renameDocument()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| documentId | UUID | Yes | Document ID |
| newName | string | Yes | New file name |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| documentId | UUID | Document ID |
| name | string | Updated name |

**Business Rules:**
- BR-08.10.1: Preserve file extension
- BR-08.10.2: Name unique in folder

**API Endpoint:** `PUT /api/v1/documents/{documentId}/rename`

**Authorization:** Manager

---

### FR-08.11: Rename Folder

**Function:** `renameFolder()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| folderId | UUID | Yes | Folder ID |
| newName | string | Yes | New folder name |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| folderId | UUID | Folder ID |
| name | string | Updated name |
| path | string | Updated path |

**Business Rules:**
- BR-08.11.1: Name unique within parent
- BR-08.11.2: Update child paths

**API Endpoint:** `PUT /api/v1/documents/folders/{folderId}/rename`

**Authorization:** Manager

---

### FR-08.12: Delete Folder

**Function:** `deleteFolder()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| folderId | UUID | Yes | Folder ID |
| force | boolean | No | Delete with contents |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| success | boolean | Whether deletion succeeded |
| deletedDocuments | integer | Number of deleted documents |

**Business Rules:**
- BR-08.12.1: Empty folder or force required
- BR-08.12.2: Cascade delete if force

**API Endpoint:** `DELETE /api/v1/documents/folders/{folderId}`

**Authorization:** Manager

---

### FR-08.13: Set Document Access Permissions

**Function:** `setDocumentPermissions()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| documentId | UUID | Yes | Document ID |
| visibility | enum | Yes | `all` \| `owners` \| `tenants` \| `managers` \| `custom` |
| allowedUserIds | array | No | Specific user IDs (if custom) |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| documentId | UUID | Document ID |
| visibility | string | Updated visibility |

**Business Rules:**
- BR-08.13.1: Custom requires user list
- BR-08.13.2: Managers always have access

**API Endpoint:** `PUT /api/v1/documents/{documentId}/permissions`

**Authorization:** Manager

---

### FR-08.14: Share Document with Specific Owners

**Function:** `shareDocument()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| documentId | UUID | Yes | Document ID |
| userIds | array | Yes | User IDs to share with |
| notifyUsers | boolean | No | Send notification |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| documentId | UUID | Document ID |
| sharedWith | array | User IDs with access |

**Business Rules:**
- BR-08.14.1: Users must be in building
- BR-08.14.2: Adds to existing permissions

**API Endpoint:** `POST /api/v1/documents/{documentId}/share`

**Authorization:** Manager

---

## FR-09: Forms

### FR-09.1: Search Forms

**Function:** `searchForms()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| query | string | Yes | Search query |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| items | array | Matching forms |
| items[].id | UUID | Form ID |
| items[].name | string | Form name |
| items[].description | string | Description |
| items[].isOnline | boolean | Can submit online |
| pagination | object | Pagination metadata |

**Business Rules:**
- BR-09.1.1: Only show published forms
- BR-09.1.2: Search name and description

**API Endpoint:** `GET /api/v1/buildings/{buildingId}/forms/search`

**Authorization:** Owner, Tenant

---

### FR-09.2: Download Form

**Function:** `downloadForm()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| formId | UUID | Yes | Form ID |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| downloadUrl | string | Temporary download URL |
| filename | string | Form filename |
| mimeType | string | File type |

**Business Rules:**
- BR-09.2.1: URL valid for 1 hour
- BR-09.2.2: Track download count

**API Endpoint:** `GET /api/v1/forms/{formId}/download`

**Authorization:** Owner, Tenant

---

### FR-09.3: Publish Form

**Function:** `publishForm()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| name | string | Yes | Form name |
| description | string | No | Form description |
| file | file | No | PDF form file |
| fields | array | No | Online form fields |
| isOnline | boolean | Yes | Allow online submission |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| formId | UUID | Created form ID |
| publishedAt | datetime | Publication timestamp |

**Business Rules:**
- BR-09.3.1: Either file or fields required
- BR-09.3.2: Max 50 fields for online forms

**API Endpoint:** `POST /api/v1/buildings/{buildingId}/forms`

**Authorization:** Manager

---

### FR-09.4: Submit Filled Form Online

**Function:** `submitForm()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| formId | UUID | Yes | Form ID |
| responses | object | Yes | Field responses |
| attachments | array | No | File attachments |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| submissionId | UUID | Submission ID |
| submittedAt | datetime | Submission timestamp |
| confirmationNumber | string | Reference number |

**Business Rules:**
- BR-09.4.1: Validate required fields
- BR-09.4.2: Form must allow online submission
- BR-09.4.3: Send confirmation email

**API Endpoint:** `POST /api/v1/forms/{formId}/submit`

**Authorization:** Owner, Tenant

---

### FR-09.5: Edit Form Template

**Function:** `updateForm()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| formId | UUID | Yes | Form ID |
| name | string | No | Updated name |
| description | string | No | Updated description |
| fields | array | No | Updated fields |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| formId | UUID | Form ID |
| updatedAt | datetime | Update timestamp |

**Business Rules:**
- BR-09.5.1: Cannot change existing field types
- BR-09.5.2: New fields added as optional

**API Endpoint:** `PUT /api/v1/forms/{formId}`

**Authorization:** Manager

---

### FR-09.6: Delete Form

**Function:** `deleteForm()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| formId | UUID | Yes | Form ID |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| success | boolean | Whether deletion succeeded |

**Business Rules:**
- BR-09.6.1: Preserve existing submissions
- BR-09.6.2: Soft delete

**API Endpoint:** `DELETE /api/v1/forms/{formId}`

**Authorization:** Manager

---

### FR-09.7: View Form Submissions

**Function:** `listFormSubmissions()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| formId | UUID | Yes | Form ID |
| page | integer | No | Page number |
| limit | integer | No | Items per page |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| items | array | Submissions |
| items[].id | UUID | Submission ID |
| items[].submittedBy | object | Submitter info |
| items[].submittedAt | datetime | Submission date |
| items[].responses | object | Field responses |
| pagination | object | Pagination metadata |

**Business Rules:**
- BR-09.7.1: Sort by submittedAt descending
- BR-09.7.2: Include submitter details

**API Endpoint:** `GET /api/v1/forms/{formId}/submissions`

**Authorization:** Manager

---

### FR-09.8: Export Form Submissions

**Function:** `exportFormSubmissions()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| formId | UUID | Yes | Form ID |
| format | enum | Yes | `xlsx` \| `csv` |
| dateFrom | date | No | Filter start date |
| dateTo | date | No | Filter end date |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| downloadUrl | string | Export file URL |
| filename | string | File name |
| recordCount | integer | Number of records |

**Business Rules:**
- BR-09.8.1: Include all field responses
- BR-09.8.2: URL valid for 1 hour

**API Endpoint:** `GET /api/v1/forms/{formId}/submissions/export`

**Authorization:** Manager

---

## FR-10: Person-Months

### FR-10.1: Add Person-Month Record

**Function:** `createPersonMonthRecord()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| unitId | UUID | Yes | Unit ID |
| year | integer | Yes | Year |
| month | integer | Yes | Month (1-12) |
| personCount | integer | Yes | Number of persons |
| notes | string | No | Additional notes |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| recordId | UUID | Record ID |
| createdAt | datetime | Creation timestamp |

**Business Rules:**
- BR-10.1.1: One record per unit per month
- BR-10.1.2: Person count >= 0
- BR-10.1.3: Cannot add future months

**API Endpoint:** `POST /api/v1/units/{unitId}/person-months`

**Authorization:** Owner, Manager

---

### FR-10.2: View Person-Month History

**Function:** `listPersonMonthHistory()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| unitId | UUID | Yes | Unit ID |
| yearFrom | integer | No | Start year |
| yearTo | integer | No | End year |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| items | array | Person-month records |
| items[].id | UUID | Record ID |
| items[].year | integer | Year |
| items[].month | integer | Month |
| items[].personCount | integer | Person count |
| items[].submittedBy | object | Submitter info |
| items[].submittedAt | datetime | Submission date |

**Business Rules:**
- BR-10.2.1: Sort by year/month descending
- BR-10.2.2: Default last 2 years

**API Endpoint:** `GET /api/v1/units/{unitId}/person-months`

**Authorization:** Owner, Manager

---

### FR-10.3: Edit Person-Month Record

**Function:** `updatePersonMonthRecord()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| recordId | UUID | Yes | Record ID |
| personCount | integer | Yes | Updated person count |
| notes | string | No | Updated notes |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| recordId | UUID | Record ID |
| updatedAt | datetime | Update timestamp |

**Business Rules:**
- BR-10.3.1: Owner can edit own records
- BR-10.3.2: Manager can edit any record
- BR-10.3.3: Log changes

**API Endpoint:** `PUT /api/v1/person-months/{recordId}`

**Authorization:** Owner, Manager

---

### FR-10.4: Delete Person-Month Record

**Function:** `deletePersonMonthRecord()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| recordId | UUID | Yes | Record ID |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| success | boolean | Whether deletion succeeded |

**Business Rules:**
- BR-10.4.1: Only managers can delete
- BR-10.4.2: Log deletion

**API Endpoint:** `DELETE /api/v1/person-months/{recordId}`

**Authorization:** Manager

---

### FR-10.5: Bulk Entry Person-Months

**Function:** `bulkCreatePersonMonths()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| year | integer | Yes | Year |
| month | integer | Yes | Month |
| entries | array | Yes | Unit person counts |
| entries[].unitId | UUID | Yes | Unit ID |
| entries[].personCount | integer | Yes | Person count |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| createdCount | integer | Records created |
| updatedCount | integer | Records updated |
| errors | array | Any errors |

**Business Rules:**
- BR-10.5.1: Upsert existing records
- BR-10.5.2: Validate all units in building

**API Endpoint:** `POST /api/v1/buildings/{buildingId}/person-months/bulk`

**Authorization:** Manager

---

### FR-10.6: Export Person-Month Data

**Function:** `exportPersonMonths()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| yearFrom | integer | Yes | Start year |
| yearTo | integer | Yes | End year |
| format | enum | Yes | `xlsx` \| `csv` |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| downloadUrl | string | Export file URL |
| filename | string | File name |

**Business Rules:**
- BR-10.6.1: Include all units
- BR-10.6.2: Pivot by month

**API Endpoint:** `GET /api/v1/buildings/{buildingId}/person-months/export`

**Authorization:** Manager

---

### FR-10.7: Set Reminder for Person-Month Entry

**Function:** `setPersonMonthReminder()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| dayOfMonth | integer | Yes | Day to send reminder (1-28) |
| enabled | boolean | Yes | Enable/disable reminder |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| buildingId | UUID | Building ID |
| reminderConfig | object | Reminder configuration |

**Business Rules:**
- BR-10.7.1: Send to all owners without submission
- BR-10.7.2: Day 1-28 only

**API Endpoint:** `PUT /api/v1/buildings/{buildingId}/person-months/reminder`

**Authorization:** Manager

---

## FR-11: Self-Readings

### FR-11.1: Submit Meter Reading

**Function:** `submitMeterReading()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| unitId | UUID | Yes | Unit ID |
| meterId | UUID | Yes | Meter ID |
| readingValue | number | Yes | Meter reading value |
| readingDate | date | Yes | Date of reading |
| photo | file | No | Photo of meter |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| readingId | UUID | Created reading ID |
| status | string | "pending_verification" |
| submittedAt | datetime | Submission timestamp |

**Business Rules:**
- BR-11.1.1: Reading must be >= previous reading
- BR-11.1.2: Reading date cannot be future
- BR-11.1.3: One reading per meter per month
- BR-11.1.4: AI validates photo if provided

**API Endpoint:** `POST /api/v1/units/{unitId}/meter-readings`

**Authorization:** Owner

---

### FR-11.2: View Self-Readings Overview

**Function:** `listMeterReadings()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| year | integer | No | Filter by year |
| month | integer | No | Filter by month |
| status | enum | No | `pending` \| `verified` \| `rejected` |
| page | integer | No | Page number |
| limit | integer | No | Items per page |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| items | array | Meter readings |
| items[].id | UUID | Reading ID |
| items[].unitNumber | string | Unit number |
| items[].meterType | string | water/electricity/gas |
| items[].readingValue | number | Submitted value |
| items[].status | string | Verification status |
| items[].submittedAt | datetime | Submission date |
| summary | object | Aggregated stats |
| pagination | object | Pagination metadata |

**Business Rules:**
- BR-11.2.1: Show all units in building
- BR-11.2.2: Include missing readings

**API Endpoint:** `GET /api/v1/buildings/{buildingId}/meter-readings`

**Authorization:** Manager

---

### FR-11.3: Export Self-Readings

**Function:** `exportMeterReadings()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| year | integer | Yes | Year to export |
| format | enum | Yes | `xlsx` \| `csv` |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| downloadUrl | string | Export file URL |
| filename | string | File name |

**Business Rules:**
- BR-11.3.1: Include all meter types
- BR-11.3.2: Pivot by month

**API Endpoint:** `GET /api/v1/buildings/{buildingId}/meter-readings/export`

**Authorization:** Manager

---

### FR-11.4: Verify Meter Reading

**Function:** `verifyMeterReading()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| readingId | UUID | Yes | Reading ID |
| verified | boolean | Yes | Approve or not |
| note | string | No | Verification note |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| readingId | UUID | Reading ID |
| status | string | "verified" or unchanged |
| verifiedAt | datetime | Verification timestamp |

**Business Rules:**
- BR-11.4.1: Only pending readings can be verified
- BR-11.4.2: Log verification action

**API Endpoint:** `POST /api/v1/meter-readings/{readingId}/verify`

**Authorization:** Manager

---

### FR-11.5: Edit Meter Reading

**Function:** `updateMeterReading()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| readingId | UUID | Yes | Reading ID |
| readingValue | number | Yes | Updated value |
| reason | string | Yes | Reason for edit |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| readingId | UUID | Reading ID |
| updatedAt | datetime | Update timestamp |

**Business Rules:**
- BR-11.5.1: Owner can edit unverified only
- BR-11.5.2: Manager can edit any
- BR-11.5.3: Log all changes

**API Endpoint:** `PUT /api/v1/meter-readings/{readingId}`

**Authorization:** Owner, Manager

---

### FR-11.6: Reject Meter Reading

**Function:** `rejectMeterReading()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| readingId | UUID | Yes | Reading ID |
| reason | string | Yes | Rejection reason |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| readingId | UUID | Reading ID |
| status | string | "rejected" |
| rejectedAt | datetime | Rejection timestamp |

**Business Rules:**
- BR-11.6.1: Notify owner of rejection
- BR-11.6.2: Request resubmission

**API Endpoint:** `POST /api/v1/meter-readings/{readingId}/reject`

**Authorization:** Manager

---

### FR-11.7: Request Reading Correction

**Function:** `requestReadingCorrection()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| readingId | UUID | Yes | Reading ID |
| message | string | Yes | Correction request message |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| requestId | UUID | Request ID |
| sentAt | datetime | Request timestamp |

**Business Rules:**
- BR-11.7.1: Notify owner
- BR-11.7.2: Set status to pending_correction

**API Endpoint:** `POST /api/v1/meter-readings/{readingId}/request-correction`

**Authorization:** Manager

---

### FR-11.8: Send Reading Submission Reminder

**Function:** `sendReadingReminder()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| year | integer | Yes | Target year |
| month | integer | Yes | Target month |
| message | string | No | Custom message |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| sentCount | integer | Reminders sent |
| targetedUnits | array | Units reminded |

**Business Rules:**
- BR-11.8.1: Only remind units without submission
- BR-11.8.2: Rate limit: 1 per week

**API Endpoint:** `POST /api/v1/buildings/{buildingId}/meter-readings/remind`

**Authorization:** Manager

---

### FR-11.9: View Reading History

**Function:** `getMeterReadingHistory()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| unitId | UUID | Yes | Unit ID |
| meterId | UUID | No | Specific meter |
| yearFrom | integer | No | Start year |
| yearTo | integer | No | End year |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| items | array | Historical readings |
| items[].year | integer | Year |
| items[].month | integer | Month |
| items[].value | number | Reading value |
| items[].consumption | number | Calculated consumption |
| items[].status | string | Verification status |

**Business Rules:**
- BR-11.9.1: Calculate consumption from delta
- BR-11.9.2: Default last 2 years

**API Endpoint:** `GET /api/v1/units/{unitId}/meter-readings/history`

**Authorization:** Owner, Manager

---

### FR-11.10: Compare Readings Over Time

**Function:** `compareReadings()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| meterType | enum | Yes | `water` \| `electricity` \| `gas` |
| yearFrom | integer | Yes | Start year |
| yearTo | integer | Yes | End year |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| data | array | Comparison data |
| anomalies | array | Detected anomalies |
| averages | object | Building averages |
| trends | object | Consumption trends |

**Business Rules:**
- BR-11.10.1: Flag readings >2 std dev from mean
- BR-11.10.2: Calculate YoY trends

**API Endpoint:** `GET /api/v1/buildings/{buildingId}/meter-readings/compare`

**Authorization:** Manager

---

## FR-12: Outages

### FR-12.1: View Outages List

**Function:** `listOutages()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| status | enum | No | `active` \| `planned` \| `resolved` |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| items | array | Outages |
| items[].id | UUID | Outage ID |
| items[].type | string | water/electricity/gas/heating |
| items[].status | string | Current status |
| items[].startTime | datetime | Outage start |
| items[].endTime | datetime | Expected end |
| items[].description | string | Details |
| items[].supplier | object | Supplier info |

**Business Rules:**
- BR-12.1.1: Sort by startTime descending
- BR-12.1.2: Active outages first

**API Endpoint:** `GET /api/v1/buildings/{buildingId}/outages`

**Authorization:** Owner, Tenant

---

### FR-12.2: View Outages by Commodity

**Function:** `listOutagesByType()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| commodityType | enum | Yes | `water` \| `electricity` \| `gas` \| `heating` |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| items | array | Filtered outages |
| supplier | object | Commodity supplier info |

**Business Rules:**
- BR-12.2.1: Include supplier contact
- BR-12.2.2: Include outage hotline

**API Endpoint:** `GET /api/v1/buildings/{buildingId}/outages?type={type}`

**Authorization:** Owner, Tenant

---

### FR-12.3: Call Supplier

**Function:** `getSupplierPhone()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| supplierId | UUID | Yes | Supplier ID |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| phone | string | Supplier phone number |
| hotline | string | Emergency hotline |
| callUri | string | tel: URI for mobile |

**Business Rules:**
- BR-12.3.1: Return emergency hotline if available

**API Endpoint:** `GET /api/v1/suppliers/{supplierId}/phone`

**Authorization:** Owner, Tenant

---

### FR-12.4: View Supplier Outage Page

**Function:** `getSupplierOutageUrl()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| supplierId | UUID | Yes | Supplier ID |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| outageUrl | string | Supplier outage page URL |
| supplierName | string | Supplier name |

**Business Rules:**
- BR-12.4.1: Open in external browser

**API Endpoint:** `GET /api/v1/suppliers/{supplierId}/outage-url`

**Authorization:** Owner, Tenant

---

### FR-12.5: Report Unplanned Outage

**Function:** `reportOutage()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| commodityType | enum | Yes | `water` \| `electricity` \| `gas` \| `heating` |
| description | string | Yes | Outage description |
| affectedAreas | array | No | Affected units/areas |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| outageId | UUID | Created outage ID |
| status | string | "reported" |
| reportedAt | datetime | Report timestamp |

**Business Rules:**
- BR-12.5.1: Notify building managers
- BR-12.5.2: Check for duplicate reports

**API Endpoint:** `POST /api/v1/buildings/{buildingId}/outages/report`

**Authorization:** Owner, Tenant

---

### FR-12.6: Subscribe to Outage Notifications

**Function:** `subscribeToOutages()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| userId | UUID | Yes | User ID |
| commodityTypes | array | Yes | Types to subscribe to |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| subscriptions | array | Active subscriptions |

**Business Rules:**
- BR-12.6.1: Per-user, per-building
- BR-12.6.2: Default: all types subscribed

**API Endpoint:** `PUT /api/v1/outages/subscriptions`

**Authorization:** Owner, Tenant

---

### FR-12.7: View Outage History

**Function:** `getOutageHistory()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| yearFrom | integer | No | Start year |
| yearTo | integer | No | End year |
| commodityType | enum | No | Filter by type |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| items | array | Historical outages |
| statistics | object | Outage statistics |

**Business Rules:**
- BR-12.7.1: Include duration calculations
- BR-12.7.2: Default last year

**API Endpoint:** `GET /api/v1/buildings/{buildingId}/outages/history`

**Authorization:** Owner, Tenant, Manager

---

### FR-12.8: Add Outage

**Function:** `createOutage()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| commodityType | enum | Yes | Outage type |
| status | enum | Yes | `active` \| `planned` |
| startTime | datetime | Yes | Start time |
| endTime | datetime | No | Expected end |
| description | string | Yes | Outage details |
| supplierId | UUID | No | Related supplier |
| notifyResidents | boolean | No | Send notifications |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| outageId | UUID | Created outage ID |
| notificationsSent | integer | Notifications sent |

**Business Rules:**
- BR-12.8.1: Planned outages require endTime
- BR-12.8.2: Notify subscribed users

**API Endpoint:** `POST /api/v1/buildings/{buildingId}/outages`

**Authorization:** Manager

---

## FR-13: News

### FR-13.1: Search News

**Function:** `searchNews()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| query | string | Yes | Search query |
| page | integer | No | Page number |
| limit | integer | No | Items per page |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| items | array | Matching articles |
| pagination | object | Pagination metadata |

**Business Rules:**
- BR-13.1.1: Full-text search on title/content
- BR-13.1.2: Only published articles

**API Endpoint:** `GET /api/v1/buildings/{buildingId}/news/search`

**Authorization:** Owner, Tenant, Manager

---

### FR-13.2: View News Article

**Function:** `getNewsArticle()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| articleId | UUID | Yes | Article ID |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| id | UUID | Article ID |
| title | string | Headline |
| content | string | Full content |
| author | object | Author info |
| attachments | array | Attached files |
| reactionCount | integer | Number of reactions |
| commentCount | integer | Number of comments |
| publishedAt | datetime | Publication date |
| viewCount | integer | View count |

**Business Rules:**
- BR-13.2.1: Increment view count
- BR-13.2.2: Track reader analytics

**API Endpoint:** `GET /api/v1/news/{articleId}`

**Authorization:** Owner, Tenant, Manager

---

### FR-13.3: React to News Article

**Function:** `reactToNews()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| articleId | UUID | Yes | Article ID |
| reaction | enum | Yes | `like` (toggle) |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| articleId | UUID | Article ID |
| userReacted | boolean | Current reaction state |
| reactionCount | integer | Total reactions |

**Business Rules:**
- BR-13.3.1: Toggle reaction on/off
- BR-13.3.2: One reaction per user per article

**API Endpoint:** `POST /api/v1/news/{articleId}/react`

**Authorization:** Owner, Tenant

---

### FR-13.4: Publish News Article

**Function:** `createNewsArticle()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| title | string | Yes | Article headline |
| content | string | Yes | Article content |
| attachments | array | No | File attachments |
| status | enum | No | `draft` \| `published` |
| scheduledAt | datetime | No | Scheduled publication |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| articleId | UUID | Created article ID |
| status | string | Current status |
| publishedAt | datetime | Publication timestamp |

**Business Rules:**
- BR-13.4.1: Max 5 attachments
- BR-13.4.2: Notify building residents

**API Endpoint:** `POST /api/v1/buildings/{buildingId}/news`

**Authorization:** Manager

---

### FR-13.5: Edit News Article

**Function:** `updateNewsArticle()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| articleId | UUID | Yes | Article ID |
| title | string | No | Updated title |
| content | string | No | Updated content |
| attachments | array | No | Updated attachments |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| articleId | UUID | Article ID |
| updatedAt | datetime | Update timestamp |

**Business Rules:**
- BR-13.5.1: Only author or admin can edit
- BR-13.5.2: Log edits

**API Endpoint:** `PUT /api/v1/news/{articleId}`

**Authorization:** Manager

---

### FR-13.6: Delete News Article

**Function:** `deleteNewsArticle()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| articleId | UUID | Yes | Article ID |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| success | boolean | Whether deletion succeeded |

**Business Rules:**
- BR-13.6.1: Soft delete
- BR-13.6.2: Delete associated comments

**API Endpoint:** `DELETE /api/v1/news/{articleId}`

**Authorization:** Manager

---

### FR-13.7: Archive News Article

**Function:** `archiveNewsArticle()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| articleId | UUID | Yes | Article ID |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| articleId | UUID | Article ID |
| status | string | "archived" |
| archivedAt | datetime | Archive timestamp |

**Business Rules:**
- BR-13.7.1: Remove from main feed
- BR-13.7.2: Remain searchable

**API Endpoint:** `POST /api/v1/news/{articleId}/archive`

**Authorization:** Manager

---

### FR-13.8: Comment on News Article

**Function:** `addNewsComment()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| articleId | UUID | Yes | Article ID |
| content | string | Yes | Comment text |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| commentId | UUID | Comment ID |
| content | string | Comment content |
| createdAt | datetime | Creation timestamp |

**Business Rules:**
- BR-13.8.1: Max 1000 characters
- BR-13.8.2: Moderate for spam

**API Endpoint:** `POST /api/v1/news/{articleId}/comments`

**Authorization:** Owner, Tenant, Manager

---

### FR-13.9: Share News Article

**Function:** `shareNewsArticle()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| articleId | UUID | Yes | Article ID |
| method | enum | Yes | `email` \| `link` |
| recipientEmail | string | No | Email recipient |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| shareUrl | string | Public share URL |
| emailSent | boolean | Whether email was sent |

**Business Rules:**
- BR-13.9.1: Generate temporary share link
- BR-13.9.2: Track share count

**API Endpoint:** `POST /api/v1/news/{articleId}/share`

**Authorization:** Owner, Tenant

---

## FR-15: Building/Property Management

### FR-15.1: Add Building

**Function:** `createBuilding()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| name | string | Yes | Building name |
| address | object | Yes | Building address |
| address.street | string | Yes | Street address |
| address.city | string | Yes | City |
| address.postalCode | string | Yes | Postal code |
| address.country | string | Yes | Country code |
| address.coordinates | object | No | GPS coordinates |
| totalUnits | integer | Yes | Number of units |
| floors | integer | Yes | Number of floors |
| entrances | array | No | Entrance identifiers |
| yearBuilt | integer | No | Construction year |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| buildingId | UUID | Created building ID |
| createdAt | datetime | Creation timestamp |

**Business Rules:**
- BR-15.1.1: Address must be valid
- BR-15.1.2: Geocode address automatically
- BR-15.1.3: Create default document folders

**API Endpoint:** `POST /api/v1/buildings`

**Authorization:** Manager, System Administrator

---

### FR-15.2: Edit Building Information

**Function:** `updateBuilding()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| name | string | No | Updated name |
| address | object | No | Updated address |
| totalUnits | integer | No | Updated unit count |
| floors | integer | No | Updated floors |
| entrances | array | No | Updated entrances |
| metadata | object | No | Additional metadata |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| buildingId | UUID | Building ID |
| updatedAt | datetime | Update timestamp |

**Business Rules:**
- BR-15.2.1: Re-geocode if address changes
- BR-15.2.2: Log all changes

**API Endpoint:** `PUT /api/v1/buildings/{buildingId}`

**Authorization:** Manager, System Administrator

---

### FR-15.3: View Building Details

**Function:** `getBuilding()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| id | UUID | Building ID |
| name | string | Building name |
| address | object | Full address |
| coordinates | object | GPS coordinates |
| totalUnits | integer | Total units |
| occupiedUnits | integer | Occupied units |
| floors | integer | Number of floors |
| entrances | array | Entrances |
| yearBuilt | integer | Construction year |
| managers | array | Assigned managers |
| createdAt | datetime | Creation date |

**Business Rules:**
- BR-15.3.1: Calculate occupancy stats
- BR-15.3.2: Include assigned managers

**API Endpoint:** `GET /api/v1/buildings/{buildingId}`

**Authorization:** Owner, Tenant, Manager

---

### FR-15.4: Add Unit to Building

**Function:** `createUnit()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| unitNumber | string | Yes | Unit number/identifier |
| floor | integer | Yes | Floor number |
| entrance | string | No | Entrance identifier |
| type | enum | Yes | `apartment` \| `commercial` \| `parking` \| `storage` |
| size | number | No | Size in sq meters |
| rooms | integer | No | Number of rooms |
| ownershipShare | number | No | Ownership share percentage |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| unitId | UUID | Created unit ID |
| unitNumber | string | Unit number |

**Business Rules:**
- BR-15.4.1: Unit number unique in building
- BR-15.4.2: Create associated meters

**API Endpoint:** `POST /api/v1/buildings/{buildingId}/units`

**Authorization:** Manager, System Administrator

---

### FR-15.5: Edit Unit Information

**Function:** `updateUnit()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| unitId | UUID | Yes | Unit ID |
| unitNumber | string | No | Updated number |
| floor | integer | No | Updated floor |
| type | enum | No | Updated type |
| size | number | No | Updated size |
| rooms | integer | No | Updated rooms |
| ownershipShare | number | No | Updated share |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| unitId | UUID | Unit ID |
| updatedAt | datetime | Update timestamp |

**Business Rules:**
- BR-15.5.1: Preserve unit number history
- BR-15.5.2: Log changes

**API Endpoint:** `PUT /api/v1/units/{unitId}`

**Authorization:** Manager, System Administrator

---

### FR-15.6: Assign Owner to Unit

**Function:** `assignOwnerToUnit()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| unitId | UUID | Yes | Unit ID |
| ownerId | UUID | Yes | Owner user ID |
| ownershipPercentage | number | No | Ownership % (for co-owners) |
| effectiveDate | date | No | Assignment start date |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| unitId | UUID | Unit ID |
| ownerId | UUID | Assigned owner |
| assignedAt | datetime | Assignment timestamp |

**Business Rules:**
- BR-15.6.1: Multiple owners allowed (co-ownership)
- BR-15.6.2: Total ownership <= 100%
- BR-15.6.3: Notify new owner

**API Endpoint:** `POST /api/v1/units/{unitId}/owners`

**Authorization:** Manager, System Administrator

---

### FR-15.7: View Building Statistics

**Function:** `getBuildingStatistics()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| period | enum | No | `month` \| `quarter` \| `year` |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| occupancy | object | Occupancy statistics |
| faults | object | Fault statistics |
| payments | object | Payment statistics |
| participation | object | Voting participation |
| consumption | object | Utility consumption |

**Business Rules:**
- BR-15.7.1: Cache stats for performance
- BR-15.7.2: Real-time for current period

**API Endpoint:** `GET /api/v1/buildings/{buildingId}/statistics`

**Authorization:** Manager

---

### FR-15.8: Bulk Import Buildings

**Function:** `importBuildings()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| file | file | Yes | CSV file with buildings |
| dryRun | boolean | No | Validate only |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| importedCount | integer | Buildings imported |
| skippedCount | integer | Rows skipped |
| errors | array | Import errors |

**Business Rules:**
- BR-15.8.1: Validate all rows before import
- BR-15.8.2: Support CSV and XLSX

**API Endpoint:** `POST /api/v1/buildings/import`

**Authorization:** System Administrator, Organization Admin

---

### FR-15.9: Merge Duplicate Buildings

**Function:** `mergeBuildings()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| sourceId | UUID | Yes | Building to merge from |
| targetId | UUID | Yes | Building to merge into |
| mergeOptions | object | No | What to merge |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| targetId | UUID | Resulting building ID |
| mergedUnits | integer | Units merged |
| mergedDocuments | integer | Documents merged |

**Business Rules:**
- BR-15.9.1: Cannot undo merge
- BR-15.9.2: Preserve all history
- BR-15.9.3: Redirect references

**API Endpoint:** `POST /api/v1/buildings/merge`

**Authorization:** System Administrator

---

### FR-15.10: Archive Building

**Function:** `archiveBuilding()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| reason | string | No | Archive reason |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| buildingId | UUID | Building ID |
| status | string | "archived" |
| archivedAt | datetime | Archive timestamp |

**Business Rules:**
- BR-15.10.1: Preserve all data
- BR-15.10.2: Remove from active lists
- BR-15.10.3: Notify managers

**API Endpoint:** `POST /api/v1/buildings/{buildingId}/archive`

**Authorization:** Manager, System Administrator

---

## FR-16: Financial Management

### FR-16.1: View Account Balance

**Function:** `getAccountBalance()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| unitId | UUID | Yes | Unit ID |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| currentBalance | number | Current balance |
| outstandingAmount | number | Amount due |
| overdueAmount | number | Overdue amount |
| creditAmount | number | Credit if overpaid |
| lastPaymentDate | datetime | Last payment |
| nextDueDate | datetime | Next payment due |

**Business Rules:**
- BR-16.1.1: Calculate from all transactions
- BR-16.1.2: Include pending payments

**API Endpoint:** `GET /api/v1/units/{unitId}/balance`

**Authorization:** Owner

---

### FR-16.2: View Payment History

**Function:** `getPaymentHistory()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| unitId | UUID | Yes | Unit ID |
| yearFrom | integer | No | Start year |
| yearTo | integer | No | End year |
| page | integer | No | Page number |
| limit | integer | No | Items per page |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| items | array | Payment records |
| items[].id | UUID | Payment ID |
| items[].amount | number | Payment amount |
| items[].date | date | Payment date |
| items[].type | string | Payment type |
| items[].reference | string | Reference number |
| items[].status | string | Payment status |
| pagination | object | Pagination metadata |

**Business Rules:**
- BR-16.2.1: Sort by date descending
- BR-16.2.2: Include invoice links

**API Endpoint:** `GET /api/v1/units/{unitId}/payments`

**Authorization:** Owner, Manager

---

### FR-16.3: Make Payment

**Function:** `createPayment()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| unitId | UUID | Yes | Unit ID |
| amount | number | Yes | Payment amount |
| invoiceId | UUID | No | Specific invoice |
| paymentMethod | enum | Yes | `card` \| `bank_transfer` |
| returnUrl | string | No | Return URL after payment |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| paymentId | UUID | Payment ID |
| paymentUrl | string | Payment gateway URL |
| expiresAt | datetime | Payment session expiry |

**Business Rules:**
- BR-16.3.1: Redirect to payment gateway
- BR-16.3.2: Handle webhook callback
- BR-16.3.3: Update balance on success

**API Endpoint:** `POST /api/v1/units/{unitId}/payments`

**Authorization:** Owner

---

### FR-16.4: Generate Invoice

**Function:** `generateInvoice()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| unitId | UUID | Yes | Unit ID |
| items | array | Yes | Invoice line items |
| items[].description | string | Yes | Item description |
| items[].amount | number | Yes | Item amount |
| dueDate | date | Yes | Payment due date |
| notes | string | No | Additional notes |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| invoiceId | UUID | Invoice ID |
| invoiceNumber | string | Invoice number |
| totalAmount | number | Total amount |
| pdfUrl | string | Invoice PDF URL |

**Business Rules:**
- BR-16.4.1: Auto-generate invoice number
- BR-16.4.2: Send email notification
- BR-16.4.3: Generate PDF

**API Endpoint:** `POST /api/v1/units/{unitId}/invoices`

**Authorization:** Manager

---

### FR-16.5: Export Financial Report

**Function:** `exportFinancialReport()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| reportType | enum | Yes | `balance` \| `income` \| `expense` |
| dateFrom | date | Yes | Start date |
| dateTo | date | Yes | End date |
| format | enum | Yes | `pdf` \| `xlsx` |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| downloadUrl | string | Report URL |
| filename | string | File name |

**Business Rules:**
- BR-16.5.1: Include all transactions
- BR-16.5.2: Group by category

**API Endpoint:** `GET /api/v1/buildings/{buildingId}/financial-reports/export`

**Authorization:** Manager

---

### FR-16.6: View Annual Settlement

**Function:** `getAnnualSettlement()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| unitId | UUID | Yes | Unit ID |
| year | integer | Yes | Settlement year |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| year | integer | Settlement year |
| totalCharges | number | Total charges |
| totalPayments | number | Total payments |
| balance | number | Year-end balance |
| breakdown | array | Itemized breakdown |
| pdfUrl | string | Settlement PDF |

**Business Rules:**
- BR-16.6.1: Include all charge types
- BR-16.6.2: Compare with previous year

**API Endpoint:** `GET /api/v1/units/{unitId}/settlements/{year}`

**Authorization:** Owner

---

### FR-16.7: Download Invoice PDF

**Function:** `downloadInvoice()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| invoiceId | UUID | Yes | Invoice ID |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| downloadUrl | string | Signed download URL |
| filename | string | PDF filename |

**Business Rules:**
- BR-16.7.1: URL valid for 1 hour
- BR-16.7.2: Include QR code for payment

**API Endpoint:** `GET /api/v1/invoices/{invoiceId}/download`

**Authorization:** Owner

---

### FR-16.8: Reconcile Payments

**Function:** `reconcilePayments()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| bankStatementId | UUID | Yes | Bank statement ID |
| matchings | array | Yes | Payment matchings |
| matchings[].transactionId | UUID | Yes | Bank transaction |
| matchings[].invoiceId | UUID | Yes | Matched invoice |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| reconciledCount | integer | Payments reconciled |
| unmatchedCount | integer | Unmatched transactions |

**Business Rules:**
- BR-16.8.1: Auto-match by reference
- BR-16.8.2: Manual matching for ambiguous

**API Endpoint:** `POST /api/v1/buildings/{buildingId}/reconcile`

**Authorization:** Manager

---

### FR-16.9: Process Refund

**Function:** `processRefund()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| paymentId | UUID | Yes | Original payment ID |
| amount | number | Yes | Refund amount |
| reason | string | Yes | Refund reason |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| refundId | UUID | Refund ID |
| status | string | Refund status |
| processedAt | datetime | Processing timestamp |

**Business Rules:**
- BR-16.9.1: Amount <= original payment
- BR-16.9.2: Notify owner
- BR-16.9.3: Update balance

**API Endpoint:** `POST /api/v1/payments/{paymentId}/refund`

**Authorization:** Manager

---

### FR-16.10: Configure Late Payment Fees

**Function:** `configureLatePaymentFees()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| enabled | boolean | Yes | Enable late fees |
| gracePeriodDays | integer | Yes | Days before fee |
| feeType | enum | Yes | `fixed` \| `percentage` |
| feeAmount | number | Yes | Fee amount or % |
| maxFee | number | No | Maximum fee cap |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| buildingId | UUID | Building ID |
| config | object | Updated configuration |

**Business Rules:**
- BR-16.10.1: Grace period minimum 7 days
- BR-16.10.2: Apply automatically

**API Endpoint:** `PUT /api/v1/buildings/{buildingId}/late-fees`

**Authorization:** Manager, Organization Admin

---

## FR-17: Reports and Analytics

### FR-17.1: Generate Fault Statistics Report

**Function:** `generateFaultReport()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| dateFrom | date | Yes | Start date |
| dateTo | date | Yes | End date |
| groupBy | enum | No | `category` \| `status` \| `month` |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| totalFaults | integer | Total faults |
| byCategory | object | Faults by category |
| byStatus | object | Faults by status |
| avgResolutionTime | number | Average hours to resolve |
| trends | array | Monthly trends |

**Business Rules:**
- BR-17.1.1: Include resolution metrics
- BR-17.1.2: Compare with previous period

**API Endpoint:** `GET /api/v1/buildings/{buildingId}/reports/faults`

**Authorization:** Manager

---

### FR-17.2: Generate Voting Participation Report

**Function:** `generateVotingReport()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| year | integer | Yes | Report year |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| totalVotes | integer | Total votes held |
| avgParticipation | number | Average participation % |
| voteDetails | array | Per-vote breakdown |
| unitParticipation | array | Per-unit participation |

**Business Rules:**
- BR-17.2.1: Exclude cancelled votes
- BR-17.2.2: Identify low-participation units

**API Endpoint:** `GET /api/v1/buildings/{buildingId}/reports/voting`

**Authorization:** Manager

---

### FR-17.3: Generate Occupancy Report

**Function:** `generateOccupancyReport()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| year | integer | Yes | Report year |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| avgOccupancy | number | Average persons/unit |
| monthlyData | array | Monthly breakdown |
| unitDetails | array | Per-unit occupancy |
| totalPersonMonths | integer | Total person-months |

**Business Rules:**
- BR-17.3.1: Based on person-month data
- BR-17.3.2: Flag missing data

**API Endpoint:** `GET /api/v1/buildings/{buildingId}/reports/occupancy`

**Authorization:** Manager

---

### FR-17.4: Generate Consumption Report

**Function:** `generateConsumptionReport()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| commodityType | enum | Yes | `water` \| `electricity` \| `gas` |
| year | integer | Yes | Report year |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| totalConsumption | number | Total consumption |
| avgPerUnit | number | Average per unit |
| monthlyData | array | Monthly breakdown |
| unitRanking | array | Units by consumption |
| anomalies | array | Unusual patterns |

**Business Rules:**
- BR-17.4.1: Based on meter readings
- BR-17.4.2: Highlight anomalies

**API Endpoint:** `GET /api/v1/buildings/{buildingId}/reports/consumption`

**Authorization:** Manager

---

### FR-17.5: Export Report to PDF/Excel

**Function:** `exportReport()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| reportType | enum | Yes | Report type |
| reportId | UUID | Yes | Report ID |
| format | enum | Yes | `pdf` \| `xlsx` |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| downloadUrl | string | Export URL |
| filename | string | File name |
| expiresAt | datetime | URL expiration |

**Business Rules:**
- BR-17.5.1: Include charts in PDF
- BR-17.5.2: Raw data in Excel

**API Endpoint:** `GET /api/v1/reports/{reportId}/export`

**Authorization:** Manager

---

## FR-18: System Administration

### FR-18.1: Manage User Roles

**Function:** `updateUserRole()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| userId | UUID | Yes | User ID |
| role | enum | Yes | New role |
| buildingIds | array | No | Assigned buildings |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| userId | UUID | User ID |
| role | string | Updated role |
| permissions | array | Role permissions |

**Business Rules:**
- BR-18.1.1: Cannot demote last admin
- BR-18.1.2: Log role changes

**API Endpoint:** `PUT /api/v1/users/{userId}/role`

**Authorization:** System Administrator

---

### FR-18.2: View Audit Log

**Function:** `getAuditLog()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| dateFrom | datetime | No | Start date |
| dateTo | datetime | No | End date |
| userId | UUID | No | Filter by user |
| action | string | No | Filter by action type |
| page | integer | No | Page number |
| limit | integer | No | Items per page |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| items | array | Audit entries |
| items[].id | UUID | Entry ID |
| items[].action | string | Action performed |
| items[].userId | UUID | Acting user |
| items[].resourceType | string | Resource type |
| items[].resourceId | UUID | Resource ID |
| items[].details | object | Action details |
| items[].ip | string | IP address |
| items[].timestamp | datetime | Event timestamp |
| pagination | object | Pagination metadata |

**Business Rules:**
- BR-18.2.1: Retain 2 years
- BR-18.2.2: Cannot modify logs

**API Endpoint:** `GET /api/v1/audit-log`

**Authorization:** System Administrator

---

### FR-18.3: Configure System Settings

**Function:** `updateSystemSettings()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| settings | object | Yes | Settings to update |
| settings.defaultLanguage | string | No | Default language |
| settings.sessionTimeout | integer | No | Session timeout minutes |
| settings.passwordPolicy | object | No | Password requirements |
| settings.features | object | No | Feature toggles |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| settings | object | Updated settings |
| updatedAt | datetime | Update timestamp |

**Business Rules:**
- BR-18.3.1: Validate setting values
- BR-18.3.2: Some settings require restart

**API Endpoint:** `PUT /api/v1/settings`

**Authorization:** System Administrator

---

### FR-18.4: Manage Email Templates

**Function:** `updateEmailTemplate()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| templateId | string | Yes | Template identifier |
| subject | string | No | Email subject |
| body | string | No | Email body (HTML) |
| variables | array | No | Available variables |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| templateId | string | Template ID |
| updatedAt | datetime | Update timestamp |

**Business Rules:**
- BR-18.4.1: Support variable placeholders
- BR-18.4.2: Preview before save

**API Endpoint:** `PUT /api/v1/email-templates/{templateId}`

**Authorization:** System Administrator

---

### FR-18.5: Backup Data

**Function:** `createBackup()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| includeDocuments | boolean | No | Include file storage |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| backupId | UUID | Backup ID |
| status | string | "in_progress" |
| estimatedSize | integer | Estimated size bytes |

**Business Rules:**
- BR-18.5.1: Run async
- BR-18.5.2: Encrypt backup
- BR-18.5.3: Notify when complete

**API Endpoint:** `POST /api/v1/backups`

**Authorization:** System Administrator

---

### FR-18.6: View System Statistics

**Function:** `getSystemStatistics()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| period | enum | No | `day` \| `week` \| `month` |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| users | object | User statistics |
| buildings | object | Building statistics |
| activity | object | Activity metrics |
| storage | object | Storage usage |
| performance | object | Performance metrics |

**Business Rules:**
- BR-18.6.1: Real-time for current period
- BR-18.6.2: Cache historical data

**API Endpoint:** `GET /api/v1/system/statistics`

**Authorization:** System Administrator

---

## FR-19: Real-time & Mobile Features

### FR-19.1: Real-time Fault Status Updates

**Function:** `subscribeFaultUpdates()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| faultId | UUID | Yes | Fault ID |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| subscriptionId | UUID | Subscription ID |
| channel | string | WebSocket channel |

**Business Rules:**
- BR-19.1.1: Push on status change
- BR-19.1.2: Push on new comments

**API Endpoint:** WebSocket `/ws/faults/{faultId}`

**Authorization:** Owner, Tenant, Manager

---

### FR-19.2: Live Voting Results

**Function:** `subscribeVoteResults()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| voteId | UUID | Yes | Vote ID |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| subscriptionId | UUID | Subscription ID |
| channel | string | WebSocket channel |

**Business Rules:**
- BR-19.2.1: Only if showResultsBeforeEnd enabled
- BR-19.2.2: Update on each vote cast

**API Endpoint:** WebSocket `/ws/votes/{voteId}/results`

**Authorization:** Owner, Manager

---

### FR-19.3: Typing Indicators in Messages

**Function:** `sendTypingIndicator()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| conversationId | UUID | Yes | Conversation ID |
| isTyping | boolean | Yes | Typing state |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| sent | boolean | Whether sent successfully |

**Business Rules:**
- BR-19.3.1: Throttle to 1 per second
- BR-19.3.2: Auto-expire after 5 seconds

**API Endpoint:** WebSocket `/ws/conversations/{conversationId}/typing`

**Authorization:** Owner, Tenant, Manager

---

### FR-19.4: Presence Indicators

**Function:** `getOnlineStatus()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| userIds | array | Yes | User IDs to check |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| statuses | object | User ID -> status map |
| statuses[userId] | enum | `online` \| `away` \| `offline` |

**Business Rules:**
- BR-19.4.1: Away after 5 min inactivity
- BR-19.4.2: Offline after 15 min

**API Endpoint:** `GET /api/v1/users/presence`

**Authorization:** Owner, Tenant

---

### FR-19.5: Live Document Collaboration

**Function:** `joinDocumentSession()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| documentId | UUID | Yes | Document ID |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| sessionId | UUID | Collaboration session |
| activeUsers | array | Currently editing users |
| channel | string | WebSocket channel |

**Business Rules:**
- BR-19.5.1: Operational transformation for sync
- BR-19.5.2: Max 10 concurrent editors

**API Endpoint:** WebSocket `/ws/documents/{documentId}/collaborate`

**Authorization:** Manager

---

### FR-19.6: Offline Mode Support

**Function:** `syncOfflineData()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| lastSyncAt | datetime | Yes | Last sync timestamp |
| pendingActions | array | Yes | Queued offline actions |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| syncedAt | datetime | New sync timestamp |
| data | object | Updated cached data |
| processedActions | array | Processed action results |
| conflicts | array | Any conflicts |

**Business Rules:**
- BR-19.6.1: Last-write-wins for conflicts
- BR-19.6.2: Queue actions for replay

**API Endpoint:** `POST /api/v1/sync`

**Authorization:** Owner, Tenant, Manager

---

### FR-19.7: Background Sync

**Function:** `registerBackgroundSync()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| deviceId | UUID | Yes | Device ID |
| syncTypes | array | Yes | Data types to sync |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| registrationId | UUID | Registration ID |
| syncSchedule | object | Sync configuration |

**Business Rules:**
- BR-19.7.1: Sync on connectivity restore
- BR-19.7.2: Battery-aware scheduling

**API Endpoint:** `POST /api/v1/sync/register`

**Authorization:** Owner, Tenant, Manager

---

### FR-19.8: Low Bandwidth Mode

**Function:** `setLowBandwidthMode()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| userId | UUID | Yes | User ID |
| enabled | boolean | Yes | Enable low bandwidth |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| enabled | boolean | Current state |
| imageQuality | string | "low" \| "high" |
| features | object | Disabled features |

**Business Rules:**
- BR-19.8.1: Compress images to 100KB
- BR-19.8.2: Disable auto-play

**API Endpoint:** `PUT /api/v1/users/{userId}/bandwidth-mode`

**Authorization:** Owner, Tenant

---

### FR-19.9: Progressive Web App Installation

**Function:** `getPwaManifest()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| manifest | object | Web app manifest |
| icons | array | App icons |
| shortcuts | array | Quick actions |

**Business Rules:**
- BR-19.9.1: Branded per organization
- BR-19.9.2: Support home screen install

**API Endpoint:** `GET /api/v1/pwa/manifest`

**Authorization:** Public

---

### FR-19.10: Switch Application Language

**Function:** `setUserLanguage()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| userId | UUID | Yes | User ID |
| language | string | Yes | Language code (en, sk, cs, etc.) |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| language | string | Updated language |

**Business Rules:**
- BR-19.10.1: Persist in user preferences
- BR-19.10.2: Reload UI strings

**API Endpoint:** `PUT /api/v1/users/{userId}/language`

**Authorization:** Owner, Tenant, Manager

---

### FR-19.11: Auto-translate Announcements

**Function:** `translateContent()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| contentId | UUID | Yes | Content ID |
| targetLanguage | string | Yes | Target language code |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| translatedContent | string | Translated text |
| sourceLanguage | string | Detected source |
| confidence | number | Translation confidence |

**Business Rules:**
- BR-19.11.1: Cache translations
- BR-19.11.2: Human review for important content

**API Endpoint:** `POST /api/v1/translate`

**Authorization:** AI System

---

### FR-19.12: Multi-language Document Support

**Function:** `detectDocumentLanguage()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| documentId | UUID | Yes | Document ID |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| detectedLanguage | string | Language code |
| confidence | number | Detection confidence |
| alternativeLanguages | array | Other possible languages |

**Business Rules:**
- BR-19.12.1: Run on upload
- BR-19.12.2: Store in metadata

**API Endpoint:** `POST /api/v1/documents/{documentId}/detect-language`

**Authorization:** Manager

---

## FR-20: AI/ML Features

### FR-20.1: AI Chatbot for Common Questions

**Function:** `askChatbot()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| userId | UUID | Yes | User ID |
| message | string | Yes | User question |
| conversationId | UUID | No | Existing conversation |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| response | string | AI response |
| conversationId | UUID | Conversation ID |
| suggestedActions | array | Suggested next actions |
| confidence | number | Response confidence |

**Business Rules:**
- BR-20.1.1: Escalate to human if low confidence
- BR-20.1.2: Learn from feedback

**API Endpoint:** `POST /api/v1/ai/chatbot`

**Authorization:** Owner, Tenant

---

### FR-20.2: AI-Assisted Fault Reporting

**Function:** `assistFaultReport()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| userDescription | string | Yes | Initial description |
| photos | array | No | Uploaded photos |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| suggestedCategory | string | Suggested category |
| suggestedPriority | string | Suggested priority |
| clarifyingQuestions | array | Questions to ask |
| similarFaults | array | Similar past faults |

**Business Rules:**
- BR-20.2.1: Analyze photos for category
- BR-20.2.2: Check for duplicates

**API Endpoint:** `POST /api/v1/ai/assist-fault`

**Authorization:** Owner, Tenant, AI System

---

### FR-20.3: AI Fault Categorization

**Function:** `categorizeFault()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| faultId | UUID | Yes | Fault ID |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| category | string | Assigned category |
| subcategory | string | Subcategory |
| tags | array | Auto-generated tags |
| confidence | number | Categorization confidence |

**Business Rules:**
- BR-20.3.1: Run on fault creation
- BR-20.3.2: Manager can override

**API Endpoint:** `POST /api/v1/ai/categorize-fault`

**Authorization:** AI System

---

### FR-20.4: AI Response Suggestions

**Function:** `suggestResponse()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| messageId | UUID | Yes | Message to respond to |
| context | object | No | Additional context |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| suggestions | array | Response suggestions |
| suggestions[].text | string | Suggested response |
| suggestions[].tone | string | Response tone |

**Business Rules:**
- BR-20.4.1: Learn from accepted suggestions
- BR-20.4.2: Personalize to manager style

**API Endpoint:** `POST /api/v1/ai/suggest-response`

**Authorization:** Manager, AI System

---

### FR-20.5: Virtual Building Assistant

**Function:** `voiceQuery()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| userId | UUID | Yes | User ID |
| audioData | binary | Yes | Voice input |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| transcription | string | Voice to text |
| response | string | AI response |
| audioResponse | binary | Text to speech |
| actions | array | Executable actions |

**Business Rules:**
- BR-20.5.1: Support multiple languages
- BR-20.5.2: Handle voice commands

**API Endpoint:** `POST /api/v1/ai/voice-assistant`

**Authorization:** Owner, Tenant, AI System

---

### FR-20.6: Predict Maintenance Needs

**Function:** `predictMaintenance()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| equipmentType | string | No | Equipment filter |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| predictions | array | Maintenance predictions |
| predictions[].equipment | string | Equipment name |
| predictions[].predictedDate | date | Predicted need |
| predictions[].confidence | number | Prediction confidence |
| predictions[].basis | array | Factors used |

**Business Rules:**
- BR-20.6.1: Based on fault history
- BR-20.6.2: Consider equipment age

**API Endpoint:** `GET /api/v1/ai/predict-maintenance`

**Authorization:** Manager, AI System

---

### FR-20.7: Consumption Anomaly Detection

**Function:** `detectConsumptionAnomalies()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| period | enum | Yes | `month` \| `quarter` |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| anomalies | array | Detected anomalies |
| anomalies[].unitId | UUID | Affected unit |
| anomalies[].type | string | Anomaly type |
| anomalies[].deviation | number | Standard deviations |
| anomalies[].possibleCauses | array | Possible explanations |

**Business Rules:**
- BR-20.7.1: Alert if >2 std dev
- BR-20.7.2: Weekly analysis

**API Endpoint:** `GET /api/v1/ai/consumption-anomalies`

**Authorization:** Manager, AI System

---

### FR-20.8: Predict Payment Delays

**Function:** `predictPaymentDelays()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| predictions | array | Risk predictions |
| predictions[].unitId | UUID | Unit ID |
| predictions[].riskScore | number | Risk score 0-100 |
| predictions[].factors | array | Risk factors |
| predictions[].recommendation | string | Suggested action |

**Business Rules:**
- BR-20.8.1: Based on payment history
- BR-20.8.2: GDPR compliant

**API Endpoint:** `GET /api/v1/ai/payment-risk`

**Authorization:** Manager, AI System

---

### FR-20.9: Fault Resolution Time Prediction

**Function:** `predictResolutionTime()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| faultId | UUID | Yes | Fault ID |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| estimatedHours | number | Estimated resolution hours |
| confidence | number | Prediction confidence |
| factors | array | Influencing factors |
| similarFaults | array | Similar historical faults |

**Business Rules:**
- BR-20.9.1: Run on fault creation
- BR-20.9.2: Update on status change

**API Endpoint:** `GET /api/v1/ai/predict-resolution`

**Authorization:** Owner, Tenant, AI System

---

### FR-20.10: Occupancy Prediction

**Function:** `predictOccupancy()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| months | integer | Yes | Months to predict |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| predictions | array | Monthly predictions |
| predictions[].month | date | Prediction month |
| predictions[].avgOccupancy | number | Predicted avg persons |
| predictions[].confidence | number | Prediction confidence |

**Business Rules:**
- BR-20.10.1: Based on historical trends
- BR-20.10.2: Consider seasonality

**API Endpoint:** `GET /api/v1/ai/predict-occupancy`

**Authorization:** Manager, AI System

---

### FR-20.11: OCR Meter Reading

**Function:** `ocrMeterReading()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| photo | file | Yes | Meter photo |
| meterType | enum | Yes | `water` \| `electricity` \| `gas` |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| readingValue | number | Extracted reading |
| confidence | number | OCR confidence |
| boundingBox | object | Reading location in image |
| needsVerification | boolean | Low confidence flag |

**Business Rules:**
- BR-20.11.1: Request verification if <80% confidence
- BR-20.11.2: Support various meter types

**API Endpoint:** `POST /api/v1/ai/ocr-meter`

**Authorization:** Owner, AI System

---

### FR-20.12: AI Fault Image Analysis

**Function:** `analyzeFaultImage()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| faultId | UUID | Yes | Fault ID |
| photo | file | Yes | Fault photo |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| detectedIssues | array | Detected issues |
| severity | enum | `low` \| `medium` \| `high` \| `critical` |
| suggestedCategory | string | Suggested fault category |
| confidence | number | Analysis confidence |

**Business Rules:**
- BR-20.12.1: Run on photo upload
- BR-20.12.2: Update fault priority suggestion

**API Endpoint:** `POST /api/v1/ai/analyze-fault-image`

**Authorization:** AI System

---

### FR-20.13: Document OCR & Indexing

**Function:** `indexDocument()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| documentId | UUID | Yes | Document ID |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| extractedText | string | Full text content |
| keywords | array | Extracted keywords |
| entities | array | Named entities |
| language | string | Detected language |

**Business Rules:**
- BR-20.13.1: Run async on upload
- BR-20.13.2: Update search index

**API Endpoint:** `POST /api/v1/ai/index-document`

**Authorization:** Manager, AI System

---

### FR-20.14: Face Recognition for Access

**Function:** `verifyFaceAccess()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| faceImage | file | Yes | Face capture |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| matched | boolean | Match found |
| userId | UUID | Matched user ID |
| confidence | number | Match confidence |
| accessGranted | boolean | Access decision |

**Business Rules:**
- BR-20.14.1: Require >95% confidence
- BR-20.14.2: GDPR consent required

**API Endpoint:** `POST /api/v1/ai/face-access`

**Authorization:** Owner, Tenant, AI System

---

### FR-20.15: Damage Assessment from Photos

**Function:** `assessDamage()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| photos | array | Yes | Damage photos |
| damageType | string | No | Known damage type |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| severity | enum | Damage severity |
| estimatedCost | object | Cost estimate range |
| damageType | string | Detected damage type |
| repairRecommendations | array | Suggested repairs |
| confidence | number | Assessment confidence |

**Business Rules:**
- BR-20.15.1: Range estimate (min-max)
- BR-20.15.2: Flag for expert review

**API Endpoint:** `POST /api/v1/ai/assess-damage`

**Authorization:** Manager, AI System

---

### FR-20.16: Sentiment Analysis on Feedback

**Function:** `analyzeSentiment()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| period | enum | Yes | Analysis period |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| overallSentiment | number | -1 to 1 score |
| trend | string | Improving/declining |
| topIssues | array | Common complaints |
| positiveTopics | array | Positive feedback areas |

**Business Rules:**
- BR-20.16.1: Analyze messages and comments
- BR-20.16.2: Weekly summary

**API Endpoint:** `GET /api/v1/ai/sentiment-analysis`

**Authorization:** Manager, AI System

---

### FR-20.17: Smart Search with NLP

**Function:** `smartSearch()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| query | string | Yes | Natural language query |
| filters | object | No | Additional filters |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| results | array | Search results |
| results[].type | string | Result type |
| results[].item | object | Result item |
| results[].relevance | number | Relevance score |
| interpretation | string | Query interpretation |

**Business Rules:**
- BR-20.17.1: Search across all content types
- BR-20.17.2: Understand natural language

**API Endpoint:** `POST /api/v1/ai/smart-search`

**Authorization:** Owner, Tenant, Manager, AI System

---

### FR-20.18: Auto-summarize Long Documents

**Function:** `summarizeDocument()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| documentId | UUID | Yes | Document ID |
| maxLength | integer | No | Max summary length |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| summary | string | Document summary |
| keyPoints | array | Key points |
| wordCount | integer | Summary word count |

**Business Rules:**
- BR-20.18.1: Preserve key information
- BR-20.18.2: Multiple length options

**API Endpoint:** `POST /api/v1/ai/summarize`

**Authorization:** Owner, Tenant, AI System

---

### FR-20.19: Auto-generate Meeting Minutes

**Function:** `generateMeetingMinutes()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| meetingId | UUID | Yes | Meeting ID |
| transcript | string | Yes | Meeting transcript |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| minutes | string | Formatted minutes |
| attendees | array | Detected attendees |
| decisions | array | Key decisions |
| actionItems | array | Action items |

**Business Rules:**
- BR-20.19.1: Structure by agenda
- BR-20.19.2: Highlight decisions

**API Endpoint:** `POST /api/v1/ai/meeting-minutes`

**Authorization:** Manager, AI System

---

### FR-20.20: Spam/Abuse Detection

**Function:** `detectSpam()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| content | string | Yes | Content to check |
| contentType | enum | Yes | `message` \| `comment` |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| isSpam | boolean | Spam detection |
| isAbusive | boolean | Abuse detection |
| confidence | number | Detection confidence |
| flags | array | Specific flags triggered |

**Business Rules:**
- BR-20.20.1: Block confirmed spam
- BR-20.20.2: Flag for review if uncertain

**API Endpoint:** `POST /api/v1/ai/detect-spam`

**Authorization:** AI System

---

### FR-20.21: Recommend Similar Faults

**Function:** `findSimilarFaults()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| faultId | UUID | Yes | Current fault ID |
| limit | integer | No | Max results |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| similarFaults | array | Similar past faults |
| similarFaults[].id | UUID | Fault ID |
| similarFaults[].similarity | number | Similarity score |
| similarFaults[].resolution | string | How it was resolved |
| similarFaults[].resolutionTime | integer | Hours to resolve |

**Business Rules:**
- BR-20.21.1: Match by description and category
- BR-20.21.2: Prioritize resolved faults

**API Endpoint:** `GET /api/v1/ai/similar-faults`

**Authorization:** Manager, Technical Manager, AI System

---

### FR-20.22: Suggest Document Tags

**Function:** `suggestDocumentTags()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| documentId | UUID | Yes | Document ID |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| suggestedTags | array | Suggested tags |
| suggestedTags[].tag | string | Tag name |
| suggestedTags[].confidence | number | Suggestion confidence |

**Business Rules:**
- BR-20.22.1: Based on content analysis
- BR-20.22.2: Use existing tag vocabulary

**API Endpoint:** `POST /api/v1/ai/suggest-tags`

**Authorization:** Manager, AI System

---

### FR-20.23: Personalized News Feed

**Function:** `getPersonalizedFeed()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| userId | UUID | Yes | User ID |
| page | integer | No | Page number |
| limit | integer | No | Items per page |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| items | array | Curated content |
| items[].type | string | Content type |
| items[].content | object | Content item |
| items[].relevance | number | Personalization score |
| pagination | object | Pagination metadata |

**Business Rules:**
- BR-20.23.1: Learn from reading history
- BR-20.23.2: Balance new and relevant

**API Endpoint:** `GET /api/v1/ai/personalized-feed`

**Authorization:** Owner, Tenant, AI System

---

### FR-20.24: Smart Notification Prioritization

**Function:** `prioritizeNotifications()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| userId | UUID | Yes | User ID |
| notifications | array | Yes | Pending notifications |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| prioritized | array | Ordered notifications |
| prioritized[].id | UUID | Notification ID |
| prioritized[].priority | integer | Priority score |
| prioritized[].sendAt | datetime | Optimal send time |

**Business Rules:**
- BR-20.24.1: Learn user preferences
- BR-20.24.2: Consider time sensitivity

**API Endpoint:** `POST /api/v1/ai/prioritize-notifications`

**Authorization:** AI System

---

## FR-21: IoT & Smart Building

### FR-21.1: View Connected Device Status

**Function:** `listIoTDevices()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| buildingId | UUID | Yes | Building ID |
| deviceType | enum | No | Filter by type |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| items | array | Connected devices |
| items[].id | UUID | Device ID |
| items[].name | string | Device name |
| items[].type | string | Device type |
| items[].status | string | online/offline/error |
| items[].lastSeen | datetime | Last communication |

**Business Rules:**
- BR-21.1.1: Show only building's devices
- BR-21.1.2: Real-time status updates

**API Endpoint:** `GET /api/v1/buildings/{buildingId}/iot/devices`

**Authorization:** Manager

---

### FR-21.2-10: Additional IoT Functions

*Includes: Receive Sensor Alerts, View Smart Meter Data, Configure Device Automation, Control Smart Lock, View Energy Dashboard, Register IoT Device, View Access Log, Configure Alert Thresholds, Grant Guest IoT Access*

---

## FR-22: External Integrations

### FR-22.1-10: Integration Functions

*Includes: Sync Calendar Events, Generate Video Conference Link, Process E-Signatures, Send SMS Notification, Connect Payment Gateway, Submit to Government Portal, Import Bank Statement, Export to Accounting Software, Fetch Weather Alerts, Push to External CRM*

---

## FR-23: Security & Compliance

### FR-23.1: Enable Two-Factor Authentication

**Function:** `enableMfa()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| userId | UUID | Yes | User ID |
| method | enum | Yes | `totp` \| `sms` \| `email` |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| mfaEnabled | boolean | MFA status |
| setupData | object | Setup data |
| backupCodes | array | Recovery codes |

**Business Rules:**
- BR-23.1.1: Generate backup codes
- BR-23.1.2: Require verification to enable

**API Endpoint:** `POST /api/v1/auth/mfa/enable`

**Authorization:** Owner, Tenant, Manager

---

### FR-23.2-12: Additional Security Functions

*Includes: Configure Biometric Login, Export Personal Data (GDPR), Request Data Deletion (GDPR), View Consent History, Generate Security Report, View Active Sessions, Revoke Session, Configure IP Allowlist, View Data Processing Log, Set Up SSO, Verify MFA Code*

---

## FR-24: Community & Social

### FR-24.1-10: Community Functions

*Includes: Join Interest Group, Create Community Event, RSVP to Event, Post to Community Board, Rate Service Provider, Book Shared Amenity, Offer Neighbor Help, Request Neighbor Help, Marketplace Listing, View Community Leaderboard*

---

## FR-25: Accessibility

### FR-25.1-8: Accessibility Functions

*Includes: Configure Screen Reader Support, Enable Keyboard Navigation, Set Voice Command Preferences, Request Alternative Format, Set Color Blind Mode, Generate Alt Text, Text-to-Speech for Content, WCAG Compliance Check*

---

## FR-26: Workflow Automation

### FR-26.1-10: Workflow Functions

*Includes: Create Approval Workflow, Auto-assign Fault Handler, Schedule Recurring Tasks, Set Auto-Reminders, Trigger Webhook, Automate Document Generation, Set Up Escalation Rules, Batch Process Operations, Configure Notification Rules, View Workflow History*

---

## FR-28: Delegation & Permissions

### FR-28.1-10: Delegation Functions

*Includes: Create Permission Delegation, Revoke Delegation, View Active Delegations, Extend Delegation Period, Set Owner Delegate, Create Custom Role, Assign Role to User, View Permission Matrix, Request Permission, Approve Permission Request*

---

## FR-29: Short-term Rental Management

### FR-29.1: Connect Airbnb/Booking Account

**Function:** `connectRentalPlatform()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | Yes | Organization tenant ID |
| unitId | UUID | Yes | Unit ID |
| platform | enum | Yes | `airbnb` \| `booking` \| `vrbo` |
| credentials | object | Yes | OAuth credentials |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| connectionId | UUID | Connection ID |
| listings | array | Discovered listings |
| status | string | Connection status |

**Business Rules:**
- BR-29.1.1: OAuth flow
- BR-29.1.2: Discover existing listings

**API Endpoint:** `POST /api/v1/rentals/connect`

**Authorization:** Owner, Property Manager

---

### FR-29.2-15: Additional Rental Functions

*Includes: Sync Booking Calendar, View Upcoming Reservations, Notify Management of Booking, Generate Check-in Instructions, Set Rental Pricing, Block Dates, Track Revenue, Manage Cleaning Schedule, Handle Guest Issue, Coordinate Key Exchange, Report Noise Violation, Send Guest Message, Generate House Rules, Rate Guest*

---

## FR-30: Guest Registration System

### FR-30.1-10: Guest Registration Functions

*Includes: Register Guest, Submit Police Registration, Scan Guest ID, View Guest Log, Update Guest Information, Check Out Guest, Generate Guest Access Code, Revoke Guest Access, Export Guest Register, View Registration History*

---

## FR-31: Real Estate & Listings

### FR-31.1-14: Real Estate Functions

*Includes: Create Property Listing, Upload Listing Photos, Publish Listing, Edit Listing Details, Set Listing Price, Archive Listing, View Listing Analytics, Mark as Sold/Rented, Schedule Viewing, Receive Inquiries, Respond to Inquiry, Feature Listing, Share Listing, Duplicate Listing*

---

## FR-32: Real Estate Portal Integration

### FR-32.1-10: Portal Integration Functions

*Includes: Connect to Portal, Sync Listings, Map Portal Fields, View Sync Status, Resolve Conflicts, Configure Auto-Publish, Export to Portal, Import from Portal, Disconnect Portal, View Portal Analytics*

---

## FR-33: Tenant Screening

### FR-33.1-12: Screening Functions

*Includes: Request Background Check, Verify Income, Check Credit Score, Verify Employment, Contact References, View Screening Report, Share Report with Landlord, Request Additional Documents, Approve Applicant, Reject Applicant, Add Screening Notes, Generate Screening Summary*

---

## FR-34: Lease Management

### FR-34.1-10: Lease Functions

*Includes: Create Lease Agreement, Send for Signature, Track Signing Progress, Store Signed Lease, Set Renewal Reminder, Renew Lease, Terminate Lease, Amendment Lease, View Lease History, Export Lease Document*

---

## FR-35: Insurance Management

### FR-35.1-8: Insurance Functions

*Includes: Add Insurance Policy, View Policy Details, Set Renewal Reminder, File Insurance Claim, Track Claim Status, Upload Claim Documents, View Claim History, Generate Insurance Report*

---

## FR-36: Maintenance Scheduling

### FR-36.1-8: Maintenance Functions

*Includes: Create Maintenance Schedule, View Upcoming Maintenance, Assign Technician, Complete Maintenance, Record Maintenance Log, Set Recurring Maintenance, View Maintenance History, Generate Maintenance Report*

---

## FR-37: Supplier/Vendor Management

### FR-37.1-8: Vendor Functions

*Includes: Add Vendor, View Vendor Directory, Rate Vendor, Request Quote, Accept Quote, Track Work Order, Process Payment, View Vendor History*

---

## FR-38: Legal & Compliance

### FR-38.1-8: Legal Functions

*Includes: Store Legal Document, Set Compliance Deadline, Track Regulatory Requirements, Generate Compliance Report, Schedule Legal Review, Log Compliance Action, View Compliance History, Export Compliance Data*

---

## FR-39: Emergency Management

### FR-39.1-8: Emergency Functions

*Includes: Declare Emergency, Broadcast Emergency Alert, View Emergency Contacts, Activate Emergency Plan, Log Emergency Response, Coordinate with Authorities, Close Emergency, Generate Incident Report*

---

## FR-40: Budget & Planning

### FR-40.1-8: Budget Functions

*Includes: Create Annual Budget, Allocate Funds, Track Expenses, Compare Budget vs Actual, Approve Budget Item, Request Budget Amendment, Generate Budget Report, Forecast Future Costs*

---

## FR-41: Subscription & Billing

### FR-41.1: View Subscription Plans

**Function:** `listSubscriptionPlans()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| tenantId | UUID | No | Current tenant |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| plans | array | Available plans |
| plans[].id | UUID | Plan ID |
| plans[].name | string | Plan name |
| plans[].price | number | Monthly price |
| plans[].features | array | Included features |

**Business Rules:**
- BR-41.1.1: Show all public plans
- BR-41.1.2: Highlight current plan

**API Endpoint:** `GET /api/v1/subscriptions/plans`

**Authorization:** Public

---

### FR-41.2-11: Additional Subscription Functions

*Includes: Subscribe to Plan, Upgrade Subscription, Downgrade Subscription, Cancel Subscription, View Billing History, Download Invoice, Update Payment Method, Apply Coupon Code, Handle Failed Payment, Reactivate Subscription*

---

## FR-42: Onboarding & Help

### FR-42.1-8: Onboarding Functions

*Includes: View Onboarding Progress, Complete Setup Step, Skip Optional Step, Request Demo, View Help Articles, Search Knowledge Base, Contact Support, Submit Feedback*

---

## FR-43: Mobile App Features

### FR-43.1-8: Mobile Functions

*Includes: Enable Push Notifications, Configure Notification Preferences, Use Biometric Login, Download Offline Data, Scan QR Code, Use Camera for OCR, View Widget Data, Share via Mobile*

---

## FR-44: Favorites Management (Reality Portal)

### FR-44.1-6: Favorites Functions

*Includes: Add to Favorites, Remove from Favorites, View Favorites List, Organize Favorites, Share Favorites, Get Price Alerts*

---

## FR-45: Saved Searches & Alerts (Reality Portal)

### FR-45.1-8: Search Alert Functions

*Includes: Save Search, Edit Saved Search, Delete Saved Search, Enable Search Alerts, Configure Alert Frequency, View New Matches, Pause Alerts, View Search History*

---

## FR-46: Contact Inquiries (Reality Portal)

### FR-46.1-6: Inquiry Functions

*Includes: Send Property Inquiry, View Sent Inquiries, Receive Inquiry Response, Schedule Property Viewing, Cancel Viewing, Rate Realtor*

---

## FR-47: Portal User Accounts (Reality Portal)

### FR-47.1: Register Portal Account

**Function:** `registerPortalUser()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| email | string | Yes | Email address |
| password | string | Yes | Password |
| firstName | string | Yes | First name |
| lastName | string | Yes | Last name |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| userId | UUID | Created user ID |
| verificationSent | boolean | Email sent |

**Business Rules:**
- BR-47.1.1: Email verification required
- BR-47.1.2: Password strength validation

**API Endpoint:** `POST /api/v1/portal/register`

**Authorization:** Public

---

### FR-47.2-15: Additional Portal Account Functions

*Includes: Login with Password, Login with Social (Google/Apple/Facebook), Verify Email, Reset Password, Edit Profile, Delete Portal Account, Manage Linked Social Accounts, Register as Realtor, Create Reality Agency, Invite Realtors to Agency, Accept Agency Invitation, Manage Agency Realtors, View Activity History, Configure Privacy Settings*

---

## FR-48: Property Comparison (Reality Portal)

### FR-48.1-5: Comparison Functions

*Includes: Add to Comparison, View Comparison, Remove from Comparison, Share Comparison, Export Comparison*

---

## FR-49: Agency Management (Reality Portal)

### FR-49.1-10: Agency Functions

*Includes: View Agency Dashboard, Edit Agency Profile, Set Agency Branding, View Agency Listings, View Agency Inquiries, Assign Inquiry to Realtor, View Realtor Performance, Suspend Realtor, Remove Realtor from Agency, Configure Agency Settings*

---

## FR-50: Property Import (Reality Portal)

### FR-50.1-10: Import Functions

*Includes: Connect External CRM, Import Properties from CRM, Map CRM Fields, Schedule Automatic Sync, View Import History, Resolve Import Conflicts, Import from XML Feed, Import from CSV File, Import from IDX/RETS, Export Properties*

---

## FR-51: Realtor Profile & Listings (Reality Portal)

### FR-51.1: Create Realtor Profile

**Function:** `createRealtorProfile()`

**Inputs:**
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| userId | UUID | Yes | User ID |
| bio | string | Yes | Professional bio |
| photo | file | Yes | Profile photo |
| licenseNumber | string | Yes | License number |
| specializations | array | No | Specialties |

**Outputs:**
| Field | Type | Description |
|-------|------|-------------|
| profileId | UUID | Profile ID |
| verificationStatus | string | License status |

**Business Rules:**
- BR-51.1.1: Verify license
- BR-51.1.2: Require photo

**API Endpoint:** `POST /api/v1/portal/realtor-profile`

**Authorization:** Realtor

---

### FR-51.2-12: Additional Realtor Functions

*Includes: Add License Information, View My Listings, Create New Listing, Edit Listing, Upload Listing Photos, Set Listing Status, View My Inquiries, Respond to Inquiry, View Listing Analytics, Feature Listing, Share Listing*

---

## Summary

This document defines functional requirements for all 508 use cases across 51 categories:

| Category Range | Domain |
|----------------|--------|
| FR-01 to FR-18 | Core Property Management |
| FR-19 to FR-26 | Modern Technology |
| FR-27 to FR-34 | Multi-tenancy & Rental |
| FR-35 to FR-43 | Operations & Support |
| FR-44 to FR-51 | Reality Portal |

Each requirement includes:
- Function name and signature
- Input parameters with types and validation
- Output data structures
- Business rules and constraints
- API endpoint mapping
- Authorization requirements

---

## References

- Use Cases: `docs/use-cases.md`
- Edge Cases: `docs/validation/edge-cases.md`
- API Specification: `docs/api/`
- Architecture: Root `CLAUDE.md`
