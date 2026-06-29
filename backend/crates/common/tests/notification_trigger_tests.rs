//! Trigger-system regression tests for Story 84.4 (Notification Trigger System).
//!
//! Coverage gap 84-4-notification-triggers: the `NotificationEvent` trigger
//! mapping (the logic that turns a domain event into a notification's
//! category / priority / title / body / deep-link) had only a handful of
//! inline unit tests and no dedicated trigger-specific test file. These
//! integration tests exhaustively pin the mapping for every event variant so
//! a regression in the trigger logic fails CI.

use chrono::{TimeZone, Utc};
use common::notifications::{
    events::{FaultStatus, NotificationEvent, TargetType},
    NotificationCategory, NotificationPriority,
};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Builders — one per variant, kept terse so the assertions stay readable.
// ---------------------------------------------------------------------------

fn announcement() -> NotificationEvent {
    NotificationEvent::AnnouncementPublished {
        announcement_id: Uuid::new_v4(),
        organization_id: Uuid::new_v4(),
        target_type: TargetType::Building,
        target_ids: vec![Uuid::new_v4()],
        title: "Lobby closure".to_string(),
    }
}

fn fault_status(old: FaultStatus, new: FaultStatus) -> NotificationEvent {
    NotificationEvent::FaultStatusChanged {
        fault_id: Uuid::new_v4(),
        reporter_id: Uuid::new_v4(),
        technician_id: Some(Uuid::new_v4()),
        old_status: old,
        new_status: new,
        title: "Leaking radiator".to_string(),
    }
}

fn fault_assigned() -> NotificationEvent {
    NotificationEvent::FaultAssigned {
        fault_id: Uuid::new_v4(),
        reporter_id: Uuid::new_v4(),
        technician_id: Uuid::new_v4(),
        title: "Broken elevator".to_string(),
    }
}

fn vote_created() -> NotificationEvent {
    NotificationEvent::VoteCreated {
        vote_id: Uuid::new_v4(),
        organization_id: Uuid::new_v4(),
        building_id: None,
        title: "Annual budget".to_string(),
        deadline: Utc.with_ymd_and_hms(2026, 7, 1, 12, 0, 0).unwrap(),
    }
}

fn vote_reminder(hours_remaining: i32) -> NotificationEvent {
    NotificationEvent::VoteReminder {
        vote_id: Uuid::new_v4(),
        title: "Annual budget".to_string(),
        deadline: Utc.with_ymd_and_hms(2026, 7, 1, 12, 0, 0).unwrap(),
        hours_remaining,
    }
}

fn message() -> NotificationEvent {
    NotificationEvent::MessageReceived {
        message_id: Uuid::new_v4(),
        recipient_id: Uuid::new_v4(),
        sender_name: "Jana".to_string(),
        preview: "Hi, about the lease...".to_string(),
    }
}

fn signature_requested() -> NotificationEvent {
    NotificationEvent::SignatureRequested {
        request_id: Uuid::new_v4(),
        document_name: "Lease 2026".to_string(),
        sender_name: "Owner".to_string(),
        expires_at: Utc.with_ymd_and_hms(2026, 8, 1, 0, 0, 0).unwrap(),
    }
}

fn signature_reminder(reminder_level: i32, days_remaining: i32) -> NotificationEvent {
    NotificationEvent::SignatureReminder {
        request_id: Uuid::new_v4(),
        document_name: "Lease 2026".to_string(),
        reminder_level,
        days_remaining,
    }
}

fn signature_completed() -> NotificationEvent {
    NotificationEvent::SignatureCompleted {
        request_id: Uuid::new_v4(),
        document_name: "Lease 2026".to_string(),
    }
}

fn payment_due() -> NotificationEvent {
    NotificationEvent::PaymentDue {
        invoice_id: Uuid::new_v4(),
        amount: "120.00 EUR".to_string(),
        due_date: Utc.with_ymd_and_hms(2026, 7, 15, 0, 0, 0).unwrap(),
    }
}

fn payment_received() -> NotificationEvent {
    NotificationEvent::PaymentReceived {
        payment_id: Uuid::new_v4(),
        amount: "120.00 EUR".to_string(),
    }
}

fn emergency() -> NotificationEvent {
    NotificationEvent::EmergencyAlert {
        emergency_id: Uuid::new_v4(),
        building_id: Some(Uuid::new_v4()),
        title: "Gas leak".to_string(),
        severity: "critical".to_string(),
    }
}

