# Finance

_Server: api-server. Modules: financial.rs, budgets.rs, multi_currency.rs, subscriptions.rs, person_months.rs, market_pricing.rs, property_valuation.rs, reports.rs, accounting/ (contacts, invoices, matches, statements)._

Mount prefixes resolved from `backend/servers/api-server/src/lib.rs`:
- financial → `/api/v1/financial`
- budgets → `/api/v1/budgets`
- multi_currency → `/api/v1/multi-currency`
- subscriptions → `/api/v1/subscriptions` (+ admin_router at `/api/v1/admin/subscriptions`)
- market_pricing → DUAL MOUNT: `/api/v1/pricing` AND `/api/v1/market-pricing` (same router, both live)
- property_valuation → DUAL MOUNT: `/api/v1/property-valuations` AND `/api/v1/property-valuation` (alias)
- reports → `/api/v1/reports`
- accounting → `/api/v1/accounting`
- person_months → NOT mounted in lib.rs; nested under buildings.rs: `router()` at `/api/v1/buildings/{building_id}/units/{unit_id}/person-months`, `building_router()` at `/api/v1/buildings/{building_id}/person-months`

All handlers in this group are real (query repos/services); no `todo!()`/`unimplemented!()`/501/mock markers found. Status is therefore `done` (real handler + happy-path test) or `partial` (real handler, no happy-path test). The only tests present are mostly cross-org/IDOR/authz tests that assert 401/403/404 and do NOT exercise the success path — those leave their endpoints `partial` per spec.

## financial.rs  (mount: /api/v1/financial)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/financial/accounts | create_account | done | financial_happy_path_tests.rs | happy-path: financial_endpoints_happy_path |
| GET | /api/v1/financial/accounts | list_accounts | done | financial_cross_org_idor_tests.rs | happy-path: list_accounts_for_own_org_succeeds (200 + non-empty) |
| GET | /api/v1/financial/accounts/{id} | get_account | done | financial_happy_path_tests.rs | happy-path: financial_endpoints_happy_path |
| GET | /api/v1/financial/accounts/{id}/transactions | list_transactions | done | financial_happy_path_tests.rs | happy-path: financial_endpoints_happy_path |
| POST | /api/v1/financial/accounts/{id}/transactions | create_transaction | done | financial_happy_path_tests.rs | happy-path: financial_endpoints_happy_path |
| GET | /api/v1/financial/units/{unit_id}/ledger | get_unit_ledger | done | financial_happy_path_tests.rs | happy-path: financial_endpoints_happy_path |
| POST | /api/v1/financial/fee-schedules | create_fee_schedule | done | financial_happy_path_tests.rs | happy-path: financial_endpoints_happy_path |
| GET | /api/v1/financial/fee-schedules | list_fee_schedules | done | financial_happy_path_tests.rs | happy-path: financial_endpoints_happy_path |
| GET | /api/v1/financial/fee-schedules/{id} | get_fee_schedule | done | financial_happy_path_tests.rs | happy-path: financial_endpoints_happy_path |
| GET | /api/v1/financial/units/{unit_id}/fees | get_unit_fees | done | financial_happy_path_tests.rs | happy-path: financial_endpoints_happy_path |
| POST | /api/v1/financial/units/{unit_id}/fees | assign_unit_fee | done | financial_happy_path_tests.rs | happy-path: financial_endpoints_happy_path |
| POST | /api/v1/financial/invoices | create_invoice | done | financial_happy_path_tests.rs | happy-path: financial_endpoints_happy_path |
| GET | /api/v1/financial/invoices | list_invoices | done | financial_happy_path_tests.rs | happy-path: financial_endpoints_happy_path |
| GET | /api/v1/financial/invoices/{id} | get_invoice | done | financial_happy_path_tests.rs | happy-path: financial_endpoints_happy_path |
| POST | /api/v1/financial/invoices/{id}/send | send_invoice | done | financial_happy_path_tests.rs | happy-path: financial_endpoints_happy_path |
| GET | /api/v1/financial/invoices/{id}/pdf | get_invoice_pdf | done | financial_happy_path_tests.rs | happy-path: financial_endpoints_happy_path |
| POST | /api/v1/financial/invoices/{id}/checkout | initiate_invoice_checkout | done | financial_happy_path_tests.rs | happy-path: financial_endpoints_happy_path |
| GET | /api/v1/financial/units/{unit_id}/invoices | list_unit_invoices | done | financial_happy_path_tests.rs | happy-path: financial_endpoints_happy_path |
| POST | /api/v1/financial/payments | record_payment | done | financial_happy_path_tests.rs | happy-path: financial_endpoints_happy_path |
| GET | /api/v1/financial/payments | list_payments | done | financial_happy_path_tests.rs | happy-path: financial_endpoints_happy_path |
| GET | /api/v1/financial/payments/unallocated | list_unallocated_payments | done | financial_happy_path_tests.rs | happy-path: financial_endpoints_happy_path |
| POST | /api/v1/financial/payments/auto-match | auto_match_payments | done | financial_happy_path_tests.rs | happy-path: financial_endpoints_happy_path |
| GET | /api/v1/financial/payments/{id} | get_payment | done | financial_happy_path_tests.rs | happy-path: financial_endpoints_happy_path |
| POST | /api/v1/financial/payments/{id}/allocate | allocate_payment | done | financial_happy_path_tests.rs | happy-path: financial_endpoints_happy_path |
| GET | /api/v1/financial/units/{unit_id}/payments | list_unit_payments | done | financial_happy_path_tests.rs | happy-path: financial_endpoints_happy_path |
| GET | /api/v1/financial/reminder-schedules | get_reminder_schedules | partial | — | no test |
| GET | /api/v1/financial/late-fee-config | get_late_fee_config | partial | — | no test |
| GET | /api/v1/financial/overdue-invoices | get_overdue_invoices | partial | — | no test |
| GET | /api/v1/financial/reports/ar-aging | get_ar_aging_report | partial | — | no test |
| GET | /api/v1/financial/reports/income-statement | get_income_statement | partial | — | no test |
| GET | /api/v1/financial/reports/balance-sheet | get_balance_sheet | partial | — | no test |
| GET | /api/v1/financial/reports/cash-flow | get_cash_flow | partial | — | no test |
| GET | /api/v1/financial/reports/{report}/export | export_report | partial | — | no test |

