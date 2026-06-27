# Analytics & Portals

_Server: api-server. Modules: owner_analytics, portfolio_analytics, portfolio_performance, investor_portal, government_portal, esg_reporting._

## owner_analytics.rs  (mount: /api/v1/owner-analytics)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/owner-analytics/units/{unit_id}/valuation | get_unit_valuation | done | owner_analytics_cross_org_idor_tests.rs | T3 happy-path OK (same-org); T1/T4 are IDOR/401 |
| POST | /api/v1/owner-analytics/units/{unit_id}/valuation | create_valuation | done | analytics_owner_success_tests.rs | |
| GET | /api/v1/owner-analytics/valuations/{valuation_id} | get_valuation_with_comparables | done | analytics_owner_success_tests.rs | |
| POST | /api/v1/owner-analytics/valuations/{valuation_id}/comparables | add_comparable | done | analytics_owner_success_tests.rs | |
| GET | /api/v1/owner-analytics/units/{unit_id}/value-history | get_value_history | done | analytics_owner_success_tests.rs | |
| GET | /api/v1/owner-analytics/units/{unit_id}/value-trend | get_value_trend | done | analytics_owner_success_tests.rs | |
| POST | /api/v1/owner-analytics/units/{unit_id}/roi | calculate_roi | done | analytics_owner_success_tests.rs | |
| GET | /api/v1/owner-analytics/units/{unit_id}/cash-flow | get_cash_flow_breakdown | done | analytics_owner_success_tests.rs | |
| GET | /api/v1/owner-analytics/units/{unit_id}/roi-dashboard | get_roi_dashboard | done | analytics_owner_success_tests.rs | |
| GET | /api/v1/owner-analytics/portfolio | get_portfolio_summary | done | analytics_owner_success_tests.rs | |
| POST | /api/v1/owner-analytics/portfolio/compare | compare_properties | done | analytics_owner_success_tests.rs | |
| GET | /api/v1/owner-analytics/expense-rules | list_auto_approval_rules | done | analytics_owner_success_tests.rs | |
| POST | /api/v1/owner-analytics/expense-rules | create_auto_approval_rule | done | analytics_owner_success_tests.rs | |
| PUT | /api/v1/owner-analytics/expense-rules/{id} | update_auto_approval_rule | done | analytics_owner_success_tests.rs | |
| DELETE | /api/v1/owner-analytics/expense-rules/{id} | delete_auto_approval_rule | done | analytics_owner_success_tests.rs | |
| POST | /api/v1/owner-analytics/expenses/submit | submit_expense | done | analytics_owner_success_tests.rs | |
| GET | /api/v1/owner-analytics/expenses | list_expense_requests | done | analytics_owner_success_tests.rs | |
| POST | /api/v1/owner-analytics/expenses/{id}/review | review_expense | done | analytics_owner_success_tests.rs | |