fn community_event() -> NotificationEvent {
    NotificationEvent::CommunityEvent {
        event_id: Uuid::new_v4(),
        title: "Summer BBQ".to_string(),
        event_date: Utc.with_ymd_and_hms(2026, 7, 20, 17, 0, 0).unwrap(),
    }
}

/// All thirteen variants, used by the exhaustiveness sweeps below.
fn all_variants() -> Vec<NotificationEvent> {
    vec![
        announcement(),
        fault_status(FaultStatus::Reported, FaultStatus::Triaged),
        fault_assigned(),
        vote_created(),
        vote_reminder(48),
        message(),
        signature_requested(),
        signature_reminder(1, 5),
        signature_completed(),
        payment_due(),
        payment_received(),
        emergency(),
        community_event(),
    ]
}

// ---------------------------------------------------------------------------
// category() — every variant maps to its documented category.
// ---------------------------------------------------------------------------

#[test]
fn category_mapping_is_exhaustive_and_correct() {
    assert_eq!(
        announcement().category(),
        NotificationCategory::Announcements
    );
    assert_eq!(
        fault_status(FaultStatus::Reported, FaultStatus::Triaged).category(),
        NotificationCategory::Faults
    );
    assert_eq!(fault_assigned().category(), NotificationCategory::Faults);
    assert_eq!(vote_created().category(), NotificationCategory::Votes);
    assert_eq!(vote_reminder(12).category(), NotificationCategory::Votes);
    assert_eq!(message().category(), NotificationCategory::Messages);
    // Signature events route to Financial in the current mapping.
    assert_eq!(
        signature_requested().category(),
        NotificationCategory::Financial
    );
    assert_eq!(
        signature_reminder(1, 5).category(),
        NotificationCategory::Financial
    );
    assert_eq!(
        signature_completed().category(),
        NotificationCategory::Financial
    );
    assert_eq!(payment_due().category(), NotificationCategory::Financial);
    assert_eq!(
        payment_received().category(),
        NotificationCategory::Financial
    );
    assert_eq!(emergency().category(), NotificationCategory::System);
    assert_eq!(
        community_event().category(),
        NotificationCategory::Community
    );
}

// ---------------------------------------------------------------------------
// priority() — the conditional branches are the regression-prone bits.
// ---------------------------------------------------------------------------

#[test]
fn emergency_alert_is_always_urgent() {
    assert_eq!(emergency().priority(), NotificationPriority::Urgent);
}

#[test]
fn resolved_fault_is_high_other_statuses_normal() {
    assert_eq!(
        fault_status(FaultStatus::InProgress, FaultStatus::Resolved).priority(),
        NotificationPriority::High
    );
    // A non-resolved transition stays Normal.
    assert_eq!(
        fault_status(FaultStatus::Reported, FaultStatus::Triaged).priority(),
        NotificationPriority::Normal
    );
    assert_eq!(
        fault_status(FaultStatus::Resolved, FaultStatus::Closed).priority(),
        NotificationPriority::Normal
    );
}

#[test]
fn payment_due_is_high() {
    assert_eq!(payment_due().priority(), NotificationPriority::High);
    // Receipt is informational, not high.
    assert_eq!(payment_received().priority(), NotificationPriority::Normal);
}

#[test]
fn vote_reminder_priority_is_time_sensitive() {
    // <= 24h remaining escalates to High.
    assert_eq!(vote_reminder(24).priority(), NotificationPriority::High);
    assert_eq!(vote_reminder(1).priority(), NotificationPriority::High);
    // > 24h stays Normal.
    assert_eq!(vote_reminder(25).priority(), NotificationPriority::Normal);
    assert_eq!(vote_reminder(48).priority(), NotificationPriority::Normal);
}

#[test]
fn signature_reminder_priority_escalates_at_level_three() {
    assert_eq!(
        signature_reminder(1, 5).priority(),
        NotificationPriority::Normal
    );
    assert_eq!(
        signature_reminder(2, 3).priority(),
        NotificationPriority::Normal
    );
    assert_eq!(
        signature_reminder(3, 1).priority(),
        NotificationPriority::High
    );
    assert_eq!(
        signature_reminder(4, 0).priority(),
        NotificationPriority::High
    );
}