## budgets.rs  (mount: /api/v1/budgets)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/budgets/ | create_budget | partial | — | no test (entire module untested) |
| GET | /api/v1/budgets/ | list_budgets | partial | — | no test |
| GET | /api/v1/budgets/{id} | get_budget | partial | — | no test |
| PUT | /api/v1/budgets/{id} | update_budget | partial | — | no test |
| DELETE | /api/v1/budgets/{id} | delete_budget | done | budget_capital_forecast_tests.rs | happy-path: forecasts_and_budget_delete_happy_path |
| POST | /api/v1/budgets/{id}/submit | submit_budget | partial | — | no test |
| POST | /api/v1/budgets/{id}/approve | approve_budget | partial | — | no test |
| POST | /api/v1/budgets/{id}/activate | activate_budget | partial | — | no test |
| POST | /api/v1/budgets/{id}/close | close_budget | partial | — | no test |
| GET | /api/v1/budgets/{id}/summary | get_budget_summary | partial | — | no test |
| GET | /api/v1/budgets/{id}/variance | get_category_variance | partial | — | no test |
| GET | /api/v1/budgets/{id}/alerts | list_variance_alerts | partial | — | no test |
| POST | /api/v1/budgets/{id}/items | add_budget_item | partial | — | no test |
| GET | /api/v1/budgets/{id}/items | list_budget_items | partial | — | no test |
| PUT | /api/v1/budgets/items/{item_id} | update_budget_item | partial | — | no test |
| DELETE | /api/v1/budgets/items/{item_id} | delete_budget_item | partial | — | no test |
| POST | /api/v1/budgets/items/{item_id}/actuals | record_actual | partial | — | no test |
| GET | /api/v1/budgets/items/{item_id}/actuals | list_actuals | partial | — | no test |
| POST | /api/v1/budgets/categories | create_category | partial | — | no test |
| GET | /api/v1/budgets/categories | list_categories | partial | — | no test |
| PUT | /api/v1/budgets/categories/{id} | update_category | partial | — | no test |
| DELETE | /api/v1/budgets/categories/{id} | delete_category | partial | — | no test |
| POST | /api/v1/budgets/alerts/{id}/acknowledge | acknowledge_alert | partial | — | no test |
| GET | /api/v1/budgets/dashboard | get_dashboard | partial | — | no test |
| POST | /api/v1/budgets/capital-plans | create_capital_plan | done | budget_capital_forecast_tests.rs | happy-path: capital_plans_happy_path |
| GET | /api/v1/budgets/capital-plans | list_capital_plans | done | budget_capital_forecast_tests.rs | happy-path: capital_plans_happy_path |
| GET | /api/v1/budgets/capital-plans/summary | get_yearly_capital_summary | done | budget_capital_forecast_tests.rs | happy-path: capital_plans_happy_path |
| GET | /api/v1/budgets/capital-plans/{id} | get_capital_plan | done | budget_capital_forecast_tests.rs | happy-path: capital_plans_happy_path |
| PUT | /api/v1/budgets/capital-plans/{id} | update_capital_plan | done | budget_capital_forecast_tests.rs | happy-path: capital_plans_happy_path |
| DELETE | /api/v1/budgets/capital-plans/{id} | delete_capital_plan | done | budget_capital_forecast_tests.rs | happy-path: capital_plans_happy_path |
| POST | /api/v1/budgets/capital-plans/{id}/start | start_capital_plan | done | budget_capital_forecast_tests.rs | happy-path: capital_plans_happy_path |
| POST | /api/v1/budgets/capital-plans/{id}/complete | complete_capital_plan | done | budget_capital_forecast_tests.rs | happy-path: capital_plans_happy_path |
| POST | /api/v1/budgets/forecasts | create_forecast | done | budget_capital_forecast_tests.rs | happy-path: forecasts_and_budget_delete_happy_path |
| GET | /api/v1/budgets/forecasts | list_forecasts | done | budget_capital_forecast_tests.rs | happy-path: forecasts_and_budget_delete_happy_path |
| GET | /api/v1/budgets/forecasts/{id} | get_forecast | done | budget_capital_forecast_tests.rs | happy-path: forecasts_and_budget_delete_happy_path |
| PUT | /api/v1/budgets/forecasts/{id} | update_forecast | done | budget_capital_forecast_tests.rs | happy-path: forecasts_and_budget_delete_happy_path |
| DELETE | /api/v1/budgets/forecasts/{id} | delete_forecast | done | budget_capital_forecast_tests.rs | happy-path: forecasts_and_budget_delete_happy_path |