## portfolio_analytics.rs  (mount: /api/v1/portfolio-analytics)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/portfolio-analytics/summary | get_portfolio_summary | done | analytics_portfolio_success_tests.rs | no test file references this prefix |
| GET | /api/v1/portfolio-analytics/benchmarks | list_benchmarks | done | analytics_portfolio_success_tests.rs | |
| POST | /api/v1/portfolio-analytics/benchmarks | create_benchmark | done | analytics_portfolio_success_tests.rs | |
| GET | /api/v1/portfolio-analytics/benchmarks/{id} | get_benchmark | done | analytics_portfolio_success_tests.rs | |
| PUT | /api/v1/portfolio-analytics/benchmarks/{id} | update_benchmark | done | analytics_portfolio_success_tests.rs | |
| DELETE | /api/v1/portfolio-analytics/benchmarks/{id} | delete_benchmark | done | analytics_portfolio_success_tests.rs | |
| GET | /api/v1/portfolio-analytics/properties/metrics | list_property_metrics | done | analytics_portfolio_success_tests.rs | |
| POST | /api/v1/portfolio-analytics/properties/metrics | upsert_property_metrics | done | analytics_portfolio_success_tests.rs | |
| GET | /api/v1/portfolio-analytics/properties/{building_id}/metrics | get_property_metrics | done | analytics_portfolio_success_tests.rs | |
| GET | /api/v1/portfolio-analytics/metrics | get_portfolio_metrics | done | analytics_portfolio_success_tests.rs | |
| POST | /api/v1/portfolio-analytics/metrics/calculate | calculate_portfolio_metrics | done | analytics_portfolio_success_tests.rs | |
| GET | /api/v1/portfolio-analytics/comparisons | list_comparisons | done | analytics_portfolio_success_tests.rs | |
| POST | /api/v1/portfolio-analytics/comparisons | create_comparison | done | analytics_portfolio_success_tests.rs | |
| GET | /api/v1/portfolio-analytics/comparisons/{id} | get_comparison | done | analytics_portfolio_success_tests.rs | |
| DELETE | /api/v1/portfolio-analytics/comparisons/{id} | delete_comparison | done | analytics_portfolio_success_tests.rs | |
| GET | /api/v1/portfolio-analytics/trends | get_trends | done | analytics_portfolio_success_tests.rs | |
| POST | /api/v1/portfolio-analytics/trends | record_trend | done | analytics_portfolio_success_tests.rs | |
| GET | /api/v1/portfolio-analytics/alerts/rules | list_alert_rules | done | analytics_portfolio_success_tests.rs | |
| POST | /api/v1/portfolio-analytics/alerts/rules | create_alert_rule | done | analytics_portfolio_success_tests.rs | |
| GET | /api/v1/portfolio-analytics/alerts/rules/{id} | get_alert_rule | done | analytics_portfolio_success_tests.rs | |
| PUT | /api/v1/portfolio-analytics/alerts/rules/{id} | update_alert_rule | done | analytics_portfolio_success_tests.rs | |
| DELETE | /api/v1/portfolio-analytics/alerts/rules/{id} | delete_alert_rule | done | analytics_portfolio_success_tests.rs | |
| GET | /api/v1/portfolio-analytics/alerts | list_alerts | done | analytics_portfolio_success_tests.rs | |
| GET | /api/v1/portfolio-analytics/alerts/stats | get_alert_stats | done | analytics_portfolio_success_tests.rs | |
| GET | /api/v1/portfolio-analytics/alerts/{id} | get_alert | done | analytics_portfolio_success_tests.rs | |
| POST | /api/v1/portfolio-analytics/alerts/{id}/acknowledge | acknowledge_alert | done | analytics_portfolio_success_tests.rs | |
| POST | /api/v1/portfolio-analytics/alerts/{id}/resolve | resolve_alert | done | analytics_portfolio_success_tests.rs | |

