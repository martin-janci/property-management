# Story 84.4: Notification Trigger System

Status: done

## Story

As a **property manager or resident**,
I want to **receive notifications for relevant events automatically**,
So that **I stay informed about important announcements and updates**.

## Acceptance Criteria

1. **AC-1: Announcement Notification**
   - Given an announcement is published
   - When it matches my targeting criteria
   - Then I receive a notification
   - And I can tap to view the announcement

2. **AC-2: Fault Status Notification**
   - Given I reported a fault
   - When the status changes
   - Then I receive a notification
   - And the new status is shown

3. **AC-3: Vote Notification**
   - Given a new vote is created
   - When I am eligible to participate
   - Then I receive a notification
   - And I can tap to cast my vote

4. **AC-4: Notification Preferences**
   - Given I want to control notifications
   - When I configure preferences
   - Then I can enable/disable by category
   - And I can choose channels (push, email, in-app)

5. **AC-5: Notification Delivery**
   - Given a notification is triggered
   - When sent to the user
   - Then it appears in all enabled channels
   - And it is stored in notification history

## Tasks / Subtasks

- [ ] Task 1: Create Notification Event System (AC: 1, 2, 3)
  - [ ] 1.1 Create notification event types enum
  - [ ] 1.2 Create event publisher trait
  - [ ] 1.3 Implement event dispatcher
  - [ ] 1.4 Add event handlers registry

- [ ] Task 2: Implement Announcement Triggers (AC: 1)
  - [ ] 2.1 Update `/backend/servers/api-server/src/routes/announcements.rs:827`
  - [ ] 2.2 Trigger on announcement publish
  - [ ] 2.3 Resolve target audience
  - [ ] 2.4 Send to all targeted users

- [ ] Task 3: Implement Fault Triggers (AC: 2)
  - [ ] 3.1 Trigger on fault status change
  - [ ] 3.2 Notify fault reporter
  - [ ] 3.3 Notify assigned technician
  - [ ] 3.4 Include status in notification

- [ ] Task 4: Implement Vote Triggers (AC: 3)
  - [ ] 4.1 Trigger on vote creation
  - [ ] 4.2 Resolve eligible voters
  - [ ] 4.3 Send voting notification
  - [ ] 4.4 Send reminder before deadline

- [ ] Task 5: Create Notification Preferences System (AC: 4)
  - [ ] 5.1 Create preferences table
  - [ ] 5.2 Default preferences per user
  - [ ] 5.3 Preferences API endpoints
  - [ ] 5.4 Preference checking in dispatch

- [ ] Task 6: Implement Multi-channel Delivery (AC: 5)
  - [ ] 6.1 Push notification delivery
  - [ ] 6.2 Email notification delivery
  - [ ] 6.3 In-app notification storage
  - [ ] 6.4 Notification history API

## Dev Notes

### Architecture Requirements
- Event-driven notification system
- Multi-channel delivery (push, email, in-app)
- User preference respecting
- Notification history storage

### Technical Specifications
- Push: Firebase Cloud Messaging (FCM) / APNs
- Email: AWS SES / SendGrid
- In-app: WebSocket real-time + REST history
- Batch size: 1000 recipients per batch

### Existing TODO Reference
```rust
// backend/servers/api-server/src/routes/announcements.rs:827
// TODO: Trigger notifications on announcement publish
// - Resolve target audience from targeting criteria
// - Send push/email based on preferences
// - Store in notification history
```

### Notification Event Types
```rust
#[derive(Clone, Debug)]
pub enum NotificationEvent {
    AnnouncementPublished {
        announcement_id: Uuid,
        organization_id: Uuid,
        target_type: TargetType,
        target_ids: Vec<Uuid>,
        title: String,
    },
    FaultStatusChanged {
        fault_id: Uuid,
        reporter_id: Uuid,
        technician_id: Option<Uuid>,
        old_status: FaultStatus,
        new_status: FaultStatus,
    },
    VoteCreated {
        vote_id: Uuid,
        organization_id: Uuid,
        building_id: Option<Uuid>,
        title: String,
        deadline: DateTime<Utc>,
    },
    VoteReminder {
        vote_id: Uuid,
        deadline: DateTime<Utc>,
    },
    MessageReceived {
        message_id: Uuid,
        recipient_id: Uuid,
        sender_name: String,
    },
}
```