## multi_currency.rs  (mount: /api/v1/multi-currency)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/multi-currency/config | get_currency_config | partial | — | no test |
| POST | /api/v1/multi-currency/config | create_or_update_currency_config | partial | — | no test |
| PUT | /api/v1/multi-currency/config | update_currency_config | partial | — | no test |
| GET | /api/v1/multi-currency/properties | list_property_currency_configs | partial | — | no test |
| POST | /api/v1/multi-currency/properties | create_property_currency_config | partial | — | no test |
| GET | /api/v1/multi-currency/properties/{building_id} | get_property_currency_config | done | multi_currency_cross_org_idor_tests.rs | happy-path: get_property_currency_config_same_org_succeeds (200 + body) |
| PUT | /api/v1/multi-currency/properties/{building_id} | update_property_currency_config | partial | — | no test |
| GET | /api/v1/multi-currency/rates | list_exchange_rates | partial | — | no test |
| POST | /api/v1/multi-currency/rates | create_exchange_rate | partial | — | no test |
| GET | /api/v1/multi-currency/rates/latest | get_latest_exchange_rate | partial | — | no test |
| POST | /api/v1/multi-currency/rates/override | override_exchange_rate | partial | — | no test |
| POST | /api/v1/multi-currency/rates/fetch | fetch_exchange_rates | partial | — | no test |
| GET | /api/v1/multi-currency/transactions | list_transactions | partial | — | no test |
| POST | /api/v1/multi-currency/transactions | create_transaction | partial | — | no test |
| GET | /api/v1/multi-currency/transactions/{id} | get_transaction | partial | — | no test |
| PUT | /api/v1/multi-currency/transactions/{id}/rate | update_transaction_rate | partial | — | no test |
| GET | /api/v1/multi-currency/cross-border | list_cross_border_leases | partial | — | no test |
| POST | /api/v1/multi-currency/cross-border | create_cross_border_lease | partial | — | no test |
| GET | /api/v1/multi-currency/cross-border/{lease_id} | get_cross_border_lease | partial | — | no test |
| PUT | /api/v1/multi-currency/cross-border/{lease_id} | update_cross_border_lease | partial | — | no test |
| GET | /api/v1/multi-currency/cross-border/compliance/{country} | get_compliance_requirements | partial | — | no test |
| GET | /api/v1/multi-currency/reports/configs | list_report_configs | partial | — | no test |
| POST | /api/v1/multi-currency/reports/configs | create_report_config | partial | — | no test |
| POST | /api/v1/multi-currency/reports/generate | generate_report | partial | — | no test |
| GET | /api/v1/multi-currency/reports/snapshots | list_report_snapshots | partial | — | no test |
| GET | /api/v1/multi-currency/reports/exposure | get_currency_exposure | partial | — | no test |
| GET | /api/v1/multi-currency/dashboard | get_dashboard | partial | — | no test |
| GET | /api/v1/multi-currency/statistics | get_statistics | partial | — | no test |

