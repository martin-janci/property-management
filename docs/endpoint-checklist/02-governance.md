# Governance & Community endpoints

| Method + Path | Handler | Status | Test | Notes |
|---|---|---|---|---|
| `POST /api/v1/voting` | `voting.rs:create_vote` | done | `voting_tests.rs` | |
| `GET /api/v1/voting` | `voting.rs:list_votes` | done | `voting_tests.rs` | |
| `GET /api/v1/voting/{id}` | `voting.rs:get_vote` | done | `voting_tests.rs` | |
| `PUT /api/v1/voting/{id}` | `voting.rs:update_vote` | partial | — | no test |
| `DELETE /api/v1/voting/{id}` | `voting.rs:delete_vote` | partial | — | no test |
| `POST /api/v1/voting/{id}/publish` | `voting.rs:publish_vote` | done | `voting_tests.rs` | |
| `POST /api/v1/voting/{id}/cancel` | `voting.rs:cancel_vote` | partial | — | no test |
| `POST /api/v1/voting/{id}/close` | `voting.rs:close_vote` | partial | — | no test |
| `POST /api/v1/voting/{id}/questions` | `voting.rs:add_question` | partial | — | no test |
| `GET /api/v1/voting/{id}/questions` | `voting.rs:list_questions` | partial | — | no test |
| `PUT /api/v1/voting/{id}/questions/{question_id}` | `voting.rs:update_question` | partial | — | no test |
| `DELETE /api/v1/voting/{id}/questions/{question_id}` | `voting.rs:delete_question` | partial | — | no test |
| `GET /api/v1/voting/{id}/eligibility` | `voting.rs:check_eligibility` | partial | — | no test |
| `POST /api/v1/voting/{id}/cast` | `voting.rs:cast_vote` | done | `voting_tests.rs` | |
| `GET /api/v1/voting/{id}/my-response` | `voting.rs:get_my_response` | partial | — | no test |
| `POST /api/v1/voting/{id}/comments` | `voting.rs:add_comment` | partial | — | no test |
| `GET /api/v1/voting/{id}/comments` | `voting.rs:list_comments` | partial | — | no test |
| `GET /api/v1/voting/{id}/comments/{comment_id}/replies` | `voting.rs:list_replies` | partial | — | no test |
| `POST /api/v1/voting/{id}/comments/{comment_id}/hide` | `voting.rs:hide_comment` | partial | — | no test |
| `GET /api/v1/voting/{id}/results` | `voting.rs:get_results` | done | `voting_tests.rs` | |
| `GET /api/v1/voting/{id}/report` | `voting.rs:get_report_data` | done | `voting_report_tests.rs` | |
| `GET /api/v1/voting/{id}/report.pdf` | `voting.rs:get_report_pdf` | done | `voting_report_tests.rs` | |
| `GET /api/v1/voting/{id}/audit` | `voting.rs:get_audit_log` | partial | — | no test |
| `GET /api/v1/voting/building/{building_id}/active` | `voting.rs:list_active_by_building` | done | `voting_tests.rs` | |
| `POST /api/v1/delegations` | `delegations.rs:create_delegation` | partial | — | no test |
| `GET /api/v1/delegations` | `delegations.rs:list_my_delegations` | partial | — | no test |
| `GET /api/v1/delegations/received` | `delegations.rs:list_received_delegations` | partial | — | no test |
| `GET /api/v1/delegations/{id}` | `delegations.rs:get_delegation` | partial | — | no test |
| `POST /api/v1/delegations/{id}/accept` | `delegations.rs:accept_delegation` | partial | — | no test |
| `POST /api/v1/delegations/{id}/decline` | `delegations.rs:decline_delegation` | partial | — | no test |
| `DELETE /api/v1/delegations/{id}/revoke` | `delegations.rs:revoke_delegation` | partial | — | no test |
| `GET /api/v1/delegations/check/{unit_id}/{scope}` | `delegations.rs:check_delegation` | partial | — | no test |
| `GET /api/v1/board-meetings/dashboard` | `board_meetings.rs:get_dashboard` | partial | — | no test |
| `GET /api/v1/board-meetings/members` | `board_meetings.rs:list_board_members` | partial | — | no test |
| `POST /api/v1/board-meetings/members` | `board_meetings.rs:create_board_member` | partial | — | no test |
| `GET /api/v1/board-meetings/members/{id}` | `board_meetings.rs:get_board_member` | done | `board_meetings_auth_tests.rs` | |
| `PUT /api/v1/board-meetings/members/{id}` | `board_meetings.rs:update_board_member` | done | `board_meetings_auth_tests.rs` | |
| `DELETE /api/v1/board-meetings/members/{id}` | `board_meetings.rs:delete_board_member` | done | `board_meetings_auth_tests.rs` | |
| `GET /api/v1/board-meetings/members/{id}/attendance` | `board_meetings.rs:get_member_attendance` | done | `board_meetings_auth_tests.rs` | |
| `GET /api/v1/board-meetings` | `board_meetings.rs:list_meetings` | partial | — | no test |
| `POST /api/v1/board-meetings` | `board_meetings.rs:create_meeting` | partial | — | no test |
| `GET /api/v1/board-meetings/{id}` | `board_meetings.rs:get_meeting` | done | `board_meetings_auth_tests.rs` | |
| `PUT /api/v1/board-meetings/{id}` | `board_meetings.rs:update_meeting` | done | `board_meetings_cross_org_idor_tests.rs` | |
| `DELETE /api/v1/board-meetings/{id}` | `board_meetings.rs:delete_meeting` | done | `board_meetings_auth_tests.rs` | |
| `POST /api/v1/board-meetings/{id}/start` | `board_meetings.rs:start_meeting` | done | `board_meetings_auth_tests.rs` | |
| `POST /api/v1/board-meetings/{id}/end` | `board_meetings.rs:end_meeting` | done | `board_meetings_auth_tests.rs` | |
| `POST /api/v1/board-meetings/{id}/cancel` | `board_meetings.rs:cancel_meeting` | partial | — | no test |
| `GET /api/v1/board-meetings/{meeting_id}/agenda` | `board_meetings.rs:list_agenda_items` | partial | — | no test |
| `POST /api/v1/board-meetings/{meeting_id}/agenda` | `board_meetings.rs:add_agenda_item` | done | `board_meetings_cross_org_idor_tests.rs` | |
| `GET /api/v1/board-meetings/agenda/{id}` | `board_meetings.rs:get_agenda_item` | done | `board_meetings_cross_org_idor_tests.rs` | |
| `PUT /api/v1/board-meetings/agenda/{id}` | `board_meetings.rs:update_agenda_item` | partial | — | no test |
| `POST /api/v1/board-meetings/agenda/{id}/complete` | `board_meetings.rs:complete_agenda_item` | partial | — | no test |
| `DELETE /api/v1/board-meetings/agenda/{id}` | `board_meetings.rs:delete_agenda_item` | partial | — | no test |
| `GET /api/v1/board-meetings/{meeting_id}/motions` | `board_meetings.rs:list_meeting_motions` | partial | — | no test |
| `POST /api/v1/board-meetings/{meeting_id}/motions` | `board_meetings.rs:create_motion` | partial | — | no test |
| `GET /api/v1/board-meetings/motions` | `board_meetings.rs:list_motions` | partial | — | no test |
| `GET /api/v1/board-meetings/motions/{id}` | `board_meetings.rs:get_motion` | done | `board_meetings_auth_tests.rs` | |
| `PUT /api/v1/board-meetings/motions/{id}` | `board_meetings.rs:update_motion` | done | `board_meetings_auth_tests.rs` | |
| `POST /api/v1/board-meetings/motions/{id}/second` | `board_meetings.rs:second_motion` | partial | — | no test |
| `POST /api/v1/board-meetings/motions/{id}/start-voting` | `board_meetings.rs:start_motion_voting` | done | `board_meetings_auth_tests.rs` | |
| `POST /api/v1/board-meetings/motions/{id}/end-voting` | `board_meetings.rs:end_motion_voting` | done | `board_meetings_auth_tests.rs` | |
| `POST /api/v1/board-meetings/motions/{id}/vote` | `board_meetings.rs:cast_vote` | done | `board_meetings_auth_tests.rs` | |
| `DELETE /api/v1/board-meetings/motions/{id}` | `board_meetings.rs:delete_motion` | done | `board_meetings_auth_tests.rs` | |
| `GET /api/v1/board-meetings/{meeting_id}/attendance` | `board_meetings.rs:list_attendance` | partial | — | no test |
| `POST /api/v1/board-meetings/{meeting_id}/attendance` | `board_meetings.rs:record_attendance` | partial | — | no test |
| `GET /api/v1/board-meetings/{meeting_id}/minutes` | `board_meetings.rs:get_minutes` | partial | — | no test |
| `POST /api/v1/board-meetings/{meeting_id}/minutes` | `board_meetings.rs:create_minutes` | partial | — | no test |
| `PUT /api/v1/board-meetings/minutes/{id}` | `board_meetings.rs:update_minutes` | partial | — | no test |
| `POST /api/v1/board-meetings/minutes/{id}/approve` | `board_meetings.rs:approve_minutes` | partial | — | no test |
| `GET /api/v1/board-meetings/{meeting_id}/actions` | `board_meetings.rs:list_meeting_action_items` | partial | — | no test |
| `POST /api/v1/board-meetings/{meeting_id}/actions` | `board_meetings.rs:create_action_item` | partial | — | no test |
| `GET /api/v1/board-meetings/actions` | `board_meetings.rs:list_action_items` | partial | — | no test |
| `GET /api/v1/board-meetings/actions/{id}` | `board_meetings.rs:get_action_item` | partial | — | no test |
| `PUT /api/v1/board-meetings/actions/{id}` | `board_meetings.rs:update_action_item` | partial | — | no test |
| `POST /api/v1/board-meetings/actions/{id}/complete` | `board_meetings.rs:complete_action_item` | partial | — | no test |
| `DELETE /api/v1/board-meetings/actions/{id}` | `board_meetings.rs:delete_action_item` | partial | — | no test |
| `GET /api/v1/board-meetings/{meeting_id}/documents` | `board_meetings.rs:list_documents` | partial | — | no test |
| `POST /api/v1/board-meetings/{meeting_id}/documents` | `board_meetings.rs:upload_document` | partial | — | no test |
| `DELETE /api/v1/board-meetings/documents/{id}` | `board_meetings.rs:delete_document` | partial | — | no test |
| `GET /api/v1/board-meetings/statistics/types` | `board_meetings.rs:get_meeting_type_counts` | partial | — | no test |
| `GET /api/v1/board-meetings/statistics/motions` | `board_meetings.rs:get_motion_status_counts` | partial | — | no test |
| `POST /api/v1/disputes` | `disputes.rs:file_dispute` | partial | — | no test |
| `GET /api/v1/disputes` | `disputes.rs:list_disputes` | partial | — | no test |
| `GET /api/v1/disputes/statistics` | `disputes.rs:get_statistics` | partial | — | no test |
| `GET /api/v1/disputes/{id}` | `disputes.rs:get_dispute` | partial | — | no test |
| `PATCH /api/v1/disputes/{id}` | `disputes.rs:update_dispute_status` | partial | — | no test |
| `DELETE /api/v1/disputes/{id}` | `disputes.rs:withdraw_dispute` | partial | — | no test |
| `PATCH /api/v1/disputes/{id}/resolve` | `disputes.rs:resolve_dispute` | done | `dispute_cross_org_idor_tests.rs` | |
| `PATCH /api/v1/disputes/{id}/mediation-notes` | `disputes.rs:update_mediation_notes` | done | `dispute_cross_org_idor_tests.rs` | |
| `GET /api/v1/disputes/{id}/parties` | `disputes.rs:list_parties` | partial | — | no test |
| `POST /api/v1/disputes/{id}/parties` | `disputes.rs:add_party` | partial | — | no test |
| `GET /api/v1/disputes/{id}/evidence` | `disputes.rs:list_evidence` | partial | — | no test |
| `POST /api/v1/disputes/{id}/evidence` | `disputes.rs:add_evidence` | done | `dispute_evidence_auth_tests.rs` | |
| `DELETE /api/v1/disputes/{id}/evidence/{evidence_id}` | `disputes.rs:delete_evidence` | partial | — | no test |
| `GET /api/v1/disputes/{id}/activities` | `disputes.rs:list_activities` | partial | — | no test |
| `GET /api/v1/disputes/{id}/sessions` | `disputes.rs:list_sessions` | partial | — | no test |
| `POST /api/v1/disputes/{id}/sessions` | `disputes.rs:schedule_session` | partial | — | no test |
| `GET /api/v1/disputes/{id}/sessions/{session_id}` | `disputes.rs:get_session` | partial | — | no test |
| `PATCH /api/v1/disputes/{id}/sessions/{session_id}` | `disputes.rs:update_session` | partial | — | no test |
| `POST /api/v1/disputes/{id}/sessions/{session_id}/cancel` | `disputes.rs:cancel_session` | partial | — | no test |
| `GET /api/v1/disputes/{id}/sessions/{session_id}/attendance` | `disputes.rs:get_attendance` | partial | — | no test |
| `PATCH /api/v1/disputes/{id}/sessions/{session_id}/attendance/{party_id}` | `disputes.rs:update_attendance` | partial | — | no test |
| `POST /api/v1/disputes/{id}/sessions/{session_id}/notes` | `disputes.rs:record_notes` | partial | — | no test |
| `GET /api/v1/disputes/{id}/submissions` | `disputes.rs:list_submissions` | partial | — | no test |
| `POST /api/v1/disputes/{id}/submissions` | `disputes.rs:submit_response` | partial | — | no test |
| `GET /api/v1/disputes/{id}/mediation-case` | `disputes.rs:get_mediation_case` | partial | — | no test |
| `GET /api/v1/disputes/{id}/resolutions` | `disputes.rs:list_resolutions` | partial | — | no test |
| `POST /api/v1/disputes/{id}/resolutions` | `disputes.rs:propose_resolution` | partial | — | no test |
| `GET /api/v1/disputes/{id}/resolutions/{resolution_id}` | `disputes.rs:get_resolution` | partial | — | no test |
| `POST /api/v1/disputes/{id}/resolutions/{resolution_id}/vote` | `disputes.rs:vote_on_resolution` | partial | — | no test |
| `POST /api/v1/disputes/{id}/resolutions/{resolution_id}/accept` | `disputes.rs:accept_resolution` | partial | — | no test |
| `POST /api/v1/disputes/{id}/resolutions/{resolution_id}/implement` | `disputes.rs:implement_resolution` | partial | — | no test |
| `GET /api/v1/disputes/{id}/actions` | `disputes.rs:list_action_items` | partial | — | no test |
| `POST /api/v1/disputes/{id}/actions` | `disputes.rs:create_action_item` | partial | — | no test |
| `GET /api/v1/disputes/{id}/actions/{action_id}` | `disputes.rs:get_action_item` | partial | — | no test |
| `PATCH /api/v1/disputes/{id}/actions/{action_id}` | `disputes.rs:update_action_item` | partial | — | no test |
| `POST /api/v1/disputes/{id}/actions/{action_id}/complete` | `disputes.rs:complete_action_item` | partial | — | no test |
| `POST /api/v1/disputes/{id}/actions/{action_id}/remind` | `disputes.rs:send_action_reminder` | partial | — | no test |
| `GET /api/v1/disputes/{id}/escalations` | `disputes.rs:list_escalations` | partial | — | no test |
| `POST /api/v1/disputes/{id}/escalations` | `disputes.rs:create_escalation` | partial | — | no test |
| `POST /api/v1/disputes/{id}/escalations/{escalation_id}/resolve` | `disputes.rs:resolve_escalation` | partial | — | no test |
| `GET /api/v1/disputes/my-actions` | `disputes.rs:get_my_actions` | partial | — | no test |
| `GET /api/v1/disputes/overdue-actions` | `disputes.rs:list_overdue_actions` | partial | — | no test |
| `GET /api/v1/violations/rules` | `violations.rs:list_rules` | partial | — | no test |
| `POST /api/v1/violations/rules` | `violations.rs:create_rule` | partial | — | no test |
| `GET /api/v1/violations/rules/{rule_id}` | `violations.rs:get_rule` | partial | — | no test |
| `PUT /api/v1/violations/rules/{rule_id}` | `violations.rs:update_rule` | partial | — | no test |
| `DELETE /api/v1/violations/rules/{rule_id}` | `violations.rs:delete_rule` | partial | — | no test |
| `GET /api/v1/violations` | `violations.rs:list_violations` | partial | — | no test |
| `POST /api/v1/violations` | `violations.rs:create_violation` | partial | — | no test |
| `GET /api/v1/violations/{violation_id}` | `violations.rs:get_violation` | done | `violations_cross_org_idor_tests.rs` | |
| `PUT /api/v1/violations/{violation_id}` | `violations.rs:update_violation` | partial | — | no test |
| `POST /api/v1/violations/{violation_id}/assign` | `violations.rs:assign_violation` | partial | — | no test |
| `GET /api/v1/violations/{violation_id}/evidence` | `violations.rs:list_evidence` | partial | — | no test |
| `POST /api/v1/violations/{violation_id}/evidence` | `violations.rs:add_evidence` | partial | — | no test |
| `DELETE /api/v1/violations/{violation_id}/evidence/{evidence_id}` | `violations.rs:delete_evidence` | partial | — | no test |
| `GET /api/v1/violations/{violation_id}/actions` | `violations.rs:list_actions` | partial | — | no test |
| `POST /api/v1/violations/{violation_id}/actions` | `violations.rs:create_action` | partial | — | no test |
| `GET /api/v1/violations/{violation_id}/actions/{action_id}` | `violations.rs:get_action` | partial | — | no test |
| `PUT /api/v1/violations/{violation_id}/actions/{action_id}` | `violations.rs:update_action` | partial | — | no test |
| `POST /api/v1/violations/{violation_id}/actions/{action_id}/send` | `violations.rs:mark_action_sent` | partial | — | no test |
| `GET /api/v1/violations/{violation_id}/actions/{action_id}/payments` | `violations.rs:list_payments` | partial | — | no test |
| `POST /api/v1/violations/{violation_id}/actions/{action_id}/payments` | `violations.rs:record_payment` | partial | — | no test |
| `POST /api/v1/violations/{violation_id}/appeals` | `violations.rs:create_appeal` | partial | — | no test |
| `GET /api/v1/violations/appeals` | `violations.rs:list_appeals` | partial | — | no test |
| `GET /api/v1/violations/appeals/{appeal_id}` | `violations.rs:get_appeal` | partial | — | no test |
| `PUT /api/v1/violations/appeals/{appeal_id}` | `violations.rs:update_appeal` | partial | — | no test |
| `POST /api/v1/violations/appeals/{appeal_id}/decide` | `violations.rs:decide_appeal` | partial | — | no test |
| `GET /api/v1/violations/{violation_id}/comments` | `violations.rs:list_comments` | partial | — | no test |
| `POST /api/v1/violations/{violation_id}/comments` | `violations.rs:add_comment` | partial | — | no test |
| `GET /api/v1/violations/dashboard` | `violations.rs:get_dashboard` | partial | — | no test |
| `GET /api/v1/violations/history` | `violations.rs:get_violator_history` | partial | — | no test |
| `GET /api/v1/violations/statistics` | `violations.rs:get_statistics` | partial | — | no test |
| `GET /api/v1/buildings/{building_id}/neighbors` | `neighbors.rs:list_neighbors` | partial | — | no test |
| `GET /api/v1/users/me/privacy` | `neighbors.rs:get_privacy_settings` | partial | — | no test |
| `PUT /api/v1/users/me/privacy` | `neighbors.rs:update_privacy_settings` | partial | — | no test |
| `GET /api/v1/community/buildings/{building_id}/groups` | `community.rs:list_groups` | partial | — | no test |
| `POST /api/v1/community/buildings/{building_id}/groups` | `community.rs:create_group` | partial | — | no test |
| `GET /api/v1/community/groups/{id}` | `community.rs:get_group` | partial | — | no test |
| `POST /api/v1/community/groups/{id}/join` | `community.rs:join_group` | partial | — | no test |
| `POST /api/v1/community/groups/{id}/leave` | `community.rs:leave_group` | partial | — | no test |
| `GET /api/v1/community/groups/{group_id}/posts` | `community.rs:list_posts` | partial | — | no test |
| `POST /api/v1/community/groups/{group_id}/posts` | `community.rs:create_post` | partial | — | no test |
| `POST /api/v1/community/posts/{id}/reactions` | `community.rs:add_reaction` | partial | — | no test |
| `POST /api/v1/community/posts/{id}/comments` | `community.rs:create_comment` | partial | — | no test |
| `GET /api/v1/community/buildings/{building_id}/events` | `community.rs:list_events` | partial | — | no test |
| `POST /api/v1/community/buildings/{building_id}/events` | `community.rs:create_event` | partial | — | no test |
| `POST /api/v1/community/events/{id}/rsvp` | `community.rs:rsvp_event` | partial | — | no test |
| `GET /api/v1/community/buildings/{building_id}/marketplace` | `community.rs:list_items` | partial | — | no test |
| `POST /api/v1/community/buildings/{building_id}/marketplace` | `community.rs:create_item` | partial | — | no test |
| `GET /api/v1/community/marketplace/{id}` | `community.rs:get_item` | partial | — | no test |
| `POST /api/v1/community/marketplace/{id}/inquiries` | `community.rs:create_inquiry` | partial | — | no test |

## Tally
done: 30  partial: 142  stub: 0  missing: 0  total: 172
