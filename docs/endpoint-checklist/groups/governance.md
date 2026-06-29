# Governance

_Server: api-server. Modules: voting.rs, board_meetings.rs, delegations.rs, violations.rs, disputes.rs, community.rs, neighbors.rs, package_visitor.rs, reserve_funds/._

No stub markers (`todo!`/`unimplemented!`/`NOT_IMPLEMENTED`/`ROADMAP`) found in any governance module; all handlers are real. Test coverage is dominated by authz (401/403) and cross-org IDOR (404/reject) suites that do NOT exercise the success path. Genuine happy-path (200/201) coverage exists only for a handful of GET-by-id / report endpoints; everything else is `partial`.

## voting.rs  (mount: /api/v1/voting)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/voting | create_vote | done | voting_tests.rs | happy-path integrated |
| GET | /api/v1/voting | list_votes | done | voting_tests.rs | happy-path integrated |
| GET | /api/v1/voting/{id} | get_vote | done | marketplace_voting_investor_cross_org_idor_tests.rs | `get_vote_for_own_org_succeeds` asserts 200 |
| PUT | /api/v1/voting/{id} | update_vote | done | voting_behaviour_tests.rs | happy-path: update title |
| DELETE | /api/v1/voting/{id} | delete_vote | done | voting_behaviour_tests.rs | happy-path: delete draft |
| POST | /api/v1/voting/{id}/publish | publish_vote | done | voting_behaviour_tests.rs | happy-path: publish draft |
| POST | /api/v1/voting/{id}/cancel | cancel_vote | done | voting_behaviour_tests.rs | happy-path: cancel active |
| POST | /api/v1/voting/{id}/close | close_vote | done | voting_behaviour_tests.rs | happy-path: close active |
| POST | /api/v1/voting/{id}/questions | add_question | done | voting_behaviour_tests.rs | happy-path: add question |
| GET | /api/v1/voting/{id}/questions | list_questions | done | voting_behaviour_tests.rs | happy-path: list questions |
| PUT | /api/v1/voting/{id}/questions/{question_id} | update_question | done | voting_behaviour_tests.rs | happy-path: update question |
| DELETE | /api/v1/voting/{id}/questions/{question_id} | delete_question | done | voting_behaviour_tests.rs | happy-path: delete question |
| GET | /api/v1/voting/{id}/eligibility | check_eligibility | done | voting_behaviour_tests.rs | happy-path: eligibility check |
| POST | /api/v1/voting/{id}/cast | cast_vote | done | voting_tests.rs | happy-path integrated |
| GET | /api/v1/voting/{id}/my-response | get_my_response | done | voting_behaviour_tests.rs | happy-path: get my response after cast |
| POST | /api/v1/voting/{id}/comments | add_comment | done | voting_behaviour_tests.rs | happy-path: add comment |
| GET | /api/v1/voting/{id}/comments | list_comments | done | voting_behaviour_tests.rs | happy-path: list comments |
| GET | /api/v1/voting/{id}/comments/{comment_id}/replies | list_replies | done | voting_behaviour_tests.rs | happy-path: list replies |
| POST | /api/v1/voting/{id}/comments/{comment_id}/hide | hide_comment | done | voting_behaviour_tests.rs | happy-path: hide comment |
| GET | /api/v1/voting/{id}/results | get_results | done | voting_tests.rs | happy-path integrated |
| GET | /api/v1/voting/{id}/report | get_report_data | done | voting_report_tests.rs | 200 (json + format=pdf) |
| GET | /api/v1/voting/{id}/report.pdf | get_report_pdf | done | voting_report_tests.rs | 200, archives document |
| GET | /api/v1/voting/{id}/audit | get_audit_log | done | voting_behaviour_tests.rs | happy-path: audit log |
| GET | /api/v1/voting/building/{building_id}/active | list_active_by_building | done | voting_behaviour_tests.rs | happy-path: active votes for building |