## subscriptions.rs  (mount: /api/v1/subscriptions ; admin_router at /api/v1/admin/subscriptions)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/subscriptions/plans | create_plan | partial | — | no test (entire module untested) |
| GET | /api/v1/subscriptions/plans | list_plans | partial | — | no test |
| GET | /api/v1/subscriptions/plans/public | list_public_plans | partial | — | no test |
| GET | /api/v1/subscriptions/plans/{id} | get_plan | partial | — | no test |
| PATCH | /api/v1/subscriptions/plans/{id} | update_plan | partial | — | no test |
| DELETE | /api/v1/subscriptions/plans/{id} | delete_plan | partial | — | no test |
| POST | /api/v1/subscriptions/ | create_subscription | partial | — | no test |
| GET | /api/v1/subscriptions/ | get_subscription | partial | — | no test |
| GET | /api/v1/subscriptions/with-plan | get_subscription_with_plan | partial | — | no test |
| PATCH | /api/v1/subscriptions/{id} | update_subscription | partial | — | no test |
| POST | /api/v1/subscriptions/{id}/change-plan | change_plan | partial | — | no test |
| POST | /api/v1/subscriptions/{id}/cancel | cancel_subscription | partial | — | no test |
| POST | /api/v1/subscriptions/{id}/reactivate | reactivate_subscription | partial | — | no test |
| POST | /api/v1/subscriptions/payment-methods | create_payment_method | partial | — | no test |
| GET | /api/v1/subscriptions/payment-methods | list_payment_methods | partial | — | no test |
| POST | /api/v1/subscriptions/payment-methods/{id}/default | set_default_payment_method | partial | — | no test |
| DELETE | /api/v1/subscriptions/payment-methods/{id} | delete_payment_method | partial | — | no test |
| GET | /api/v1/subscriptions/invoices | list_invoices | partial | — | no test |
| GET | /api/v1/subscriptions/invoices/{id} | get_invoice | partial | — | no test |
| GET | /api/v1/subscriptions/invoices/{id}/line-items | get_invoice_line_items | partial | — | no test |
| POST | /api/v1/subscriptions/invoices/{id}/pay | mark_invoice_paid | partial | — | no test |
| POST | /api/v1/subscriptions/invoices/{id}/void | void_invoice | partial | — | no test |
| POST | /api/v1/subscriptions/usage | record_usage | partial | — | no test |
| GET | /api/v1/subscriptions/usage/summary | get_usage_summary | partial | — | no test |
| GET | /api/v1/subscriptions/usage/current | get_current_usage | partial | — | no test |
| POST | /api/v1/subscriptions/coupons | create_coupon | partial | — | no test |
| GET | /api/v1/subscriptions/coupons | list_coupons | partial | — | no test |
| PATCH | /api/v1/subscriptions/coupons/{id} | update_coupon | partial | — | no test |
| POST | /api/v1/subscriptions/coupons/redeem | redeem_coupon | partial | — | no test |
| GET | /api/v1/subscriptions/statistics | get_statistics | partial | — | no test |
| GET | /api/v1/admin/subscriptions/subscriptions | list_all_subscriptions | partial | — | no test |
| GET | /api/v1/admin/subscriptions/invoices | list_all_invoices | partial | — | no test |