## portfolio_performance.rs  (mount: /api/v1/portfolio-performance)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/portfolio-performance/portfolios | create_portfolio | done | analytics_portfolio_success_tests.rs | router_single_source_tests.rs references prefix only in a comment (route-table check), no handler path exercised |
| GET | /api/v1/portfolio-performance/portfolios | list_portfolios | done | analytics_portfolio_success_tests.rs | |
| GET | /api/v1/portfolio-performance/portfolios/{id} | get_portfolio | done | analytics_portfolio_success_tests.rs | |
| PUT | /api/v1/portfolio-performance/portfolios/{id} | update_portfolio | done | analytics_portfolio_success_tests.rs | |
| DELETE | /api/v1/portfolio-performance/portfolios/{id} | delete_portfolio | done | analytics_portfolio_success_tests.rs | |
| POST | /api/v1/portfolio-performance/portfolios/{id}/properties | add_property | done | analytics_portfolio_success_tests.rs | |
| GET | /api/v1/portfolio-performance/portfolios/{id}/properties | list_properties | done | analytics_portfolio_success_tests.rs | |
| GET | /api/v1/portfolio-performance/portfolios/{id}/properties/{property_id} | get_property | done | analytics_portfolio_success_tests.rs | |
| PUT | /api/v1/portfolio-performance/portfolios/{id}/properties/{property_id} | update_property | done | analytics_portfolio_success_tests.rs | |
| DELETE | /api/v1/portfolio-performance/portfolios/{id}/properties/{property_id} | remove_property | done | analytics_portfolio_success_tests.rs | |
| POST | /api/v1/portfolio-performance/portfolios/{id}/transactions | create_transaction | done | analytics_portfolio_success_tests.rs | |
| GET | /api/v1/portfolio-performance/portfolios/{id}/transactions | list_transactions | done | analytics_portfolio_success_tests.rs | |
| GET | /api/v1/portfolio-performance/portfolios/{id}/transactions/{transaction_id} | get_transaction | done | analytics_portfolio_success_tests.rs | |
| PUT | /api/v1/portfolio-performance/portfolios/{id}/transactions/{transaction_id} | update_transaction | done | analytics_portfolio_success_tests.rs | |
| DELETE | /api/v1/portfolio-performance/portfolios/{id}/transactions/{transaction_id} | delete_transaction | done | analytics_portfolio_success_tests.rs | |
| POST | /api/v1/portfolio-performance/portfolios/{id}/cash-flows | upsert_cash_flow | done | analytics_portfolio_success_tests.rs | |
| GET | /api/v1/portfolio-performance/portfolios/{id}/cash-flows | get_cash_flows | done | analytics_portfolio_success_tests.rs | |
| POST | /api/v1/portfolio-performance/portfolios/{id}/metrics/calculate | calculate_metrics | done | analytics_portfolio_success_tests.rs | |
| GET | /api/v1/portfolio-performance/portfolios/{id}/metrics/latest | get_latest_metrics | done | analytics_portfolio_success_tests.rs | |
| GET | /api/v1/portfolio-performance/portfolios/{id}/metrics/summary | get_metrics_summary | done | analytics_portfolio_success_tests.rs | |
| POST | /api/v1/portfolio-performance/benchmarks | create_benchmark | done | analytics_portfolio_success_tests.rs | |
| GET | /api/v1/portfolio-performance/benchmarks | list_benchmarks | done | analytics_portfolio_success_tests.rs | |
| GET | /api/v1/portfolio-performance/benchmarks/{id} | get_benchmark | done | analytics_portfolio_success_tests.rs | |
| PUT | /api/v1/portfolio-performance/benchmarks/{id} | update_benchmark | done | analytics_portfolio_success_tests.rs | |
| DELETE | /api/v1/portfolio-performance/benchmarks/{id} | delete_benchmark | done | analytics_portfolio_success_tests.rs | |
| POST | /api/v1/portfolio-performance/portfolios/{id}/comparisons | create_comparison | done | analytics_portfolio_success_tests.rs | |
| GET | /api/v1/portfolio-performance/portfolios/{id}/comparisons | list_comparisons | done | analytics_portfolio_success_tests.rs | |
| GET | /api/v1/portfolio-performance/portfolios/{id}/comparisons/{comparison_id} | get_comparison | done | analytics_portfolio_success_tests.rs | |
| GET | /api/v1/portfolio-performance/portfolios/{id}/dashboard/summary | get_dashboard_summary | done | analytics_portfolio_success_tests.rs | |
| GET | /api/v1/portfolio-performance/portfolios/{id}/dashboard/property-cards | get_property_cards | done | analytics_portfolio_success_tests.rs | |
| GET | /api/v1/portfolio-performance/portfolios/{id}/dashboard/cash-flow-trend | get_cash_flow_trend | done | analytics_portfolio_success_tests.rs | |
| POST | /api/v1/portfolio-performance/portfolios/{id}/alerts | create_alert | done | analytics_portfolio_success_tests.rs | |
| GET | /api/v1/portfolio-performance/portfolios/{id}/alerts | list_alerts | done | analytics_portfolio_success_tests.rs | |
| POST | /api/v1/portfolio-performance/portfolios/{id}/alerts/{alert_id}/read | mark_alert_read | done | analytics_portfolio_success_tests.rs | |
| POST | /api/v1/portfolio-performance/portfolios/{id}/alerts/{alert_id}/resolve | resolve_alert | done | analytics_portfolio_success_tests.rs | |

