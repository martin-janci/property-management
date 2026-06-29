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
| GET | /api/v1/board-meetings/dashboard | get_dashboard | partial | — | no test |
| GET | /api/v1/board-meetings/members | list_board_members | partial | — | no test |
| POST | /api/v1/board-meetings/members | create_board_member | partial | — | no test |
| GET | /api/v1/board-meetings/members/{id} | get_board_member | partial | board_meetings_auth_tests.rs | authz-only |
| PUT | /api/v1/board-meetings/members/{id} | update_board_member | partial | board_meetings_auth_tests.rs | authz-only |
| DELETE | /api/v1/board-meetings/members/{id} | delete_board_member | partial | board_meetings_auth_tests.rs | authz-only |
| GET | /api/v1/board-meetings/members/{id}/attendance | get_member_attendance | partial | board_meetings_auth_tests.rs | authz-only |
| GET | /api/v1/board-meetings | list_meetings | partial | board_meetings_auth_tests.rs | authz-only |
| POST | /api/v1/board-meetings | create_meeting | partial | board_meetings_auth_tests.rs | authz-only |
| GET | /api/v1/board-meetings/{id} | get_meeting | partial | board_meetings_auth_tests.rs, board_meetings_cross_org_idor_tests.rs | only authz + IDOR-reject (asserts !=OK) |
| PUT | /api/v1/board-meetings/{id} | update_meeting | partial | board_meetings_auth_tests.rs, board_meetings_cross_org_idor_tests.rs | authz + IDOR-reject |
| DELETE | /api/v1/board-meetings/{id} | delete_meeting | partial | board_meetings_auth_tests.rs, board_meetings_cross_org_idor_tests.rs | authz + IDOR-reject |
| POST | /api/v1/board-meetings/{id}/start | start_meeting | partial | board_meetings_auth_tests.rs | authz-only |
| POST | /api/v1/board-meetings/{id}/end | end_meeting | partial | — | no test |
| POST | /api/v1/board-meetings/{id}/cancel | cancel_meeting | partial | — | no test |
| GET | /api/v1/board-meetings/{meeting_id}/agenda | list_agenda_items | partial | — | no test |
| POST | /api/v1/board-meetings/{meeting_id}/agenda | add_agenda_item | partial | board_meetings_cross_org_idor_tests.rs | IDOR-reject only |
| GET | /api/v1/board-meetings/agenda/{id} | get_agenda_item | partial | board_meetings_cross_org_idor_tests.rs | IDOR-reject only |
| PUT | /api/v1/board-meetings/agenda/{id} | update_agenda_item | partial | — | no test |
| POST | /api/v1/board-meetings/agenda/{id}/complete | complete_agenda_item | partial | — | no test |
| DELETE | /api/v1/board-meetings/agenda/{id} | delete_agenda_item | partial | — | no test |
| GET | /api/v1/board-meetings/{meeting_id}/motions | list_meeting_motions | partial | — | no test |
| POST | /api/v1/board-meetings/{meeting_id}/motions | create_motion | partial | — | no test |
| GET | /api/v1/board-meetings/motions | list_motions | partial | — | no test |
| GET | /api/v1/board-meetings/motions/{id} | get_motion | partial | — | no test |
| PUT | /api/v1/board-meetings/motions/{id} | update_motion | partial | — | no test |
| POST | /api/v1/board-meetings/motions/{id}/second | second_motion | partial | — | no test |
| POST | /api/v1/board-meetings/motions/{id}/start-voting | start_motion_voting | partial | board_meetings_cross_org_idor_tests.rs | IDOR-reject only |
| POST | /api/v1/board-meetings/motions/{id}/end-voting | end_motion_voting | partial | — | no test |
| POST | /api/v1/board-meetings/motions/{id}/vote | cast_vote | partial | — | no test |
| DELETE | /api/v1/board-meetings/motions/{id} | delete_motion | partial | — | no test |
| GET | /api/v1/board-meetings/{meeting_id}/attendance | list_attendance | partial | — | no test |
| POST | /api/v1/board-meetings/{meeting_id}/attendance | record_attendance | partial | — | no test |
| GET | /api/v1/board-meetings/{meeting_id}/minutes | get_minutes | partial | — | no test |
| POST | /api/v1/board-meetings/{meeting_id}/minutes | create_minutes | partial | — | no test |
| PUT | /api/v1/board-meetings/minutes/{id} | update_minutes | partial | — | no test |
| POST | /api/v1/board-meetings/minutes/{id}/approve | approve_minutes | partial | — | no test |
| GET | /api/v1/board-meetings/{meeting_id}/actions | list_meeting_action_items | partial | — | no test |
| POST | /api/v1/board-meetings/{meeting_id}/actions | create_action_item | partial | — | no test |
| GET | /api/v1/board-meetings/actions | list_action_items | partial | — | no test |
| GET | /api/v1/board-meetings/actions/{id} | get_action_item | partial | — | no test |
| PUT | /api/v1/board-meetings/actions/{id} | update_action_item | partial | — | no test |
| POST | /api/v1/board-meetings/actions/{id}/complete | complete_action_item | partial | — | no test |
| DELETE | /api/v1/board-meetings/actions/{id} | delete_action_item | partial | — | no test |
| GET | /api/v1/board-meetings/{meeting_id}/documents | list_documents | partial | — | no test |
| POST | /api/v1/board-meetings/{meeting_id}/documents | upload_document | partial | — | no test |
| DELETE | /api/v1/board-meetings/documents/{id} | delete_document | partial | — | no test |
| GET | /api/v1/board-meetings/statistics/types | get_meeting_type_counts | partial | — | no test |
| GET | /api/v1/board-meetings/statistics/motions | get_motion_status_counts | partial | — | no test |