## person_months.rs  (mount: nested under buildings — NOT in lib.rs)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/buildings/{building_id}/units/{unit_id}/person-months | get_unit_person_months | partial | — | no test |
| POST | /api/v1/buildings/{building_id}/units/{unit_id}/person-months | upsert_person_month | partial | — | no test |
| GET | /api/v1/buildings/{building_id}/units/{unit_id}/person-months/{id} | get_person_month | partial | — | no test |
| PUT | /api/v1/buildings/{building_id}/units/{unit_id}/person-months/{id} | update_person_month | partial | — | no test |
| DELETE | /api/v1/buildings/{building_id}/units/{unit_id}/person-months/{id} | delete_person_month | partial | — | no test |
| GET | /api/v1/buildings/{building_id}/units/{unit_id}/person-months/yearly | get_yearly_summary | partial | — | no test |
| POST | /api/v1/buildings/{building_id}/units/{unit_id}/person-months/calculate | calculate_from_residents | partial | — | no test |
| GET | /api/v1/buildings/{building_id}/person-months | list_building_person_months | partial | — | no test |
| POST | /api/v1/buildings/{building_id}/person-months/bulk | bulk_upsert_person_months | partial | — | no test |
| GET | /api/v1/buildings/{building_id}/person-months/summary | get_building_summary | partial | — | no test |

## market_pricing.rs  (mount: /api/v1/pricing AND /api/v1/market-pricing — DUAL MOUNT, same router)
_Both prefixes serve the identical router; paths below shown with /api/v1/pricing, each is equally reachable at /api/v1/market-pricing. Endpoints counted once._
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/pricing/regions | list_regions | partial | — | no test (also at /api/v1/market-pricing) |
| POST | /api/v1/pricing/regions | create_region | partial | — | no test |
| GET | /api/v1/pricing/regions/{id} | get_region | partial | — | no test |
| PUT | /api/v1/pricing/regions/{id} | update_region | partial | — | no test |
| DELETE | /api/v1/pricing/regions/{id} | delete_region | partial | — | no test |
| GET | /api/v1/pricing/data | list_data_points | partial | — | no test |
| POST | /api/v1/pricing/data | add_data_point | partial | — | no test |
| GET | /api/v1/pricing/statistics/{region_id} | get_statistics | partial | — | no test |
| POST | /api/v1/pricing/statistics/generate | generate_statistics | partial | — | no test |
| GET | /api/v1/pricing/recommendations | list_recommendations | partial | — | no test |
| POST | /api/v1/pricing/recommendations/request | request_recommendation | partial | — | no test |
| GET | /api/v1/pricing/recommendations/{id} | get_recommendation | partial | — | no test |
| GET | /api/v1/pricing/recommendations/{id}/details | get_recommendation_details | partial | — | no test |
| POST | /api/v1/pricing/recommendations/{id}/accept | accept_recommendation | partial | — | no test |
| POST | /api/v1/pricing/recommendations/{id}/reject | reject_recommendation | partial | — | no test |
| GET | /api/v1/pricing/units/{unit_id}/history | get_pricing_history | partial | — | no test |
| POST | /api/v1/pricing/units/{unit_id}/price | record_price_change | partial | — | no test |
| GET | /api/v1/pricing/units/{unit_id}/current-rent | get_current_rent | partial | — | no test |
| GET | /api/v1/pricing/cma | list_cmas | partial | — | no test |
| POST | /api/v1/pricing/cma | create_cma | partial | — | no test |
| GET | /api/v1/pricing/cma/{id} | get_cma | partial | — | no test |
| PUT | /api/v1/pricing/cma/{id} | update_cma | partial | — | no test |
| DELETE | /api/v1/pricing/cma/{id} | delete_cma | partial | — | no test |
| GET | /api/v1/pricing/cma/{id}/details | get_cma_details | partial | — | no test |
| GET | /api/v1/pricing/cma/{id}/properties | get_cma_properties | partial | — | no test |
| POST | /api/v1/pricing/cma/{id}/properties | add_cma_property | partial | — | no test |
| DELETE | /api/v1/pricing/cma/{cma_id}/properties/{property_id} | remove_cma_property | partial | — | no test |
| POST | /api/v1/pricing/cma/{id}/recalculate | recalculate_cma | partial | — | no test |
| GET | /api/v1/pricing/comparables | get_comparables | partial | — | no test |
| GET | /api/v1/pricing/dashboard | get_pricing_dashboard | partial | — | no test |
| POST | /api/v1/pricing/dashboard/export | export_pricing_data | partial | — | no test |