## investor_portal.rs  (mount: /api/v1/investor-portal)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/investor-portal/investors | list_investors | done | analytics_investor_portal_success_tests.rs | |
| POST | /api/v1/investor-portal/investors | create_investor | done | analytics_investor_portal_success_tests.rs | |
| GET | /api/v1/investor-portal/investors/{investor_id} | get_investor | done | analytics_investor_portal_success_tests.rs | |
| PUT | /api/v1/investor-portal/investors/{investor_id} | update_investor | done | analytics_investor_portal_success_tests.rs | |
| DELETE | /api/v1/investor-portal/investors/{investor_id} | delete_investor | done | analytics_investor_portal_success_tests.rs | |
| GET | /api/v1/investor-portal/investors/{investor_id}/summary | get_investor_summary | done | analytics_investor_portal_success_tests.rs | |
| GET | /api/v1/investor-portal/portfolios | list_portfolios | done | analytics_investor_portal_success_tests.rs | |
| POST | /api/v1/investor-portal/portfolios | create_portfolio | done | analytics_investor_portal_success_tests.rs | |
| GET | /api/v1/investor-portal/portfolios/{portfolio_id} | get_portfolio | done | analytics_investor_portal_success_tests.rs | |
| PUT | /api/v1/investor-portal/portfolios/{portfolio_id} | update_portfolio | done | analytics_investor_portal_success_tests.rs | |
| DELETE | /api/v1/investor-portal/portfolios/{portfolio_id} | delete_portfolio | done | analytics_investor_portal_success_tests.rs | |
| GET | /api/v1/investor-portal/investors/{investor_id}/portfolios | list_investor_portfolios | done | analytics_investor_portal_success_tests.rs | |
| GET | /api/v1/investor-portal/portfolios/{portfolio_id}/properties | list_portfolio_properties | done | marketplace_voting_investor_cross_org_idor_tests.rs | own-org happy-path OK (list_portfolio_properties_for_own_org_succeeds) |
| POST | /api/v1/investor-portal/portfolios/{portfolio_id}/properties | add_portfolio_property | done | analytics_investor_portal_success_tests.rs | |
| PUT | /api/v1/investor-portal/portfolios/{portfolio_id}/properties/{property_id} | update_portfolio_property | done | analytics_investor_portal_success_tests.rs | |
| DELETE | /api/v1/investor-portal/portfolios/{portfolio_id}/properties/{property_id} | remove_portfolio_property | done | analytics_investor_portal_success_tests.rs | |
| GET | /api/v1/investor-portal/roi | list_roi_calculations | done | analytics_investor_portal_success_tests.rs | |
| POST | /api/v1/investor-portal/roi | create_roi_calculation | done | analytics_investor_portal_success_tests.rs | |
| GET | /api/v1/investor-portal/portfolios/{portfolio_id}/roi/latest | get_latest_roi | done | analytics_investor_portal_success_tests.rs | |
| POST | /api/v1/investor-portal/distributions | create_distribution | done | analytics_investor_portal_success_tests.rs | |
| GET | /api/v1/investor-portal/investors/{investor_id}/distributions | list_investor_distributions | done | analytics_investor_portal_success_tests.rs | |
| PUT | /api/v1/investor-portal/distributions/{distribution_id} | update_distribution | done | analytics_investor_portal_success_tests.rs | |
| POST | /api/v1/investor-portal/reports | create_report | done | analytics_investor_portal_success_tests.rs | |
| GET | /api/v1/investor-portal/investors/{investor_id}/reports | list_investor_reports | done | analytics_investor_portal_success_tests.rs | |
| GET | /api/v1/investor-portal/reports/{report_id} | get_report | done | analytics_investor_portal_success_tests.rs | |
| POST | /api/v1/investor-portal/capital-calls | create_capital_call | done | analytics_investor_portal_success_tests.rs | |
| GET | /api/v1/investor-portal/investors/{investor_id}/capital-calls | list_investor_capital_calls | done | analytics_investor_portal_success_tests.rs | |
| PUT | /api/v1/investor-portal/capital-calls/{call_id} | update_capital_call | done | analytics_investor_portal_success_tests.rs | |
| GET | /api/v1/investor-portal/dashboard/{investor_id} | get_investor_dashboard | done | analytics_investor_portal_success_tests.rs | |
| POST | /api/v1/investor-portal/dashboard/{investor_id}/metrics | upsert_dashboard_metrics | done | analytics_investor_portal_success_tests.rs | |