## board_meetings.rs  (mount: /api/v1/board-meetings)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/board-meetings/dashboard | get_dashboard | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/board-meetings/members | list_board_members | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/board-meetings/members | create_board_member | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/board-meetings/members/{id} | get_board_member | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| PUT | /api/v1/board-meetings/members/{id} | update_board_member | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| DELETE | /api/v1/board-meetings/members/{id} | delete_board_member | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/board-meetings/members/{id}/attendance | get_member_attendance | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/board-meetings | list_meetings | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/board-meetings | create_meeting | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/board-meetings/{id} | get_meeting | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| PUT | /api/v1/board-meetings/{id} | update_meeting | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| DELETE | /api/v1/board-meetings/{id} | delete_meeting | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/board-meetings/{id}/start | start_meeting | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/board-meetings/{id}/end | end_meeting | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/board-meetings/{id}/cancel | cancel_meeting | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/board-meetings/{meeting_id}/agenda | list_agenda_items | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/board-meetings/{meeting_id}/agenda | add_agenda_item | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/board-meetings/agenda/{id} | get_agenda_item | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| PUT | /api/v1/board-meetings/agenda/{id} | update_agenda_item | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/board-meetings/agenda/{id}/complete | complete_agenda_item | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| DELETE | /api/v1/board-meetings/agenda/{id} | delete_agenda_item | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/board-meetings/{meeting_id}/motions | list_meeting_motions | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/board-meetings/{meeting_id}/motions | create_motion | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/board-meetings/motions | list_motions | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/board-meetings/motions/{id} | get_motion | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| PUT | /api/v1/board-meetings/motions/{id} | update_motion | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/board-meetings/motions/{id}/second | second_motion | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/board-meetings/motions/{id}/start-voting | start_motion_voting | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/board-meetings/motions/{id}/end-voting | end_motion_voting | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/board-meetings/motions/{id}/vote | cast_vote | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| DELETE | /api/v1/board-meetings/motions/{id} | delete_motion | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/board-meetings/{meeting_id}/attendance | list_attendance | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/board-meetings/{meeting_id}/attendance | record_attendance | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/board-meetings/{meeting_id}/minutes | get_minutes | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/board-meetings/{meeting_id}/minutes | create_minutes | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| PUT | /api/v1/board-meetings/minutes/{id} | update_minutes | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/board-meetings/minutes/{id}/approve | approve_minutes | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/board-meetings/{meeting_id}/actions | list_meeting_action_items | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/board-meetings/{meeting_id}/actions | create_action_item | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/board-meetings/actions | list_action_items | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/board-meetings/actions/{id} | get_action_item | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| PUT | /api/v1/board-meetings/actions/{id} | update_action_item | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/board-meetings/actions/{id}/complete | complete_action_item | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| DELETE | /api/v1/board-meetings/actions/{id} | delete_action_item | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/board-meetings/{meeting_id}/documents | list_documents | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/board-meetings/{meeting_id}/documents | upload_document | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| DELETE | /api/v1/board-meetings/documents/{id} | delete_document | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/board-meetings/statistics/types | get_meeting_type_counts | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/board-meetings/statistics/motions | get_motion_status_counts | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |

## delegations.rs  (mount: /api/v1/delegations)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/delegations | create_delegation | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/delegations | list_my_delegations | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/delegations/received | list_received_delegations | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/delegations/{id} | get_delegation | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/delegations/{id}/accept | accept_delegation | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/delegations/{id}/decline | decline_delegation | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| DELETE | /api/v1/delegations/{id}/revoke | revoke_delegation | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/delegations/check/{unit_id}/{scope} | check_delegation | done | board_meetings_happy_path_tests.rs | happy-path 2xx (BIT-408) |