## delegations.rs  (mount: /api/v1/delegations)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/delegations | create_delegation | partial | — | no test |
| GET | /api/v1/delegations | list_my_delegations | partial | — | no test |
| GET | /api/v1/delegations/received | list_received_delegations | partial | — | no test |
| GET | /api/v1/delegations/{id} | get_delegation | partial | — | no test |
| POST | /api/v1/delegations/{id}/accept | accept_delegation | partial | — | no test |
| POST | /api/v1/delegations/{id}/decline | decline_delegation | partial | — | no test |
| DELETE | /api/v1/delegations/{id}/revoke | revoke_delegation | partial | — | no test |
| GET | /api/v1/delegations/check/{unit_id}/{scope} | check_delegation | partial | — | no test |

## violations.rs  (mount: /api/v1/violations)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/violations/rules | list_rules | partial | — | no test |
| POST | /api/v1/violations/rules | create_rule | partial | — | no test |
| GET | /api/v1/violations/rules/{rule_id} | get_rule | partial | — | no test |
| PUT | /api/v1/violations/rules/{rule_id} | update_rule | partial | — | no test |
| DELETE | /api/v1/violations/rules/{rule_id} | delete_rule | partial | — | no test |
| GET | /api/v1/violations | list_violations | partial | — | no test |
| POST | /api/v1/violations | create_violation | partial | — | no test |
| GET | /api/v1/violations/{violation_id} | get_violation | done | violations_cross_org_idor_tests.rs | `get_violation_same_org_succeeds` asserts 200 |
| PUT | /api/v1/violations/{violation_id} | update_violation | partial | — | no test |
| POST | /api/v1/violations/{violation_id}/assign | assign_violation | partial | — | no test |
| GET | /api/v1/violations/{violation_id}/evidence | list_evidence | partial | — | no test |
| POST | /api/v1/violations/{violation_id}/evidence | add_evidence | partial | — | no test |
| DELETE | /api/v1/violations/{violation_id}/evidence/{evidence_id} | delete_evidence | partial | — | no test |
| GET | /api/v1/violations/{violation_id}/actions | list_actions | partial | — | no test |
| POST | /api/v1/violations/{violation_id}/actions | create_action | partial | — | no test |
| GET | /api/v1/violations/{violation_id}/actions/{action_id} | get_action | partial | — | no test |
| PUT | /api/v1/violations/{violation_id}/actions/{action_id} | update_action | partial | — | no test |
| POST | /api/v1/violations/{violation_id}/actions/{action_id}/send | mark_action_sent | partial | — | no test |
| GET | /api/v1/violations/{violation_id}/actions/{action_id}/payments | list_payments | partial | — | no test |
| POST | /api/v1/violations/{violation_id}/actions/{action_id}/payments | record_payment | partial | — | no test |
| POST | /api/v1/violations/{violation_id}/appeals | create_appeal | partial | — | no test |
| GET | /api/v1/violations/appeals | list_appeals | partial | — | no test |
| GET | /api/v1/violations/appeals/{appeal_id} | get_appeal | partial | — | no test |
| PUT | /api/v1/violations/appeals/{appeal_id} | update_appeal | partial | — | no test |
| POST | /api/v1/violations/appeals/{appeal_id}/decide | decide_appeal | partial | — | no test |
| GET | /api/v1/violations/{violation_id}/comments | list_comments | partial | — | no test |
| POST | /api/v1/violations/{violation_id}/comments | add_comment | partial | — | no test |
| GET | /api/v1/violations/dashboard | get_dashboard | partial | — | no test |
| GET | /api/v1/violations/history | get_violator_history | partial | — | no test |
| GET | /api/v1/violations/statistics | get_statistics | partial | — | no test |