## government_portal.rs  (mount: /api/v1/government-portal)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/government-portal/connections | list_connections | done | analytics_gov_portal_success_tests.rs | no test file references this prefix |
| POST | /api/v1/government-portal/connections | create_connection | done | analytics_gov_portal_success_tests.rs | |
| GET | /api/v1/government-portal/connections/{id} | get_connection | done | analytics_gov_portal_success_tests.rs | |
| PUT | /api/v1/government-portal/connections/{id} | update_connection | done | analytics_gov_portal_success_tests.rs | |
| DELETE | /api/v1/government-portal/connections/{id} | delete_connection | done | analytics_gov_portal_success_tests.rs | |
| POST | /api/v1/government-portal/connections/{id}/test | test_connection | done | analytics_gov_portal_success_tests.rs | |
| GET | /api/v1/government-portal/templates | list_templates | done | analytics_gov_portal_success_tests.rs | |
| GET | /api/v1/government-portal/templates/{id} | get_template | done | analytics_gov_portal_success_tests.rs | |
| GET | /api/v1/government-portal/submissions | list_submissions | done | analytics_gov_portal_success_tests.rs | |
| POST | /api/v1/government-portal/submissions | create_submission | done | analytics_gov_portal_success_tests.rs | |
| GET | /api/v1/government-portal/submissions/{id} | get_submission | done | analytics_gov_portal_success_tests.rs | |
| PUT | /api/v1/government-portal/submissions/{id} | update_submission | done | analytics_gov_portal_success_tests.rs | |
| POST | /api/v1/government-portal/submissions/{id}/validate | validate_submission | done | analytics_gov_portal_success_tests.rs | |
| POST | /api/v1/government-portal/submissions/{id}/submit | submit_submission | done | analytics_gov_portal_success_tests.rs | |
| POST | /api/v1/government-portal/submissions/{id}/cancel | cancel_submission | done | analytics_gov_portal_success_tests.rs | |
| GET | /api/v1/government-portal/submissions/{id}/audit | get_submission_audit | done | analytics_gov_portal_success_tests.rs | |
| GET | /api/v1/government-portal/submissions/{id}/attachments | list_attachments | done | analytics_gov_portal_success_tests.rs | |
| POST | /api/v1/government-portal/submissions/{id}/attachments | add_attachment | done | analytics_gov_portal_success_tests.rs | |
| DELETE | /api/v1/government-portal/submissions/{submission_id}/attachments/{attachment_id} | delete_attachment | done | analytics_gov_portal_success_tests.rs | |
| GET | /api/v1/government-portal/schedules | list_schedules | done | analytics_gov_portal_success_tests.rs | |
| POST | /api/v1/government-portal/schedules | create_schedule | done | analytics_gov_portal_success_tests.rs | |
| GET | /api/v1/government-portal/schedules/{id} | get_schedule | done | analytics_gov_portal_success_tests.rs | |
| PUT | /api/v1/government-portal/schedules/{id} | update_schedule | done | analytics_gov_portal_success_tests.rs | |
| DELETE | /api/v1/government-portal/schedules/{id} | delete_schedule | done | analytics_gov_portal_success_tests.rs | |
| GET | /api/v1/government-portal/stats | get_stats | done | analytics_gov_portal_success_tests.rs | |

