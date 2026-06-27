# Portals, Analytics & Health endpoints

Mounts (from `src/lib.rs`): `/api/v1/investor-portal`, `/api/v1/owner-analytics`, `/api/v1/portfolio-analytics`, `/api/v1/portfolio-performance`; health at root (`/health`, `/readiness`). All five routers are mounted.

| Method + Path | Handler | Status | Test | Notes |
|---|---|---|---|---|
| `GET /api/v1/investor-portal/investors` | `investor_portal.rs:list_investors` | partial | — | real repo, no test |
| `POST /api/v1/investor-portal/investors` | `investor_portal.rs:create_investor` | partial | — | real repo, no test |
| `GET /api/v1/investor-portal/investors/{investor_id}` | `investor_portal.rs:get_investor` | partial | — | real repo, no test |
| `PUT /api/v1/investor-portal/investors/{investor_id}` | `investor_portal.rs:update_investor` | partial | — | real repo, no test |
| `DELETE /api/v1/investor-portal/investors/{investor_id}` | `investor_portal.rs:delete_investor` | partial | — | real repo, no test |
| `GET /api/v1/investor-portal/investors/{investor_id}/summary` | `investor_portal.rs:get_investor_summary` | partial | — | real repo, no test |
| `GET /api/v1/investor-portal/portfolios` | `investor_portal.rs:list_portfolios` | partial | — | real repo, no test |
| `POST /api/v1/investor-portal/portfolios` | `investor_portal.rs:create_portfolio` | partial | — | real repo, no test |
| `GET /api/v1/investor-portal/portfolios/{portfolio_id}` | `investor_portal.rs:get_portfolio` | partial | — | real repo, no test |
| `PUT /api/v1/investor-portal/portfolios/{portfolio_id}` | `investor_portal.rs:update_portfolio` | partial | — | real repo, no test |
| `DELETE /api/v1/investor-portal/portfolios/{portfolio_id}` | `investor_portal.rs:delete_portfolio` | partial | — | real repo, no test |
| `GET /api/v1/investor-portal/investors/{investor_id}/portfolios` | `investor_portal.rs:list_investor_portfolios` | partial | — | real repo, no test |
| `GET /api/v1/investor-portal/portfolios/{portfolio_id}/properties` | `investor_portal.rs:list_portfolio_properties` | done | `marketplace_voting_investor_cross_org_idor_tests.rs` | cross-org + own-org asserts |
| `POST /api/v1/investor-portal/portfolios/{portfolio_id}/properties` | `investor_portal.rs:add_portfolio_property` | partial | — | real repo, no test |
| `PUT /api/v1/investor-portal/portfolios/{portfolio_id}/properties/{property_id}` | `investor_portal.rs:update_portfolio_property` | partial | — | real repo, no test |
| `DELETE /api/v1/investor-portal/portfolios/{portfolio_id}/properties/{property_id}` | `investor_portal.rs:remove_portfolio_property` | partial | — | real repo, no test |
| `GET /api/v1/investor-portal/roi` | `investor_portal.rs:list_roi_calculations` | partial | — | real repo, no test |
| `POST /api/v1/investor-portal/roi` | `investor_portal.rs:create_roi_calculation` | partial | — | real repo, no test |
| `GET /api/v1/investor-portal/portfolios/{portfolio_id}/roi/latest` | `investor_portal.rs:get_latest_roi` | partial | — | real repo, no test |
| `POST /api/v1/investor-portal/distributions` | `investor_portal.rs:create_distribution` | partial | — | real repo, no test |
| `GET /api/v1/investor-portal/investors/{investor_id}/distributions` | `investor_portal.rs:list_investor_distributions` | partial | — | real repo, no test |
| `PUT /api/v1/investor-portal/distributions/{distribution_id}` | `investor_portal.rs:update_distribution` | partial | — | real repo, no test |
| `POST /api/v1/investor-portal/reports` | `investor_portal.rs:create_report` | partial | — | real repo, no test |
| `GET /api/v1/investor-portal/investors/{investor_id}/reports` | `investor_portal.rs:list_investor_reports` | partial | — | real repo, no test |
| `GET /api/v1/investor-portal/reports/{report_id}` | `investor_portal.rs:get_report` | partial | — | real repo, no test |
| `POST /api/v1/investor-portal/capital-calls` | `investor_portal.rs:create_capital_call` | partial | — | real repo, no test |
| `GET /api/v1/investor-portal/investors/{investor_id}/capital-calls` | `investor_portal.rs:list_investor_capital_calls` | partial | — | real repo, no test |
| `PUT /api/v1/investor-portal/capital-calls/{call_id}` | `investor_portal.rs:update_capital_call` | partial | — | real repo, no test |
| `GET /api/v1/investor-portal/dashboard/{investor_id}` | `investor_portal.rs:get_investor_dashboard` | partial | — | real repo, no test |
| `POST /api/v1/investor-portal/dashboard/{investor_id}/metrics` | `investor_portal.rs:upsert_dashboard_metrics` | partial | — | real repo, no test |
| `GET /api/v1/owner-analytics/units/{unit_id}/valuation` | `owner_analytics.rs:get_unit_valuation` | done | `owner_analytics_cross_org_idor_tests.rs` | same-org/auth asserts |
| `POST /api/v1/owner-analytics/units/{unit_id}/valuation` | `owner_analytics.rs:create_valuation` | partial | — | real repo, no test |
| `GET /api/v1/owner-analytics/valuations/{valuation_id}` | `owner_analytics.rs:get_valuation_with_comparables` | partial | — | real repo, no test |
| `POST /api/v1/owner-analytics/valuations/{valuation_id}/comparables` | `owner_analytics.rs:add_comparable` | partial | — | real repo, no test |
| `GET /api/v1/owner-analytics/units/{unit_id}/value-history` | `owner_analytics.rs:get_value_history` | done | `owner_analytics_cross_org_idor_tests.rs` | cross-org leak assert |
| `GET /api/v1/owner-analytics/units/{unit_id}/value-trend` | `owner_analytics.rs:get_value_trend` | partial | — | real repo, no test |
| `POST /api/v1/owner-analytics/units/{unit_id}/roi` | `owner_analytics.rs:calculate_roi` | partial | — | real repo, no test |
| `GET /api/v1/owner-analytics/units/{unit_id}/cash-flow` | `owner_analytics.rs:get_cash_flow_breakdown` | partial | — | real repo, no test |
| `GET /api/v1/owner-analytics/units/{unit_id}/roi-dashboard` | `owner_analytics.rs:get_roi_dashboard` | partial | — | real repo, no test |
| `GET /api/v1/owner-analytics/portfolio` | `owner_analytics.rs:get_portfolio_summary` | partial | — | real repo, no test |
| `POST /api/v1/owner-analytics/portfolio/compare` | `owner_analytics.rs:compare_properties` | partial | — | real repo, no test |
| `GET /api/v1/owner-analytics/expense-rules` | `owner_analytics.rs:list_auto_approval_rules` | partial | — | real repo, no test |
| `POST /api/v1/owner-analytics/expense-rules` | `owner_analytics.rs:create_auto_approval_rule` | partial | — | real repo, no test |
| `PUT /api/v1/owner-analytics/expense-rules/{id}` | `owner_analytics.rs:update_auto_approval_rule` | partial | — | real repo, no test |
| `DELETE /api/v1/owner-analytics/expense-rules/{id}` | `owner_analytics.rs:delete_auto_approval_rule` | partial | — | real repo, no test |
| `POST /api/v1/owner-analytics/expenses/submit` | `owner_analytics.rs:submit_expense` | partial | — | real repo, no test |
| `GET /api/v1/owner-analytics/expenses` | `owner_analytics.rs:list_expense_requests` | partial | — | real repo, no test |
| `POST /api/v1/owner-analytics/expenses/{id}/review` | `owner_analytics.rs:review_expense` | partial | — | real repo, no test |
| `GET /api/v1/portfolio-analytics/summary` | `portfolio_analytics.rs:get_portfolio_summary` | partial | — | real repo, no test |
| `GET /api/v1/portfolio-analytics/benchmarks` | `portfolio_analytics.rs:list_benchmarks` | partial | — | real repo, no test |
| `POST /api/v1/portfolio-analytics/benchmarks` | `portfolio_analytics.rs:create_benchmark` | partial | — | real repo, no test |
| `GET /api/v1/portfolio-analytics/benchmarks/{id}` | `portfolio_analytics.rs:get_benchmark` | partial | — | real repo, no test |
| `PUT /api/v1/portfolio-analytics/benchmarks/{id}` | `portfolio_analytics.rs:update_benchmark` | partial | — | real repo, no test |
| `DELETE /api/v1/portfolio-analytics/benchmarks/{id}` | `portfolio_analytics.rs:delete_benchmark` | partial | — | real repo, no test |
| `GET /api/v1/portfolio-analytics/properties/metrics` | `portfolio_analytics.rs:list_property_metrics` | partial | — | real repo, no test |
| `POST /api/v1/portfolio-analytics/properties/metrics` | `portfolio_analytics.rs:upsert_property_metrics` | partial | — | real repo, no test |
| `GET /api/v1/portfolio-analytics/properties/{building_id}/metrics` | `portfolio_analytics.rs:get_property_metrics` | partial | — | real repo, no test |
| `GET /api/v1/portfolio-analytics/metrics` | `portfolio_analytics.rs:get_portfolio_metrics` | partial | — | real repo, no test |
| `POST /api/v1/portfolio-analytics/metrics/calculate` | `portfolio_analytics.rs:calculate_portfolio_metrics` | partial | — | real repo, no test |
| `GET /api/v1/portfolio-analytics/comparisons` | `portfolio_analytics.rs:list_comparisons` | partial | — | real repo, no test |
| `POST /api/v1/portfolio-analytics/comparisons` | `portfolio_analytics.rs:create_comparison` | partial | — | real repo, no test |
| `GET /api/v1/portfolio-analytics/comparisons/{id}` | `portfolio_analytics.rs:get_comparison` | partial | — | real repo, no test |
| `DELETE /api/v1/portfolio-analytics/comparisons/{id}` | `portfolio_analytics.rs:delete_comparison` | partial | — | real repo, no test |
| `GET /api/v1/portfolio-analytics/trends` | `portfolio_analytics.rs:get_trends` | partial | — | real repo, no test |
| `POST /api/v1/portfolio-analytics/trends` | `portfolio_analytics.rs:record_trend` | partial | — | real repo, no test |
| `GET /api/v1/portfolio-analytics/alerts/rules` | `portfolio_analytics.rs:list_alert_rules` | partial | — | real repo, no test |
| `POST /api/v1/portfolio-analytics/alerts/rules` | `portfolio_analytics.rs:create_alert_rule` | partial | — | real repo, no test |
| `GET /api/v1/portfolio-analytics/alerts/rules/{id}` | `portfolio_analytics.rs:get_alert_rule` | partial | — | real repo, no test |
| `PUT /api/v1/portfolio-analytics/alerts/rules/{id}` | `portfolio_analytics.rs:update_alert_rule` | partial | — | real repo, no test |
| `DELETE /api/v1/portfolio-analytics/alerts/rules/{id}` | `portfolio_analytics.rs:delete_alert_rule` | partial | — | real repo, no test |
| `GET /api/v1/portfolio-analytics/alerts` | `portfolio_analytics.rs:list_alerts` | partial | — | real repo, no test |
| `GET /api/v1/portfolio-analytics/alerts/stats` | `portfolio_analytics.rs:get_alert_stats` | partial | — | real repo, no test |
| `GET /api/v1/portfolio-analytics/alerts/{id}` | `portfolio_analytics.rs:get_alert` | partial | — | real repo, no test |
| `POST /api/v1/portfolio-analytics/alerts/{id}/acknowledge` | `portfolio_analytics.rs:acknowledge_alert` | partial | — | real repo, no test |
| `POST /api/v1/portfolio-analytics/alerts/{id}/resolve` | `portfolio_analytics.rs:resolve_alert` | partial | — | real repo, no test |
| `POST /api/v1/portfolio-performance/portfolios` | `portfolio_performance.rs:create_portfolio` | partial | — | real repo, no test |
| `GET /api/v1/portfolio-performance/portfolios` | `portfolio_performance.rs:list_portfolios` | partial | — | real repo, no test |
| `GET /api/v1/portfolio-performance/portfolios/{id}` | `portfolio_performance.rs:get_portfolio` | partial | — | real repo, no test |
| `PUT /api/v1/portfolio-performance/portfolios/{id}` | `portfolio_performance.rs:update_portfolio` | partial | — | real repo, no test |
| `DELETE /api/v1/portfolio-performance/portfolios/{id}` | `portfolio_performance.rs:delete_portfolio` | partial | — | real repo, no test |
| `POST /api/v1/portfolio-performance/portfolios/{id}/properties` | `portfolio_performance.rs:add_property` | partial | — | real repo, no test |
| `GET /api/v1/portfolio-performance/portfolios/{id}/properties` | `portfolio_performance.rs:list_properties` | partial | — | real repo, no test |
| `GET /api/v1/portfolio-performance/portfolios/{id}/properties/{property_id}` | `portfolio_performance.rs:get_property` | partial | — | real repo, no test |
| `PUT /api/v1/portfolio-performance/portfolios/{id}/properties/{property_id}` | `portfolio_performance.rs:update_property` | partial | — | real repo, no test |
| `DELETE /api/v1/portfolio-performance/portfolios/{id}/properties/{property_id}` | `portfolio_performance.rs:remove_property` | partial | — | real repo, no test |
| `POST /api/v1/portfolio-performance/portfolios/{id}/transactions` | `portfolio_performance.rs:create_transaction` | partial | — | real repo, no test |
| `GET /api/v1/portfolio-performance/portfolios/{id}/transactions` | `portfolio_performance.rs:list_transactions` | partial | — | real repo, no test |
| `GET /api/v1/portfolio-performance/portfolios/{id}/transactions/{transaction_id}` | `portfolio_performance.rs:get_transaction` | partial | — | real repo, no test |
| `PUT /api/v1/portfolio-performance/portfolios/{id}/transactions/{transaction_id}` | `portfolio_performance.rs:update_transaction` | partial | — | real repo, no test |
| `DELETE /api/v1/portfolio-performance/portfolios/{id}/transactions/{transaction_id}` | `portfolio_performance.rs:delete_transaction` | partial | — | real repo, no test |
| `POST /api/v1/portfolio-performance/portfolios/{id}/cash-flows` | `portfolio_performance.rs:upsert_cash_flow` | partial | — | real repo, no test |
| `GET /api/v1/portfolio-performance/portfolios/{id}/cash-flows` | `portfolio_performance.rs:get_cash_flows` | partial | — | real repo, no test |
| `POST /api/v1/portfolio-performance/portfolios/{id}/metrics/calculate` | `portfolio_performance.rs:calculate_metrics` | partial | — | real repo, no test |
| `GET /api/v1/portfolio-performance/portfolios/{id}/metrics/latest` | `portfolio_performance.rs:get_latest_metrics` | partial | — | real repo, no test |
| `GET /api/v1/portfolio-performance/portfolios/{id}/metrics/summary` | `portfolio_performance.rs:get_metrics_summary` | partial | — | real repo, no test |
| `POST /api/v1/portfolio-performance/benchmarks` | `portfolio_performance.rs:create_benchmark` | partial | — | real repo, no test |
| `GET /api/v1/portfolio-performance/benchmarks` | `portfolio_performance.rs:list_benchmarks` | partial | — | real repo, no test |
| `GET /api/v1/portfolio-performance/benchmarks/{id}` | `portfolio_performance.rs:get_benchmark` | partial | — | real repo, no test |
| `PUT /api/v1/portfolio-performance/benchmarks/{id}` | `portfolio_performance.rs:update_benchmark` | partial | — | real repo, no test |
| `DELETE /api/v1/portfolio-performance/benchmarks/{id}` | `portfolio_performance.rs:delete_benchmark` | partial | — | real repo, no test |
| `POST /api/v1/portfolio-performance/portfolios/{id}/comparisons` | `portfolio_performance.rs:create_comparison` | partial | — | real repo, no test |
| `GET /api/v1/portfolio-performance/portfolios/{id}/comparisons` | `portfolio_performance.rs:list_comparisons` | partial | — | real repo, no test |
| `GET /api/v1/portfolio-performance/portfolios/{id}/comparisons/{comparison_id}` | `portfolio_performance.rs:get_comparison` | partial | — | real repo, no test |
| `GET /api/v1/portfolio-performance/portfolios/{id}/dashboard/summary` | `portfolio_performance.rs:get_dashboard_summary` | partial | — | real repo, no test |
| `GET /api/v1/portfolio-performance/portfolios/{id}/dashboard/property-cards` | `portfolio_performance.rs:get_property_cards` | partial | — | real repo, no test |
| `GET /api/v1/portfolio-performance/portfolios/{id}/dashboard/cash-flow-trend` | `portfolio_performance.rs:get_cash_flow_trend` | partial | — | real repo, no test |
| `POST /api/v1/portfolio-performance/portfolios/{id}/alerts` | `portfolio_performance.rs:create_alert` | partial | — | real repo, no test |
| `GET /api/v1/portfolio-performance/portfolios/{id}/alerts` | `portfolio_performance.rs:list_alerts` | partial | — | real repo, no test |
| `POST /api/v1/portfolio-performance/portfolios/{id}/alerts/{alert_id}/read` | `portfolio_performance.rs:mark_alert_read` | partial | — | real repo, no test |
| `POST /api/v1/portfolio-performance/portfolios/{id}/alerts/{alert_id}/resolve` | `portfolio_performance.rs:resolve_alert` | partial | — | real repo, no test |
| `GET /health` | `health.rs:liveness` | done | `dev_mode_tenant_tests.rs` | 200 OK asserted via router |
| `GET /readiness` | `health.rs:readiness` | partial | — | DB+Redis check, no live test |

## Tally
done: 4  partial: 108  stub: 0  missing: 0  total: 112