## disputes.rs  (mount: /api/v1/disputes)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/disputes | file_dispute | partial | — | no test |
| GET | /api/v1/disputes | list_disputes | partial | — | no test |
| GET | /api/v1/disputes/statistics | get_statistics | partial | — | no test |
| GET | /api/v1/disputes/{id} | get_dispute | partial | — | no test |
| PATCH | /api/v1/disputes/{id} | update_dispute_status | partial | — | no test |
| DELETE | /api/v1/disputes/{id} | withdraw_dispute | partial | — | no test |
| PATCH | /api/v1/disputes/{id}/resolve | resolve_dispute | partial | dispute_cross_org_idor_tests.rs | IDOR-reject only |
| PATCH | /api/v1/disputes/{id}/mediation-notes | update_mediation_notes | partial | dispute_cross_org_idor_tests.rs | IDOR-reject only |
| GET | /api/v1/disputes/{id}/parties | list_parties | partial | — | no test |
| POST | /api/v1/disputes/{id}/parties | add_party | partial | — | no test |
| GET | /api/v1/disputes/{id}/evidence | list_evidence | partial | dispute_evidence_auth_tests.rs | authz-only |
| POST | /api/v1/disputes/{id}/evidence | add_evidence | partial | dispute_evidence_auth_tests.rs | authz-only |
| DELETE | /api/v1/disputes/{id}/evidence/{evidence_id} | delete_evidence | partial | — | no test |
| GET | /api/v1/disputes/{id}/activities | list_activities | partial | — | no test |
| GET | /api/v1/disputes/{id}/sessions | list_sessions | partial | — | no test |
| POST | /api/v1/disputes/{id}/sessions | schedule_session | partial | — | no test |
| GET | /api/v1/disputes/{id}/sessions/{session_id} | get_session | partial | — | no test |
| PATCH | /api/v1/disputes/{id}/sessions/{session_id} | update_session | partial | — | no test |
| POST | /api/v1/disputes/{id}/sessions/{session_id}/cancel | cancel_session | partial | — | no test |
| GET | /api/v1/disputes/{id}/sessions/{session_id}/attendance | get_attendance | partial | — | no test |
| PATCH | /api/v1/disputes/{id}/sessions/{session_id}/attendance/{party_id} | update_attendance | partial | — | no test |
| POST | /api/v1/disputes/{id}/sessions/{session_id}/notes | record_notes | partial | — | no test |
| GET | /api/v1/disputes/{id}/submissions | list_submissions | partial | — | no test |
| POST | /api/v1/disputes/{id}/submissions | submit_response | partial | — | no test |
| GET | /api/v1/disputes/{id}/mediation-case | get_mediation_case | partial | — | no test |
| GET | /api/v1/disputes/{id}/resolutions | list_resolutions | partial | — | no test |
| POST | /api/v1/disputes/{id}/resolutions | propose_resolution | partial | — | no test |
| GET | /api/v1/disputes/{id}/resolutions/{resolution_id} | get_resolution | partial | — | no test |
| POST | /api/v1/disputes/{id}/resolutions/{resolution_id}/vote | vote_on_resolution | partial | — | no test |
| POST | /api/v1/disputes/{id}/resolutions/{resolution_id}/accept | accept_resolution | partial | — | no test |
| POST | /api/v1/disputes/{id}/resolutions/{resolution_id}/implement | implement_resolution | partial | — | no test |
| GET | /api/v1/disputes/{id}/actions | list_action_items | partial | — | no test |
| POST | /api/v1/disputes/{id}/actions | create_action_item | partial | — | no test |
| GET | /api/v1/disputes/{id}/actions/{action_id} | get_action_item | partial | — | no test |
| PATCH | /api/v1/disputes/{id}/actions/{action_id} | update_action_item | partial | — | no test |
| POST | /api/v1/disputes/{id}/actions/{action_id}/complete | complete_action_item | partial | — | no test |
| POST | /api/v1/disputes/{id}/actions/{action_id}/remind | send_action_reminder | partial | — | no test |
| GET | /api/v1/disputes/{id}/escalations | list_escalations | partial | — | no test |
| POST | /api/v1/disputes/{id}/escalations | create_escalation | partial | — | no test |
| POST | /api/v1/disputes/{id}/escalations/{escalation_id}/resolve | resolve_escalation | partial | — | no test |
| GET | /api/v1/disputes/my-actions | get_my_actions | partial | — | no test |
| GET | /api/v1/disputes/overdue-actions | list_overdue_actions | partial | — | no test |