## property_valuation.rs  (mount: /api/v1/property-valuations AND /api/v1/property-valuation — DUAL MOUNT/alias, same router)
_Both prefixes serve the identical router; paths shown with /api/v1/property-valuations. Endpoints counted once._
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/property-valuations/dashboard | get_dashboard | partial | — | no test (also at /api/v1/property-valuation) |
| GET | /api/v1/property-valuations/expiring | get_expiring_valuations | partial | — | no test |
| GET | /api/v1/property-valuations/models | list_models | partial | — | no test |
| POST | /api/v1/property-valuations/models | create_model | partial | — | no test |
| GET | /api/v1/property-valuations/models/{model_id} | get_model | partial | — | no test |
| PUT | /api/v1/property-valuations/models/{model_id} | update_model | partial | — | no test |
| DELETE | /api/v1/property-valuations/models/{model_id} | delete_model | partial | — | no test |
| GET | /api/v1/property-valuations/ | list_valuations | partial | — | no test |
| POST | /api/v1/property-valuations/ | create_valuation | partial | — | no test |
| GET | /api/v1/property-valuations/{valuation_id} | get_valuation | partial | — | no test |
| PUT | /api/v1/property-valuations/{valuation_id} | update_valuation | partial | — | no test |
| DELETE | /api/v1/property-valuations/{valuation_id} | delete_valuation | partial | — | no test |
| PUT | /api/v1/property-valuations/{valuation_id}/approve | approve_valuation | partial | — | no test |
| GET | /api/v1/property-valuations/{valuation_id}/comparables | list_comparables | partial | — | no test |
| POST | /api/v1/property-valuations/{valuation_id}/comparables | create_comparable | partial | — | no test |
| PUT | /api/v1/property-valuations/comparables/{comparable_id} | update_comparable | partial | — | no test |
| DELETE | /api/v1/property-valuations/comparables/{comparable_id} | delete_comparable | partial | — | no test |
| GET | /api/v1/property-valuations/comparables/{comparable_id}/adjustments | list_adjustments | partial | — | no test |
| POST | /api/v1/property-valuations/comparables/{comparable_id}/adjustments | create_adjustment | partial | — | no test |
| DELETE | /api/v1/property-valuations/adjustments/{adjustment_id} | delete_adjustment | partial | — | no test |
| GET | /api/v1/property-valuations/market-data | get_market_data | partial | — | no test |
| POST | /api/v1/property-valuations/market-data | create_market_data | partial | — | no test |
| PUT | /api/v1/property-valuations/market-data/{market_data_id} | update_market_data | partial | — | no test |
| GET | /api/v1/property-valuations/properties/{property_id}/history | get_value_history | partial | — | no test |
| POST | /api/v1/property-valuations/properties/{property_id}/history | create_value_history | partial | — | no test |
| GET | /api/v1/property-valuations/requests | list_requests | partial | — | no test |
| POST | /api/v1/property-valuations/requests | create_request | partial | — | no test |
| GET | /api/v1/property-valuations/requests/{request_id} | get_request | partial | — | no test |
| PUT | /api/v1/property-valuations/requests/{request_id} | update_request | partial | — | no test |
| GET | /api/v1/property-valuations/properties/{property_id}/features | get_features | partial | — | no test |
| POST | /api/v1/property-valuations/properties/{property_id}/features | create_features | partial | — | no test |
| PUT | /api/v1/property-valuations/features/{feature_id} | update_features | partial | — | no test |
| GET | /api/v1/property-valuations/{valuation_id}/reports | list_reports | partial | — | no test |
| POST | /api/v1/property-valuations/{valuation_id}/reports | create_report | partial | — | no test |
| PUT | /api/v1/property-valuations/reports/{report_id} | update_report | partial | — | no test |
| PUT | /api/v1/property-valuations/reports/{report_id}/sign | sign_report | partial | — | no test |
| GET | /api/v1/property-valuations/{valuation_id}/audit-logs | get_audit_logs | partial | — | no test |

