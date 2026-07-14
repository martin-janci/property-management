# Story 84.2: E-Signature Email Integration

Status: done

## Story

As a **property manager**,
I want to **send signature requests via email**,
So that **signers receive notifications and can access documents to sign**.

## Acceptance Criteria

1. **AC-1: Signature Request Email**
   - Given a document requires signatures
   - When I send it for signing
   - Then each signer receives an email
   - And the email contains a secure signing link
   - And the email is properly branded

2. **AC-2: Reminder Emails**
   - Given a signature is pending
   - When the reminder interval passes
   - Then a reminder email is sent
   - And it includes the original signing link
   - And the urgency increases with each reminder

3. **AC-3: Completion Notification**
   - Given all parties have signed
   - When the document is complete
   - Then all parties receive confirmation
   - And a link to the signed document is included

4. **AC-4: Decline Notification**
   - Given a signer declines to sign
   - When the decline is recorded
   - Then the document owner is notified
   - And the reason is included if provided

5. **AC-5: Email Tracking**
   - Given a signature email is sent
   - When delivery events occur
   - Then open and click events are tracked
   - And delivery failures are handled

## Tasks / Subtasks

- [x] Task 1: Create Signature Email Templates (AC: 1, 2, 3, 4)
  - [x] 1.1 Create signature request template
  - [x] 1.2 Create reminder template (levels 1, 2, 3)
  - [x] 1.3 Create completion template
  - [x] 1.4 Create decline notification template
  - [x] 1.5 Add branding and styling

- [x] Task 2: Implement Signature Request Sending (AC: 1)
  - [x] 2.1 Update `/backend/servers/api-server/src/routes/signatures.rs:120`
  - [x] 2.2 Generate secure signing token
  - [x] 2.3 Create signing URL
  - [x] 2.4 Send email via email service
  - [x] 2.5 Record email sent status

- [x] Task 3: Implement Reminder System (AC: 2)
  - [x] 3.1 Update `/backend/servers/api-server/src/routes/signatures.rs:267`
  - [x] 3.2 Create reminder scheduling job
  - [x] 3.3 Track reminder count per signer
  - [x] 3.4 Escalate reminder urgency
  - [x] 3.5 Respect business hours

- [x] Task 4: Implement Completion Emails (AC: 3)
  - [x] 4.1 Update `/backend/servers/api-server/src/routes/signatures.rs:316`
  - [x] 4.2 Detect document completion
  - [x] 4.3 Generate signed document PDF
  - [x] 4.4 Send to all parties
  - [x] 4.5 Include audit trail

- [x] Task 5: Implement Decline Handling (AC: 4)
  - [x] 5.1 Add decline reason capture
  - [x] 5.2 Send decline notification
  - [x] 5.3 Mark document as declined
  - [x] 5.4 Allow resubmission

- [x] Task 6: Add Email Tracking (AC: 5)
  - [x] 6.1 Configure webhook for email events
  - [x] 6.2 Track email opens
  - [x] 6.3 Track link clicks
  - [x] 6.4 Handle bounces
  - [x] 6.5 Store events for audit

## Dev Notes

### Architecture Requirements
- Secure signing tokens with expiration
- Branded email templates
- Automatic reminder scheduling
- Email event tracking

### Technical Specifications
- Email service: AWS SES or SendGrid
- Token expiration: 7 days (configurable)
- Reminder schedule: Days 3, 5, 7 after initial send
- Template engine: Handlebars

### Existing TODO References
```rust
// backend/servers/api-server/src/routes/signatures.rs:120
// TODO: Send signature request email
// - Generate secure token
// - Create signing URL
// - Send branded email

// backend/servers/api-server/src/routes/signatures.rs:267
// TODO: Implement reminder emails
// - Schedule reminders
// - Escalate urgency

// backend/servers/api-server/src/routes/signatures.rs:316
// TODO: Send completion notification
// - All parties signed
// - Include signed document
```

### Email Templates
```html
<!-- signature_request.html -->
<!DOCTYPE html>
<html>
<head>
    <style>
        .container { max-width: 600px; margin: 0 auto; font-family: Arial, sans-serif; }
        .header { background: #2563eb; color: white; padding: 20px; }
        .content { padding: 20px; }
        .button {
            display: inline-block;
            background: #2563eb;
            color: white;
            padding: 12px 24px;
            text-decoration: none;
            border-radius: 4px;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>{{organization_name}}</h1>
        </div>
        <div class="content">
            <h2>You have a document to sign</h2>
            <p>{{sender_name}} has sent you "{{document_name}}" for your signature.</p>
            <p>Please review and sign the document by clicking the button below:</p>
            <p><a href="{{signing_url}}" class="button">Review & Sign</a></p>
            <p>This link expires in {{expires_in_days}} days.</p>
            <p>If you have questions, contact {{sender_email}}.</p>
        </div>
    </div>
</body>
</html>
```