## violations.rs  (mount: /api/v1/violations)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/violations/rules | list_rules | done | violations_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/violations/rules | create_rule | done | violations_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/violations/rules/{rule_id} | get_rule | done | violations_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| PUT | /api/v1/violations/rules/{rule_id} | update_rule | done | violations_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| DELETE | /api/v1/violations/rules/{rule_id} | delete_rule | done | violations_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/violations | list_violations | done | violations_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/violations | create_violation | done | violations_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/violations/{violation_id} | get_violation | done | violations_cross_org_idor_tests.rs | `get_violation_same_org_succeeds` asserts 200 |
| PUT | /api/v1/violations/{violation_id} | update_violation | done | violations_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/violations/{violation_id}/assign | assign_violation | done | violations_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/violations/{violation_id}/evidence | list_evidence | done | violations_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/violations/{violation_id}/evidence | add_evidence | done | violations_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| DELETE | /api/v1/violations/{violation_id}/evidence/{evidence_id} | delete_evidence | partial | — | no test |
| GET | /api/v1/violations/{violation_id}/actions | list_actions | done | violations_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/violations/{violation_id}/actions | create_action | done | violations_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/violations/{violation_id}/actions/{action_id} | get_action | done | violations_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| PUT | /api/v1/violations/{violation_id}/actions/{action_id} | update_action | done | violations_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/violations/{violation_id}/actions/{action_id}/send | mark_action_sent | done | violations_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/violations/{violation_id}/actions/{action_id}/payments | list_payments | done | violations_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/violations/{violation_id}/actions/{action_id}/payments | record_payment | done | violations_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/violations/{violation_id}/appeals | create_appeal | done | violations_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/violations/appeals | list_appeals | done | violations_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/violations/appeals/{appeal_id} | get_appeal | done | violations_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| PUT | /api/v1/violations/appeals/{appeal_id} | update_appeal | done | violations_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/violations/appeals/{appeal_id}/decide | decide_appeal | done | violations_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/violations/{violation_id}/comments | list_comments | done | violations_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/violations/{violation_id}/comments | add_comment | done | violations_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/violations/dashboard | get_dashboard | done | violations_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/violations/history | get_violator_history | done | violations_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/violations/statistics | get_statistics | done | violations_happy_path_tests.rs | happy-path 2xx (BIT-408) |

## disputes.rs  (mount: /api/v1/disputes)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/disputes | file_dispute | done | disputes_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/disputes | list_disputes | done | disputes_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/disputes/statistics | get_statistics | done | disputes_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/disputes/{id} | get_dispute | done | disputes_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| PATCH | /api/v1/disputes/{id} | update_dispute_status | done | disputes_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| DELETE | /api/v1/disputes/{id} | withdraw_dispute | done | disputes_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| PATCH | /api/v1/disputes/{id}/resolve | resolve_dispute | done | disputes_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| PATCH | /api/v1/disputes/{id}/mediation-notes | update_mediation_notes | done | disputes_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/disputes/{id}/parties | list_parties | done | disputes_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/disputes/{id}/parties | add_party | done | disputes_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/disputes/{id}/evidence | list_evidence | done | disputes_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/disputes/{id}/evidence | add_evidence | done | disputes_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| DELETE | /api/v1/disputes/{id}/evidence/{evidence_id} | delete_evidence | partial | — | no test |
| GET | /api/v1/disputes/{id}/activities | list_activities | done | disputes_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/disputes/{id}/sessions | list_sessions | done | disputes_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/disputes/{id}/sessions | schedule_session | done | disputes_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/disputes/{id}/sessions/{session_id} | get_session | done | disputes_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| PATCH | /api/v1/disputes/{id}/sessions/{session_id} | update_session | done | disputes_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/disputes/{id}/sessions/{session_id}/cancel | cancel_session | done | disputes_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/disputes/{id}/sessions/{session_id}/attendance | get_attendance | done | disputes_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| PATCH | /api/v1/disputes/{id}/sessions/{session_id}/attendance/{party_id} | update_attendance | done | disputes_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/disputes/{id}/sessions/{session_id}/notes | record_notes | done | disputes_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/disputes/{id}/submissions | list_submissions | done | disputes_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/disputes/{id}/submissions | submit_response | done | disputes_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/disputes/{id}/mediation-case | get_mediation_case | done | disputes_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/disputes/{id}/resolutions | list_resolutions | done | disputes_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/disputes/{id}/resolutions | propose_resolution | done | disputes_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/disputes/{id}/resolutions/{resolution_id} | get_resolution | done | disputes_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/disputes/{id}/resolutions/{resolution_id}/vote | vote_on_resolution | done | disputes_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/disputes/{id}/resolutions/{resolution_id}/accept | accept_resolution | done | disputes_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/disputes/{id}/resolutions/{resolution_id}/implement | implement_resolution | done | disputes_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/disputes/{id}/actions | list_action_items | done | disputes_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/disputes/{id}/actions | create_action_item | done | disputes_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/disputes/{id}/actions/{action_id} | get_action_item | done | disputes_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| PATCH | /api/v1/disputes/{id}/actions/{action_id} | update_action_item | done | disputes_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/disputes/{id}/actions/{action_id}/complete | complete_action_item | done | disputes_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/disputes/{id}/actions/{action_id}/remind | send_action_reminder | done | disputes_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/disputes/{id}/escalations | list_escalations | done | disputes_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/disputes/{id}/escalations | create_escalation | done | disputes_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/disputes/{id}/escalations/{escalation_id}/resolve | resolve_escalation | done | disputes_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/disputes/my-actions | get_my_actions | done | disputes_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/disputes/overdue-actions | list_overdue_actions | done | disputes_happy_path_tests.rs | happy-path 2xx (BIT-408) |