## community.rs  (mount: /api/v1/community)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/community/buildings/{building_id}/groups | list_groups | partial | — | no test |
| POST | /api/v1/community/buildings/{building_id}/groups | create_group | partial | — | no test |
| GET | /api/v1/community/groups/{id} | get_group | partial | — | no test |
| POST | /api/v1/community/groups/{id}/join | join_group | partial | — | no test |
| POST | /api/v1/community/groups/{id}/leave | leave_group | partial | — | no test |
| GET | /api/v1/community/groups/{group_id}/posts | list_posts | partial | — | no test |
| POST | /api/v1/community/groups/{group_id}/posts | create_post | partial | — | no test |
| POST | /api/v1/community/posts/{id}/reactions | add_reaction | partial | — | no test |
| POST | /api/v1/community/posts/{id}/comments | create_comment | partial | — | no test |
| GET | /api/v1/community/buildings/{building_id}/events | list_events | partial | — | no test |
| POST | /api/v1/community/buildings/{building_id}/events | create_event | partial | — | no test |
| POST | /api/v1/community/events/{id}/rsvp | rsvp_event | partial | — | no test |
| GET | /api/v1/community/buildings/{building_id}/marketplace | list_items | partial | — | no test |
| POST | /api/v1/community/buildings/{building_id}/marketplace | create_item | partial | — | no test |
| GET | /api/v1/community/marketplace/{id} | get_item | partial | — | no test |
| POST | /api/v1/community/marketplace/{id}/inquiries | create_inquiry | partial | — | no test |

## neighbors.rs  (mount: /api/v1)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/buildings/{building_id}/neighbors | list_neighbors | partial | — | no test |
| GET | /api/v1/users/me/privacy | get_privacy_settings | partial | — | no test |
| PUT | /api/v1/users/me/privacy | update_privacy_settings | partial | — | no test |

## package_visitor.rs — packages (mount: /api/v1/packages)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/packages | create_package | partial | — | no test |
| GET | /api/v1/packages | list_packages | partial | — | no test |
| GET | /api/v1/packages/{id} | get_package | partial | — | no test |
| PUT | /api/v1/packages/{id} | update_package | partial | — | no test |
| DELETE | /api/v1/packages/{id} | delete_package | partial | — | no test |
| POST | /api/v1/packages/{id}/receive | receive_package | partial | — | no test |
| POST | /api/v1/packages/{id}/pickup | pickup_package | partial | — | no test |
| GET | /api/v1/packages/buildings/{building_id}/settings | get_package_settings | partial | — | no test |
| PUT | /api/v1/packages/buildings/{building_id}/settings | update_package_settings | partial | — | no test |
| GET | /api/v1/packages/buildings/{building_id}/statistics | get_package_statistics | partial | — | no test |