## reports.rs  (mount: /api/v1/reports)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/reports/faults | get_fault_statistics_report | partial | — | no test (voting_report_tests hits /api/v1/voting, not reports) |
| GET | /api/v1/reports/voting | get_voting_participation_report | partial | — | no test |
| GET | /api/v1/reports/occupancy | get_occupancy_report | partial | — | no test |
| GET | /api/v1/reports/consumption | get_consumption_report | partial | — | no test |
| POST | /api/v1/reports/export | export_report | partial | reports_export_org_scope_tests.rs | authz-only (asserts 403 cross-org); no happy path |
| GET | /api/v1/reports/export/{job_id}/status | get_export_job_status | done | reports_export_org_scope_tests.rs | happy-path: get_export_job_status_for_own_org_is_allowed (200) |
| PUT | /api/v1/reports/schedules/{id} | update_schedule | done | report_schedule_cron_roundtrip_tests.rs | happy-path: PUT roundtrip asserts 200 |
| PUT | /api/v1/reports/schedules/{id}/pause | pause_schedule | partial | report_schedule_org_scope_jwt_tests.rs, report_schedule_sibling_scope_tests.rs | IDOR-only (404 cross-tenant); no happy path |
| PUT | /api/v1/reports/schedules/{id}/resume | resume_schedule | partial | report_schedule_org_scope_jwt_tests.rs, report_schedule_sibling_scope_tests.rs | IDOR-only (404 cross-tenant); no happy path |
| GET | /api/v1/reports/schedules/{id}/executions | list_schedule_executions | done | report_execution_download_retry_e2e_tests.rs | happy-path: same-org list returns 200 |
| GET | /api/v1/reports/executions/{id} | get_execution | partial | report_schedule_sibling_scope_tests.rs | IDOR-only; no happy path |
| GET | /api/v1/reports/executions/{id}/download | get_execution_download_url | done | report_execution_download_retry_e2e_tests.rs | happy-path: same-org presign returns 200 + non-empty URL |
| POST | /api/v1/reports/executions/{id}/retry | retry_execution | done | report_execution_download_retry_e2e_tests.rs | happy-path: retry_failed_execution_resets_to_pending (200) |

## accounting/ (mount: /api/v1/accounting; sub-routers nested in accounting/mod.rs)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/accounting/invoices/ | invoices::list_invoices | partial | — | no test |
| POST | /api/v1/accounting/invoices/ | invoices::create_invoice | partial | — | no test |
| GET | /api/v1/accounting/invoices/{id} | invoices::get_invoice | partial | — | no test |
| PATCH | /api/v1/accounting/invoices/{id} | invoices::update_invoice | partial | — | no test |
| DELETE | /api/v1/accounting/invoices/{id} | invoices::delete_invoice | partial | — | no test |
| GET | /api/v1/accounting/invoices/{id}/items | invoices::list_invoice_items | partial | — | no test |
| GET | /api/v1/accounting/contacts/ | contacts::list_contacts | done | accounting_contacts_authz_tests.rs | happy-path: manager_can_list_contacts (200) |
| GET | /api/v1/accounting/statements/ | statements::list_statements | partial | — | no test |
| POST | /api/v1/accounting/statements/ | statements::upload_statement | partial | — | no test |
| GET | /api/v1/accounting/statements/{id}/lines | statements::list_statement_lines | partial | — | no test |
| GET | /api/v1/accounting/lines/{id}/matches | matches::list_matches | partial | — | no test |
| POST | /api/v1/accounting/matches/{id}/confirm | matches::confirm_match | partial | — | no test |
| POST | /api/v1/accounting/matches/{id}/reject | matches::reject_match | partial | — | no test |

## Summary
- done: 8 | partial: 226 | stub: 0 | missing: 0 | total: 234

Per-module endpoint counts: financial 33, budgets 37, multi_currency 28, subscriptions 32 (30 + 2 admin), person_months 10 (7 unit + 3 building), market_pricing 31, property_valuation 37, reports 13, accounting 13.