## community.rs  (mount: /api/v1/community)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/community/buildings/{building_id}/groups | list_groups | done | community_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/community/buildings/{building_id}/groups | create_group | done | community_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/community/groups/{id} | get_group | done | community_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/community/groups/{id}/join | join_group | done | community_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/community/groups/{id}/leave | leave_group | done | community_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/community/groups/{group_id}/posts | list_posts | done | community_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/community/groups/{group_id}/posts | create_post | done | community_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/community/posts/{id}/reactions | add_reaction | done | community_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/community/posts/{id}/comments | create_comment | done | community_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/community/buildings/{building_id}/events | list_events | done | community_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/community/buildings/{building_id}/events | create_event | done | community_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/community/events/{id}/rsvp | rsvp_event | done | community_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/community/buildings/{building_id}/marketplace | list_items | done | community_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/community/buildings/{building_id}/marketplace | create_item | done | community_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/community/marketplace/{id} | get_item | done | community_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/community/marketplace/{id}/inquiries | create_inquiry | done | community_happy_path_tests.rs | happy-path 2xx (BIT-408) |

## neighbors.rs  (mount: /api/v1)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/buildings/{building_id}/neighbors | list_neighbors | done | community_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/users/me/privacy | get_privacy_settings | done | community_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| PUT | /api/v1/users/me/privacy | update_privacy_settings | done | community_happy_path_tests.rs | happy-path 2xx (BIT-408) |

## package_visitor.rs — packages (mount: /api/v1/packages)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/packages | create_package | done | package_visitor_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/packages | list_packages | done | package_visitor_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/packages/{id} | get_package | done | package_visitor_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| PUT | /api/v1/packages/{id} | update_package | done | package_visitor_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| DELETE | /api/v1/packages/{id} | delete_package | done | package_visitor_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/packages/{id}/receive | receive_package | done | package_visitor_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/packages/{id}/pickup | pickup_package | done | package_visitor_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/packages/buildings/{building_id}/settings | get_package_settings | done | package_visitor_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| PUT | /api/v1/packages/buildings/{building_id}/settings | update_package_settings | done | package_visitor_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/packages/buildings/{building_id}/statistics | get_package_statistics | done | package_visitor_happy_path_tests.rs | happy-path 2xx (BIT-408) |