#[test]
fn default_priority_is_normal_for_informational_events() {
    assert_eq!(announcement().priority(), NotificationPriority::Normal);
    assert_eq!(fault_assigned().priority(), NotificationPriority::Normal);
    assert_eq!(vote_created().priority(), NotificationPriority::Normal);
    assert_eq!(message().priority(), NotificationPriority::Normal);
    assert_eq!(
        signature_requested().priority(),
        NotificationPriority::Normal
    );
    assert_eq!(
        signature_completed().priority(),
        NotificationPriority::Normal
    );
    assert_eq!(community_event().priority(), NotificationPriority::Normal);
}

// ---------------------------------------------------------------------------
// title() / body() — copy generation. Pin the prefixes and dynamic inserts.
// ---------------------------------------------------------------------------

#[test]
fn every_variant_produces_nonempty_title_and_body() {
    for event in all_variants() {
        assert!(!event.title().is_empty(), "empty title for {event:?}");
        assert!(!event.body().is_empty(), "empty body for {event:?}");
    }
}

#[test]
fn titles_embed_the_relevant_subject() {
    assert!(announcement().title().contains("Lobby closure"));
    assert!(fault_assigned().title().starts_with("Fault Assigned:"));
    assert!(vote_created().title().starts_with("New Vote:"));
    assert!(message().title().contains("Jana"));
    assert!(emergency().title().starts_with("EMERGENCY:"));
    // Fault status title uses the human-readable label of the new status.
    assert!(fault_status(FaultStatus::Reported, FaultStatus::InProgress)
        .title()
        .contains("In Progress"));
}

#[test]
fn signature_reminder_body_varies_by_level() {
    let l1 = signature_reminder(1, 5).body();
    let l2 = signature_reminder(2, 3).body();
    let l3 = signature_reminder(3, 1).body();
    assert!(l1.contains("5 days"));
    assert!(l2.contains("3 days"));
    // Level 3+ is the urgent copy and must differ from the lower levels.
    assert!(l3.contains("URGENT"));
    assert_ne!(l1, l2);
    assert_ne!(l2, l3);
}

#[test]
fn fault_status_body_names_both_statuses() {
    let body = fault_status(FaultStatus::Reported, FaultStatus::Resolved).body();
    assert!(body.contains("Reported"));
    assert!(body.contains("Resolved"));
}

// ---------------------------------------------------------------------------
// action_url() — every variant deep-links, and the path embeds the entity id.
// ---------------------------------------------------------------------------

#[test]
fn every_variant_has_a_deep_link() {
    for event in all_variants() {
        assert!(
            event.action_url().is_some(),
            "missing action_url for {event:?}"
        );
    }
}

#[test]
fn action_urls_point_at_the_correct_resource() {
    let fault_id = Uuid::new_v4();
    let event = NotificationEvent::FaultStatusChanged {
        fault_id,
        reporter_id: Uuid::new_v4(),
        technician_id: None,
        old_status: FaultStatus::Reported,
        new_status: FaultStatus::Triaged,
        title: "x".to_string(),
    };
    assert_eq!(event.action_url(), Some(format!("/faults/{fault_id}")));

    let vote_id = Uuid::new_v4();
    let vote = NotificationEvent::VoteReminder {
        vote_id,
        title: "x".to_string(),
        deadline: Utc::now(),
        hours_remaining: 3,
    };
    assert_eq!(vote.action_url(), Some(format!("/votes/{vote_id}")));

    let payment_id = Uuid::new_v4();
    let pay = NotificationEvent::PaymentReceived {
        payment_id,
        amount: "1.00".to_string(),
    };
    assert_eq!(pay.action_url(), Some(format!("/payments/{payment_id}")));
}

// ---------------------------------------------------------------------------
// Wire-format stability — the trigger event is serialized onto the event bus,
// so the `type` tag discriminator must stay snake_case and round-trip.
// ---------------------------------------------------------------------------

#[test]
fn event_serializes_with_snake_case_type_tag() {
    let json = serde_json::to_value(emergency()).expect("serialize");
    assert_eq!(json["type"], "emergency_alert");
    assert_eq!(json["severity"], "critical");

    let json = serde_json::to_value(payment_due()).expect("serialize");
    assert_eq!(json["type"], "payment_due");
}

#[test]
fn event_round_trips_through_json() {
    for event in all_variants() {
        let json = serde_json::to_string(&event).expect("serialize");
        let back: NotificationEvent = serde_json::from_str(&json).expect("deserialize");
        // Mapping outputs must be stable across the round-trip.
        assert_eq!(event.category(), back.category());
        assert_eq!(event.priority(), back.priority());
        assert_eq!(event.title(), back.title());
        assert_eq!(event.action_url(), back.action_url());
    }
}