## package_visitor.rs — visitors (mount: /api/v1/visitors)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/visitors | create_visitor | partial | — | no test |
| GET | /api/v1/visitors | list_visitors | partial | — | no test |
| POST | /api/v1/visitors/verify-code | verify_access_code | partial | — | no test |
| GET | /api/v1/visitors/{id} | get_visitor | partial | — | no test |
| PUT | /api/v1/visitors/{id} | update_visitor | partial | — | no test |
| DELETE | /api/v1/visitors/{id} | delete_visitor | partial | — | no test |
| POST | /api/v1/visitors/{id}/check-in | check_in_visitor | partial | — | no test |
| POST | /api/v1/visitors/{id}/check-out | check_out_visitor | partial | — | no test |
| POST | /api/v1/visitors/{id}/cancel | cancel_visitor | partial | — | no test |
| GET | /api/v1/visitors/buildings/{building_id}/settings | get_visitor_settings | partial | — | no test |
| PUT | /api/v1/visitors/buildings/{building_id}/settings | update_visitor_settings | partial | — | no test |
| GET | /api/v1/visitors/buildings/{building_id}/statistics | get_visitor_statistics | partial | — | no test |

## reserve_funds/ (mount: /api/v1/reserve-funds)
Router assembled in `reserve_funds/mod.rs` by merging seven sub-routers (funds, schedules, transactions, policies, projections, components, alerts).
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/reserve-funds | list_funds | partial | — | funds.rs; no test |
| POST | /api/v1/reserve-funds | create_fund | partial | — | funds.rs; no test |
| GET | /api/v1/reserve-funds/dashboard | get_dashboard | partial | — | funds.rs; no test |
| GET | /api/v1/reserve-funds/{fund_id} | get_fund | done | reserve_funds_cross_org_idor_tests.rs | `get_fund_same_org_succeeds` asserts 200 |
| PUT | /api/v1/reserve-funds/{fund_id} | update_fund | partial | reserve_funds_cross_org_idor_tests.rs | IDOR-reject only (404) |
| GET | /api/v1/reserve-funds/{fund_id}/health | get_fund_health | partial | — | funds.rs; no test |
| GET | /api/v1/reserve-funds/{fund_id}/schedules | list_schedules | partial | — | schedules.rs; no test |
| POST | /api/v1/reserve-funds/{fund_id}/schedules | create_schedule | partial | — | schedules.rs; no test |
| PUT | /api/v1/reserve-funds/{fund_id}/schedules/{schedule_id} | update_schedule | partial | — | schedules.rs; no test |
| GET | /api/v1/reserve-funds/{fund_id}/transactions | list_transactions | partial | — | transactions.rs; no test |
| POST | /api/v1/reserve-funds/{fund_id}/transactions | record_transaction | partial | — | transactions.rs; no test |
| POST | /api/v1/reserve-funds/transfers | transfer_funds | partial | — | transactions.rs; no test |
| GET | /api/v1/reserve-funds/{fund_id}/policies | list_policies | partial | — | policies.rs; no test |
| POST | /api/v1/reserve-funds/{fund_id}/policies | create_policy | partial | — | policies.rs; no test |
| GET | /api/v1/reserve-funds/{fund_id}/policies/active | get_active_policy | partial | — | policies.rs; no test |
| POST | /api/v1/reserve-funds/{fund_id}/projections | create_projection | partial | — | projections.rs; no test |
| GET | /api/v1/reserve-funds/{fund_id}/projections/current | get_current_projection | partial | — | projections.rs; no test |
| GET | /api/v1/reserve-funds/{fund_id}/projections/{projection_id}/items | get_projection_items | partial | — | projections.rs; no test |
| POST | /api/v1/reserve-funds/{fund_id}/projections/{projection_id}/items | add_projection_items | partial | — | projections.rs; no test |
| GET | /api/v1/reserve-funds/{fund_id}/components | list_components | partial | — | components.rs; no test |
| POST | /api/v1/reserve-funds/{fund_id}/components | create_component | partial | — | components.rs; no test |
| PUT | /api/v1/reserve-funds/{fund_id}/components/{component_id} | update_component | partial | — | components.rs; no test |
| GET | /api/v1/reserve-funds/alerts | list_alerts | partial | — | alerts.rs; no test |
| POST | /api/v1/reserve-funds/alerts/{alert_id}/acknowledge | acknowledge_alert | partial | — | alerts.rs; no test |
| POST | /api/v1/reserve-funds/alerts/{alert_id}/resolve | resolve_alert | partial | — | alerts.rs; no test |

## Summary
- done: 9 | partial: 210 | stub: 0 | missing: 0 | total: 219