### Notification Dispatcher
```rust
pub struct NotificationDispatcher {
    push_service: Arc<PushNotificationService>,
    email_service: Arc<EmailService>,
    in_app_repo: Arc<NotificationRepository>,
    preferences_repo: Arc<PreferencesRepository>,
}

impl NotificationDispatcher {
    pub async fn dispatch(&self, event: NotificationEvent) -> Result<(), NotificationError> {
        let recipients = self.resolve_recipients(&event).await?;

        for chunk in recipients.chunks(1000) {
            let notifications: Vec<_> = chunk.iter()
                .filter_map(|user_id| {
                    self.build_notification(&event, *user_id).ok()
                })
                .collect();

            // Send in parallel to all channels
            let (push_results, email_results, in_app_results) = tokio::join!(
                self.send_push_batch(&notifications),
                self.send_email_batch(&notifications),
                self.store_in_app_batch(&notifications),
            );

            // Log any failures
            self.log_failures(&push_results, &email_results, &in_app_results).await;
        }

        Ok(())
    }

    async fn resolve_recipients(&self, event: &NotificationEvent) -> Result<Vec<Uuid>, NotificationError> {
        match event {
            NotificationEvent::AnnouncementPublished { target_type, target_ids, organization_id, .. } => {
                self.resolve_announcement_targets(*organization_id, target_type, target_ids).await
            }
            NotificationEvent::FaultStatusChanged { reporter_id, technician_id, .. } => {
                let mut recipients = vec![*reporter_id];
                if let Some(tech) = technician_id {
                    recipients.push(*tech);
                }
                Ok(recipients)
            }
            NotificationEvent::VoteCreated { organization_id, building_id, .. } => {
                self.resolve_vote_eligible_voters(*organization_id, *building_id).await
            }
            // ... other event types
        }
    }
}
```

### Notification Preferences Model
```sql
CREATE TABLE notification_preferences (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    category VARCHAR(50) NOT NULL, -- 'announcements', 'faults', 'votes', 'messages'
    push_enabled BOOLEAN NOT NULL DEFAULT true,
    email_enabled BOOLEAN NOT NULL DEFAULT true,
    in_app_enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, category)
);

CREATE TABLE notifications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    category VARCHAR(50) NOT NULL,
    title VARCHAR(255) NOT NULL,
    body TEXT NOT NULL,
    data JSONB,
    read_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_notifications_user ON notifications(user_id);
CREATE INDEX idx_notifications_unread ON notifications(user_id, read_at) WHERE read_at IS NULL;
```

### File List (to create/modify)

**Create:**
- `/backend/crates/common/src/notifications/mod.rs` - Notification types
- `/backend/crates/common/src/notifications/events.rs` - Event definitions
- `/backend/crates/common/src/notifications/dispatcher.rs` - Dispatcher
- `/backend/crates/db/src/repositories/notification.rs` - Repository
- `/backend/crates/db/src/repositories/notification_preferences.rs`
- `/backend/crates/db/migrations/NNNN_create_notifications.sql`
- `/backend/servers/api-server/src/routes/notifications.rs` - API

**Modify:**
- `/backend/servers/api-server/src/routes/announcements.rs` - Add trigger
- `/backend/servers/api-server/src/routes/faults.rs` - Add trigger
- `/backend/servers/api-server/src/routes/voting.rs` - Add trigger
- `/backend/servers/api-server/src/routes/mod.rs` - Export module

### Push Notification Payload
```rust
struct PushNotification {
    token: String,
    title: String,
    body: String,
    data: HashMap<String, String>,
    platform: Platform, // iOS, Android, Web
}

impl PushNotificationService {
    async fn send_batch(&self, notifications: &[PushNotification]) -> Vec<Result<(), PushError>> {
        // Group by platform
        let ios: Vec<_> = notifications.iter().filter(|n| n.platform == Platform::iOS).collect();
        let android: Vec<_> = notifications.iter().filter(|n| n.platform == Platform::Android).collect();

        let (ios_results, android_results) = tokio::join!(
            self.send_apns_batch(&ios),
            self.send_fcm_batch(&android),
        );

        // Combine results
        ios_results.into_iter().chain(android_results).collect()
    }
}
```

### Dependencies
- Epic 2B (Notification Infrastructure) - Push/email services
- Story 79.4 (WebSocket) - Real-time in-app notifications

### References
- [Source: backend/servers/api-server/src/routes/announcements.rs:827]
- [UC-23: Notifications]