## esg_reporting.rs  (mount: /api/v1/esg)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/esg/configuration | get_configuration | done | analytics_esg_success_tests.rs | |
| POST | /api/v1/esg/configuration | upsert_configuration | done | analytics_esg_success_tests.rs | |
| GET | /api/v1/esg/metrics | list_metrics | done | analytics_esg_success_tests.rs | |
| POST | /api/v1/esg/metrics | create_metric | done | analytics_esg_success_tests.rs | |
| GET | /api/v1/esg/metrics/{id} | get_metric | done | esg_reporting_cross_org_idor_tests.rs | T9 own-org happy-path OK (get_metric_for_own_org_succeeds) |
| PUT | /api/v1/esg/metrics/{id} | update_metric | done | analytics_esg_success_tests.rs |
| POST | /api/v1/esg/metrics/{id}/verify | verify_metric | done | analytics_esg_success_tests.rs |
| POST | /api/v1/esg/metrics/{id}/delete | delete_metric | done | analytics_esg_success_tests.rs |
| GET | /api/v1/esg/carbon | list_carbon_footprints | done | analytics_esg_success_tests.rs | |
| POST | /api/v1/esg/carbon | create_carbon_footprint | done | analytics_esg_success_tests.rs | |
| GET | /api/v1/esg/carbon/summary/{year} | get_carbon_summary | done | analytics_esg_success_tests.rs | |
| GET | /api/v1/esg/carbon/{id} | get_carbon_footprint | done | analytics_esg_success_tests.rs | |
| POST | /api/v1/esg/carbon/{id}/delete | delete_carbon_footprint | done | analytics_esg_success_tests.rs | |
| GET | /api/v1/esg/benchmarks | list_benchmarks | done | analytics_esg_success_tests.rs | |
| POST | /api/v1/esg/benchmarks | create_benchmark | done | analytics_esg_success_tests.rs | |
| POST | /api/v1/esg/benchmarks/{id}/delete | delete_benchmark | done | analytics_esg_success_tests.rs | |
| GET | /api/v1/esg/targets | list_targets | done | analytics_esg_success_tests.rs | |
| POST | /api/v1/esg/targets | create_target | done | analytics_esg_success_tests.rs | |
| GET | /api/v1/esg/targets/{id} | get_target | done | analytics_esg_success_tests.rs | |
| PUT | /api/v1/esg/targets/{id} | update_target | done | analytics_esg_success_tests.rs | |
| POST | /api/v1/esg/targets/{id}/delete | delete_target | done | analytics_esg_success_tests.rs | |
| GET | /api/v1/esg/reports | list_reports | done | analytics_esg_success_tests.rs | |
| POST | /api/v1/esg/reports | create_report | done | analytics_esg_success_tests.rs | |
| GET | /api/v1/esg/reports/{id} | get_report | done | analytics_esg_success_tests.rs |
| PUT | /api/v1/esg/reports/{id} | update_report | done | analytics_esg_success_tests.rs | |
| POST | /api/v1/esg/reports/{id}/submit | submit_report | done | analytics_esg_success_tests.rs |
| POST | /api/v1/esg/reports/{id}/approve | approve_report | done | analytics_esg_success_tests.rs | |
| POST | /api/v1/esg/reports/{id}/delete | delete_report | done | analytics_esg_success_tests.rs |
| GET | /api/v1/esg/eu-taxonomy | list_eu_taxonomy_assessments | done | analytics_esg_success_tests.rs | |
| POST | /api/v1/esg/eu-taxonomy | create_eu_taxonomy_assessment | done | analytics_esg_success_tests.rs | |
| GET | /api/v1/esg/eu-taxonomy/{id} | get_eu_taxonomy_assessment | done | analytics_esg_success_tests.rs | |
| PUT | /api/v1/esg/eu-taxonomy/{id} | update_eu_taxonomy_assessment | done | analytics_esg_success_tests.rs | |
| GET | /api/v1/esg/dashboard/{year} | get_dashboard | done | analytics_esg_success_tests.rs | |
| POST | /api/v1/esg/dashboard/{year}/refresh | refresh_dashboard | done | analytics_esg_success_tests.rs | |
| GET | /api/v1/esg/imports | list_import_jobs | done | analytics_esg_success_tests.rs | |
| POST | /api/v1/esg/imports | create_import_job | done | analytics_esg_success_tests.rs | |
| GET | /api/v1/esg/imports/{id} | get_import_job | done | analytics_esg_success_tests.rs | |
| GET | /api/v1/esg/statistics | get_statistics | done | analytics_esg_success_tests.rs | |

## Summary
- done: 173 | partial: 0 | stub: 0 | missing: 0 | total: 173