## package_visitor.rs — visitors (mount: /api/v1/visitors)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/visitors | create_visitor | done | package_visitor_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/visitors | list_visitors | done | package_visitor_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/visitors/verify-code | verify_access_code | done | package_visitor_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/visitors/{id} | get_visitor | done | package_visitor_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| PUT | /api/v1/visitors/{id} | update_visitor | done | package_visitor_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| DELETE | /api/v1/visitors/{id} | delete_visitor | done | package_visitor_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/visitors/{id}/check-in | check_in_visitor | done | package_visitor_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/visitors/{id}/check-out | check_out_visitor | done | package_visitor_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/visitors/{id}/cancel | cancel_visitor | done | package_visitor_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/visitors/buildings/{building_id}/settings | get_visitor_settings | done | package_visitor_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| PUT | /api/v1/visitors/buildings/{building_id}/settings | update_visitor_settings | done | package_visitor_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/visitors/buildings/{building_id}/statistics | get_visitor_statistics | done | package_visitor_happy_path_tests.rs | happy-path 2xx (BIT-408) |

## reserve_funds/ (mount: /api/v1/reserve-funds)
Router assembled in `reserve_funds/mod.rs` by merging seven sub-routers (funds, schedules, transactions, policies, projections, components, alerts).
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/reserve-funds | list_funds | done | reserve_funds_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/reserve-funds | create_fund | done | reserve_funds_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/reserve-funds/dashboard | get_dashboard | done | reserve_funds_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/reserve-funds/{fund_id} | get_fund | done | reserve_funds_cross_org_idor_tests.rs | `get_fund_same_org_succeeds` asserts 200 |
| PUT | /api/v1/reserve-funds/{fund_id} | update_fund | done | reserve_funds_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/reserve-funds/{fund_id}/health | get_fund_health | done | reserve_funds_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/reserve-funds/{fund_id}/schedules | list_schedules | done | reserve_funds_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/reserve-funds/{fund_id}/schedules | create_schedule | done | reserve_funds_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| PUT | /api/v1/reserve-funds/{fund_id}/schedules/{schedule_id} | update_schedule | done | reserve_funds_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/reserve-funds/{fund_id}/transactions | list_transactions | done | reserve_funds_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/reserve-funds/{fund_id}/transactions | record_transaction | done | reserve_funds_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/reserve-funds/transfers | transfer_funds | done | reserve_funds_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/reserve-funds/{fund_id}/policies | list_policies | done | reserve_funds_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/reserve-funds/{fund_id}/policies | create_policy | done | reserve_funds_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/reserve-funds/{fund_id}/policies/active | get_active_policy | done | reserve_funds_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/reserve-funds/{fund_id}/projections | create_projection | done | reserve_funds_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/reserve-funds/{fund_id}/projections/current | get_current_projection | done | reserve_funds_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/reserve-funds/{fund_id}/projections/{projection_id}/items | get_projection_items | done | reserve_funds_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/reserve-funds/{fund_id}/projections/{projection_id}/items | add_projection_items | done | reserve_funds_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/reserve-funds/{fund_id}/components | list_components | done | reserve_funds_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/reserve-funds/{fund_id}/components | create_component | done | reserve_funds_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| PUT | /api/v1/reserve-funds/{fund_id}/components/{component_id} | update_component | done | reserve_funds_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| GET | /api/v1/reserve-funds/alerts | list_alerts | done | reserve_funds_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/reserve-funds/alerts/{alert_id}/acknowledge | acknowledge_alert | done | reserve_funds_happy_path_tests.rs | happy-path 2xx (BIT-408) |
| POST | /api/v1/reserve-funds/alerts/{alert_id}/resolve | resolve_alert | done | reserve_funds_happy_path_tests.rs | happy-path 2xx (BIT-408) |

## Summary
- done: 200 | partial: 19 | stub: 0 | missing: 0 | total: 219