### Signing Token Generation
```rust
#[derive(Serialize, Deserialize)]
struct SigningToken {
    signature_request_id: Uuid,
    signer_email: String,
    expires_at: DateTime<Utc>,
}

impl SigningToken {
    fn generate(request_id: Uuid, email: &str, ttl: Duration) -> String {
        let token = Self {
            signature_request_id: request_id,
            signer_email: email.to_string(),
            expires_at: Utc::now() + ttl,
        };

        // Encrypt and base64 encode
        encrypt_and_encode(&token)
    }

    fn verify(token_str: &str) -> Result<Self, TokenError> {
        let token: Self = decode_and_decrypt(token_str)?;

        if token.expires_at < Utc::now() {
            return Err(TokenError::Expired);
        }

        Ok(token)
    }
}
```

### Email Service
```rust
pub struct EmailService {
    client: SesClient, // or SendGridClient
    from_email: String,
    templates: TemplateEngine,
}

impl EmailService {
    pub async fn send_signature_request(
        &self,
        signer_email: &str,
        data: SignatureRequestEmail,
    ) -> Result<String, EmailError> {
        let html = self.templates.render("signature_request", &data)?;

        let email_id = self.client
            .send_email()
            .destination(signer_email)
            .message(Message::builder()
                .subject("Please sign: {}", &data.document_name)
                .body(Body::Html(html))
                .build())
            .from_email_address(&self.from_email)
            .send()
            .await?;

        Ok(email_id)
    }
}
```

### Reminder Scheduler
```rust
pub struct SignatureReminderJob;

impl SignatureReminderJob {
    pub async fn run(&self, state: &AppState) -> Result<(), JobError> {
        let pending = state.signature_repo
            .find_pending_signatures_needing_reminder()
            .await?;

        for signature in pending {
            let reminder_level = signature.reminder_count + 1;

            if reminder_level <= 3 {
                state.email_service.send_signature_reminder(
                    &signature.signer_email,
                    SignatureReminderEmail {
                        level: reminder_level,
                        document_name: signature.document_name.clone(),
                        signing_url: generate_signing_url(&signature),
                        sender_name: signature.sender_name.clone(),
                    },
                ).await?;

                state.signature_repo
                    .increment_reminder_count(signature.id)
                    .await?;
            }
        }

        Ok(())
    }
}
```

### File List (to create/modify)

**Create:**
- `/backend/crates/common/src/email/templates/signature_request.html`
- `/backend/crates/common/src/email/templates/signature_reminder.html`
- `/backend/crates/common/src/email/templates/signature_complete.html`
- `/backend/crates/common/src/email/templates/signature_declined.html`
- `/backend/servers/api-server/src/services/signature_email.rs`
- `/backend/servers/api-server/src/jobs/signature_reminder.rs`

**Modify:**
- `/backend/servers/api-server/src/routes/signatures.rs` - Wire email sending
- `/backend/servers/api-server/src/services/mod.rs` - Export module
- `/backend/servers/api-server/src/jobs/mod.rs` - Register job

### Email Event Tracking
```rust
async fn handle_email_event(
    State(state): State<AppState>,
    Json(event): Json<EmailEvent>,
) -> Result<StatusCode, ApiError> {
    match event.event_type.as_str() {
        "open" => {
            state.signature_repo
                .record_email_opened(event.metadata.signature_request_id)
                .await?;
        }
        "click" => {
            state.signature_repo
                .record_link_clicked(event.metadata.signature_request_id)
                .await?;
        }
        "bounce" | "complaint" => {
            state.signature_repo
                .record_delivery_failure(
                    event.metadata.signature_request_id,
                    &event.event_type,
                )
                .await?;
        }
        _ => {}
    }

    Ok(StatusCode::OK)
}
```

### Dependencies
- Story 84.1 (S3 Presigned URLs) - For document access
- Epic 2B (Notifications) - Email service infrastructure

### References
- [Source: backend/servers/api-server/src/routes/signatures.rs:120,267,316]
- [UC-17: E-Signatures]
