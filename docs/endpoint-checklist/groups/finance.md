# Finance

_Server: api-server. Modules: financial.rs, budgets.rs, multi_currency.rs, subscriptions.rs, person_months.rs, market_pricing.rs, property_valuation.rs, reports.rs, accounting/ (contacts, invoices, matches, statements)._

Mount prefixes resolved from `backend/servers/api-server/src/lib.rs`:
- financial → `/api/v1/financial`
- budgets → `/api/v1/budgets`
- multi_currency → `/api/v1/multi-currency`
- subscriptions → `/api/v1/subscriptions` (+ admin_router mounted at `/api/v1/admin` → `/api/v1/admin/subscriptions`, `/api/v1/admin/invoices`)
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
| GET | /api/v1/financial/reminder-schedules | get_reminder_schedules | done | financial_reports_happy_path_tests.rs | happy-path: financial_reports_happy_path |
| GET | /api/v1/financial/late-fee-config | get_late_fee_config | done | financial_reports_happy_path_tests.rs | happy-path: financial_reports_happy_path |
| GET | /api/v1/financial/overdue-invoices | get_overdue_invoices | done | financial_reports_happy_path_tests.rs | happy-path: financial_reports_happy_path |
| GET | /api/v1/financial/reports/ar-aging | get_ar_aging_report | done | financial_reports_happy_path_tests.rs | happy-path: financial_reports_happy_path |
| GET | /api/v1/financial/reports/income-statement | get_income_statement | done | financial_reports_happy_path_tests.rs | happy-path: financial_reports_happy_path |
| GET | /api/v1/financial/reports/balance-sheet | get_balance_sheet | done | financial_reports_happy_path_tests.rs | happy-path: financial_reports_happy_path |
| GET | /api/v1/financial/reports/cash-flow | get_cash_flow | done | financial_reports_happy_path_tests.rs | happy-path: financial_reports_happy_path |
| GET | /api/v1/financial/reports/{report}/export | export_report | done | financial_reports_happy_path_tests.rs | happy-path: financial_reports_happy_path |

## budgets.rs  (mount: /api/v1/budgets)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/budgets/ | create_budget | done | budgets_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| GET | /api/v1/budgets/ | list_budgets | done | budgets_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| GET | /api/v1/budgets/{id} | get_budget | done | budgets_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| PUT | /api/v1/budgets/{id} | update_budget | done | budgets_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| DELETE | /api/v1/budgets/{id} | delete_budget | done | budget_capital_forecast_tests.rs | happy-path: forecasts_and_budget_delete_happy_path |
| POST | /api/v1/budgets/{id}/submit | submit_budget | done | budgets_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| POST | /api/v1/budgets/{id}/approve | approve_budget | done | budgets_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| POST | /api/v1/budgets/{id}/activate | activate_budget | done | budgets_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| POST | /api/v1/budgets/{id}/close | close_budget | done | budgets_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| GET | /api/v1/budgets/{id}/summary | get_budget_summary | done | budgets_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| GET | /api/v1/budgets/{id}/variance | get_category_variance | done | budgets_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| GET | /api/v1/budgets/{id}/alerts | list_variance_alerts | done | budgets_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| POST | /api/v1/budgets/{id}/items | add_budget_item | done | budgets_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| GET | /api/v1/budgets/{id}/items | list_budget_items | done | budgets_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| PUT | /api/v1/budgets/items/{item_id} | update_budget_item | done | budgets_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| DELETE | /api/v1/budgets/items/{item_id} | delete_budget_item | done | budgets_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| POST | /api/v1/budgets/items/{item_id}/actuals | record_actual | done | budgets_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| GET | /api/v1/budgets/items/{item_id}/actuals | list_actuals | done | budgets_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| POST | /api/v1/budgets/categories | create_category | done | budgets_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| GET | /api/v1/budgets/categories | list_categories | done | budgets_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| PUT | /api/v1/budgets/categories/{id} | update_category | done | budgets_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| DELETE | /api/v1/budgets/categories/{id} | delete_category | done | budgets_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| POST | /api/v1/budgets/alerts/{id}/acknowledge | acknowledge_alert | done | budgets_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| GET | /api/v1/budgets/dashboard | get_dashboard | done | budgets_tests.rs | happy-path 2xx (BIT-415 reconcile) |
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
| GET | /api/v1/multi-currency/config | get_currency_config | done | multi_currency_happy_path_tests.rs | happy-path: multi_currency_endpoints_happy_path |
| POST | /api/v1/multi-currency/config | create_or_update_currency_config | done | multi_currency_happy_path_tests.rs | happy-path: multi_currency_endpoints_happy_path |
| PUT | /api/v1/multi-currency/config | update_currency_config | done | multi_currency_happy_path_tests.rs | happy-path: multi_currency_endpoints_happy_path |
| GET | /api/v1/multi-currency/properties | list_property_currency_configs | done | multi_currency_happy_path_tests.rs | happy-path: multi_currency_endpoints_happy_path |
| POST | /api/v1/multi-currency/properties | create_property_currency_config | done | multi_currency_happy_path_tests.rs | happy-path: multi_currency_endpoints_happy_path |
| GET | /api/v1/multi-currency/properties/{building_id} | get_property_currency_config | done | multi_currency_cross_org_idor_tests.rs | happy-path: get_property_currency_config_same_org_succeeds (200 + body) |
| PUT | /api/v1/multi-currency/properties/{building_id} | update_property_currency_config | done | multi_currency_happy_path_tests.rs | happy-path: multi_currency_endpoints_happy_path |
| GET | /api/v1/multi-currency/rates | list_exchange_rates | done | multi_currency_happy_path_tests.rs | happy-path: multi_currency_endpoints_happy_path |
| POST | /api/v1/multi-currency/rates | create_exchange_rate | done | multi_currency_happy_path_tests.rs | happy-path: multi_currency_endpoints_happy_path |
| GET | /api/v1/multi-currency/rates/latest | get_latest_exchange_rate | done | multi_currency_happy_path_tests.rs | happy-path: multi_currency_endpoints_happy_path |
| POST | /api/v1/multi-currency/rates/override | override_exchange_rate | done | multi_currency_happy_path_tests.rs | happy-path: multi_currency_endpoints_happy_path |
| POST | /api/v1/multi-currency/rates/fetch | fetch_exchange_rates | done | multi_currency_happy_path_tests.rs | happy-path: multi_currency_endpoints_happy_path |
| GET | /api/v1/multi-currency/transactions | list_transactions | done | multi_currency_happy_path_tests.rs | happy-path: multi_currency_endpoints_happy_path |
| POST | /api/v1/multi-currency/transactions | create_transaction | done | multi_currency_happy_path_tests.rs | happy-path: multi_currency_endpoints_happy_path |
| GET | /api/v1/multi-currency/transactions/{id} | get_transaction | done | multi_currency_happy_path_tests.rs | happy-path: multi_currency_endpoints_happy_path |
| PUT | /api/v1/multi-currency/transactions/{id}/rate | update_transaction_rate | done | multi_currency_happy_path_tests.rs | happy-path: multi_currency_endpoints_happy_path |
| GET | /api/v1/multi-currency/cross-border | list_cross_border_leases | done | multi_currency_happy_path_tests.rs | happy-path: multi_currency_endpoints_happy_path |
| POST | /api/v1/multi-currency/cross-border | create_cross_border_lease | done | multi_currency_happy_path_tests.rs | happy-path: multi_currency_endpoints_happy_path |
| GET | /api/v1/multi-currency/cross-border/{lease_id} | get_cross_border_lease | done | multi_currency_happy_path_tests.rs | happy-path: multi_currency_endpoints_happy_path |
| PUT | /api/v1/multi-currency/cross-border/{lease_id} | update_cross_border_lease | done | multi_currency_happy_path_tests.rs | happy-path: multi_currency_endpoints_happy_path |
| GET | /api/v1/multi-currency/cross-border/compliance/{country} | get_compliance_requirements | done | multi_currency_happy_path_tests.rs | happy-path: multi_currency_endpoints_happy_path |
| GET | /api/v1/multi-currency/reports/configs | list_report_configs | done | multi_currency_happy_path_tests.rs | happy-path: multi_currency_endpoints_happy_path |
| POST | /api/v1/multi-currency/reports/configs | create_report_config | done | multi_currency_happy_path_tests.rs | happy-path: multi_currency_endpoints_happy_path |
| POST | /api/v1/multi-currency/reports/generate | generate_report | done | multi_currency_happy_path_tests.rs | happy-path: multi_currency_endpoints_happy_path |
| GET | /api/v1/multi-currency/reports/snapshots | list_report_snapshots | done | multi_currency_happy_path_tests.rs | happy-path: multi_currency_endpoints_happy_path |
| GET | /api/v1/multi-currency/reports/exposure | get_currency_exposure | done | multi_currency_happy_path_tests.rs | happy-path: multi_currency_endpoints_happy_path |
| GET | /api/v1/multi-currency/dashboard | get_dashboard | done | multi_currency_happy_path_tests.rs | happy-path: multi_currency_endpoints_happy_path |
| GET | /api/v1/multi-currency/statistics | get_statistics | done | multi_currency_happy_path_tests.rs | happy-path: multi_currency_endpoints_happy_path |

## subscriptions.rs  (mount: /api/v1/subscriptions ; admin_router at /api/v1/admin → /api/v1/admin/{subscriptions,invoices})
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/subscriptions/plans | create_plan | done | subscriptions_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| GET | /api/v1/subscriptions/plans | list_plans | done | subscriptions_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| GET | /api/v1/subscriptions/plans/public | list_public_plans | done | subscriptions_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| GET | /api/v1/subscriptions/plans/{id} | get_plan | done | subscriptions_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| PATCH | /api/v1/subscriptions/plans/{id} | update_plan | done | subscriptions_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| DELETE | /api/v1/subscriptions/plans/{id} | delete_plan | done | subscriptions_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| POST | /api/v1/subscriptions/ | create_subscription | done | subscriptions_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| GET | /api/v1/subscriptions/ | get_subscription | done | subscriptions_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| GET | /api/v1/subscriptions/with-plan | get_subscription_with_plan | done | subscriptions_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| PATCH | /api/v1/subscriptions/{id} | update_subscription | done | subscriptions_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| POST | /api/v1/subscriptions/{id}/change-plan | change_plan | done | subscriptions_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| POST | /api/v1/subscriptions/{id}/cancel | cancel_subscription | done | subscriptions_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| POST | /api/v1/subscriptions/{id}/reactivate | reactivate_subscription | done | subscriptions_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| POST | /api/v1/subscriptions/payment-methods | create_payment_method | done | subscriptions_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| GET | /api/v1/subscriptions/payment-methods | list_payment_methods | done | subscriptions_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| POST | /api/v1/subscriptions/payment-methods/{id}/default | set_default_payment_method | done | subscriptions_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| DELETE | /api/v1/subscriptions/payment-methods/{id} | delete_payment_method | done | subscriptions_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| GET | /api/v1/subscriptions/invoices | list_invoices | done | subscriptions_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| GET | /api/v1/subscriptions/invoices/{id} | get_invoice | done | subscriptions_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| GET | /api/v1/subscriptions/invoices/{id}/line-items | get_invoice_line_items | done | subscriptions_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| POST | /api/v1/subscriptions/invoices/{id}/pay | mark_invoice_paid | done | subscriptions_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| POST | /api/v1/subscriptions/invoices/{id}/void | void_invoice | done | subscriptions_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| POST | /api/v1/subscriptions/usage | record_usage | done | subscriptions_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| GET | /api/v1/subscriptions/usage/summary | get_usage_summary | done | subscriptions_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| GET | /api/v1/subscriptions/usage/current | get_current_usage | done | subscriptions_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| POST | /api/v1/subscriptions/coupons | create_coupon | done | subscriptions_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| GET | /api/v1/subscriptions/coupons | list_coupons | done | subscriptions_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| PATCH | /api/v1/subscriptions/coupons/{id} | update_coupon | done | subscriptions_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| POST | /api/v1/subscriptions/coupons/redeem | redeem_coupon | done | subscriptions_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| GET | /api/v1/subscriptions/statistics | get_statistics | done | subscriptions_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| GET | /api/v1/admin/subscriptions | list_all_subscriptions | partial | — | no test |
| GET | /api/v1/admin/invoices | list_all_invoices | partial | — | no test |

## person_months.rs  (mount: nested under buildings — NOT in lib.rs)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/buildings/{building_id}/units/{unit_id}/person-months | get_unit_person_months | done | person_months_happy_path_tests.rs | happy-path: person_months_endpoints_happy_path |
| POST | /api/v1/buildings/{building_id}/units/{unit_id}/person-months | upsert_person_month | done | person_months_happy_path_tests.rs | happy-path: person_months_endpoints_happy_path |
| GET | /api/v1/buildings/{building_id}/units/{unit_id}/person-months/{id} | get_person_month | done | person_months_happy_path_tests.rs | happy-path: person_months_endpoints_happy_path |
| PUT | /api/v1/buildings/{building_id}/units/{unit_id}/person-months/{id} | update_person_month | done | person_months_happy_path_tests.rs | happy-path: person_months_endpoints_happy_path |
| DELETE | /api/v1/buildings/{building_id}/units/{unit_id}/person-months/{id} | delete_person_month | done | person_months_happy_path_tests.rs | happy-path: person_months_endpoints_happy_path |
| GET | /api/v1/buildings/{building_id}/units/{unit_id}/person-months/yearly | get_yearly_summary | done | person_months_happy_path_tests.rs | happy-path: person_months_endpoints_happy_path |
| POST | /api/v1/buildings/{building_id}/units/{unit_id}/person-months/calculate | calculate_from_residents | done | person_months_happy_path_tests.rs | happy-path: person_months_endpoints_happy_path |
| GET | /api/v1/buildings/{building_id}/person-months | list_building_person_months | done | person_months_happy_path_tests.rs | happy-path: person_months_endpoints_happy_path |
| POST | /api/v1/buildings/{building_id}/person-months/bulk | bulk_upsert_person_months | done | person_months_happy_path_tests.rs | happy-path: person_months_endpoints_happy_path |
| GET | /api/v1/buildings/{building_id}/person-months/summary | get_building_summary | done | person_months_happy_path_tests.rs | happy-path: person_months_endpoints_happy_path |

## market_pricing.rs  (mount: /api/v1/pricing AND /api/v1/market-pricing — DUAL MOUNT, same router)
_Both prefixes serve the identical router; paths below shown with /api/v1/pricing, each is equally reachable at /api/v1/market-pricing. Endpoints counted once._
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/pricing/regions | list_regions | done | market_pricing_happy_path_tests.rs | happy-path: market_pricing_endpoints_happy_path (also at /api/v1/market-pricing) |
| POST | /api/v1/pricing/regions | create_region | done | market_pricing_happy_path_tests.rs | happy-path: market_pricing_endpoints_happy_path |
| GET | /api/v1/pricing/regions/{id} | get_region | done | market_pricing_happy_path_tests.rs | happy-path: market_pricing_endpoints_happy_path |
| PUT | /api/v1/pricing/regions/{id} | update_region | done | market_pricing_happy_path_tests.rs | happy-path: market_pricing_endpoints_happy_path |
| DELETE | /api/v1/pricing/regions/{id} | delete_region | done | market_pricing_happy_path_tests.rs | happy-path: market_pricing_endpoints_happy_path |
| GET | /api/v1/pricing/data | list_data_points | done | market_pricing_happy_path_tests.rs | happy-path: market_pricing_endpoints_happy_path |
| POST | /api/v1/pricing/data | add_data_point | done | market_pricing_happy_path_tests.rs | happy-path: market_pricing_endpoints_happy_path |
| GET | /api/v1/pricing/statistics/{region_id} | get_statistics | done | market_pricing_happy_path_tests.rs | happy-path: market_pricing_endpoints_happy_path |
| POST | /api/v1/pricing/statistics/generate | generate_statistics | done | market_pricing_happy_path_tests.rs | happy-path: market_pricing_endpoints_happy_path |
| GET | /api/v1/pricing/recommendations | list_recommendations | done | market_pricing_happy_path_tests.rs | happy-path: market_pricing_endpoints_happy_path |
| POST | /api/v1/pricing/recommendations/request | request_recommendation | done | market_pricing_happy_path_tests.rs | happy-path: market_pricing_endpoints_happy_path |
| GET | /api/v1/pricing/recommendations/{id} | get_recommendation | done | market_pricing_happy_path_tests.rs | happy-path: market_pricing_endpoints_happy_path |
| GET | /api/v1/pricing/recommendations/{id}/details | get_recommendation_details | done | market_pricing_happy_path_tests.rs | happy-path: market_pricing_endpoints_happy_path |
| POST | /api/v1/pricing/recommendations/{id}/accept | accept_recommendation | done | market_pricing_happy_path_tests.rs | happy-path: market_pricing_endpoints_happy_path |
| POST | /api/v1/pricing/recommendations/{id}/reject | reject_recommendation | done | market_pricing_happy_path_tests.rs | happy-path: market_pricing_endpoints_happy_path |
| GET | /api/v1/pricing/units/{unit_id}/history | get_pricing_history | done | market_pricing_happy_path_tests.rs | happy-path: market_pricing_endpoints_happy_path |
| POST | /api/v1/pricing/units/{unit_id}/price | record_price_change | done | market_pricing_happy_path_tests.rs | happy-path: market_pricing_endpoints_happy_path |
| GET | /api/v1/pricing/units/{unit_id}/current-rent | get_current_rent | done | market_pricing_happy_path_tests.rs | happy-path: market_pricing_endpoints_happy_path |
| GET | /api/v1/pricing/cma | list_cmas | done | market_pricing_happy_path_tests.rs | happy-path: market_pricing_endpoints_happy_path |
| POST | /api/v1/pricing/cma | create_cma | done | market_pricing_happy_path_tests.rs | happy-path: market_pricing_endpoints_happy_path |
| GET | /api/v1/pricing/cma/{id} | get_cma | done | market_pricing_happy_path_tests.rs | happy-path: market_pricing_endpoints_happy_path |
| PUT | /api/v1/pricing/cma/{id} | update_cma | done | market_pricing_happy_path_tests.rs | happy-path: market_pricing_endpoints_happy_path |
| DELETE | /api/v1/pricing/cma/{id} | delete_cma | done | market_pricing_happy_path_tests.rs | happy-path: market_pricing_endpoints_happy_path |
| GET | /api/v1/pricing/cma/{id}/details | get_cma_details | done | market_pricing_happy_path_tests.rs | happy-path: market_pricing_endpoints_happy_path |
| GET | /api/v1/pricing/cma/{id}/properties | get_cma_properties | done | market_pricing_happy_path_tests.rs | happy-path: market_pricing_endpoints_happy_path |
| POST | /api/v1/pricing/cma/{id}/properties | add_cma_property | done | market_pricing_happy_path_tests.rs | happy-path: market_pricing_endpoints_happy_path |
| DELETE | /api/v1/pricing/cma/{cma_id}/properties/{property_id} | remove_cma_property | done | market_pricing_happy_path_tests.rs | happy-path: market_pricing_endpoints_happy_path |
| POST | /api/v1/pricing/cma/{id}/recalculate | recalculate_cma | done | market_pricing_happy_path_tests.rs | happy-path: market_pricing_endpoints_happy_path |
| GET | /api/v1/pricing/comparables | get_comparables | done | market_pricing_happy_path_tests.rs | happy-path: market_pricing_endpoints_happy_path |
| GET | /api/v1/pricing/dashboard | get_pricing_dashboard | done | market_pricing_happy_path_tests.rs | happy-path: market_pricing_endpoints_happy_path |
| POST | /api/v1/pricing/dashboard/export | export_pricing_data | done | market_pricing_happy_path_tests.rs | happy-path: market_pricing_endpoints_happy_path |

## property_valuation.rs  (mount: /api/v1/property-valuations AND /api/v1/property-valuation — DUAL MOUNT/alias, same router)
_Both prefixes serve the identical router; paths shown with /api/v1/property-valuations. Endpoints counted once._
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/property-valuations/dashboard | get_dashboard | done | property_valuation_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| GET | /api/v1/property-valuations/expiring | get_expiring_valuations | done | property_valuation_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| GET | /api/v1/property-valuations/models | list_models | done | property_valuation_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| POST | /api/v1/property-valuations/models | create_model | done | property_valuation_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| GET | /api/v1/property-valuations/models/{model_id} | get_model | done | property_valuation_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| PUT | /api/v1/property-valuations/models/{model_id} | update_model | done | property_valuation_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| DELETE | /api/v1/property-valuations/models/{model_id} | delete_model | done | property_valuation_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| GET | /api/v1/property-valuations/ | list_valuations | done | property_valuation_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| POST | /api/v1/property-valuations/ | create_valuation | done | property_valuation_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| GET | /api/v1/property-valuations/{valuation_id} | get_valuation | done | property_valuation_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| PUT | /api/v1/property-valuations/{valuation_id} | update_valuation | done | property_valuation_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| DELETE | /api/v1/property-valuations/{valuation_id} | delete_valuation | done | property_valuation_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| PUT | /api/v1/property-valuations/{valuation_id}/approve | approve_valuation | done | property_valuation_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| GET | /api/v1/property-valuations/{valuation_id}/comparables | list_comparables | partial | — | no test |
| POST | /api/v1/property-valuations/{valuation_id}/comparables | create_comparable | partial | — | no test |
| PUT | /api/v1/property-valuations/comparables/{comparable_id} | update_comparable | partial | — | no test |
| DELETE | /api/v1/property-valuations/comparables/{comparable_id} | delete_comparable | partial | — | no test |
| GET | /api/v1/property-valuations/comparables/{comparable_id}/adjustments | list_adjustments | partial | — | no test |
| POST | /api/v1/property-valuations/comparables/{comparable_id}/adjustments | create_adjustment | partial | — | no test |
| DELETE | /api/v1/property-valuations/adjustments/{adjustment_id} | delete_adjustment | partial | — | no test |
| GET | /api/v1/property-valuations/market-data | get_market_data | done | property_valuation_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| POST | /api/v1/property-valuations/market-data | create_market_data | done | property_valuation_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| PUT | /api/v1/property-valuations/market-data/{market_data_id} | update_market_data | done | property_valuation_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| GET | /api/v1/property-valuations/properties/{property_id}/history | get_value_history | done | property_valuation_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| POST | /api/v1/property-valuations/properties/{property_id}/history | create_value_history | done | property_valuation_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| GET | /api/v1/property-valuations/requests | list_requests | done | property_valuation_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| POST | /api/v1/property-valuations/requests | create_request | done | property_valuation_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| GET | /api/v1/property-valuations/requests/{request_id} | get_request | done | property_valuation_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| PUT | /api/v1/property-valuations/requests/{request_id} | update_request | partial | — | no test |
| GET | /api/v1/property-valuations/properties/{property_id}/features | get_features | partial | — | no test |
| POST | /api/v1/property-valuations/properties/{property_id}/features | create_features | partial | — | no test |
| PUT | /api/v1/property-valuations/features/{feature_id} | update_features | partial | — | no test |
| GET | /api/v1/property-valuations/{valuation_id}/reports | list_reports | partial | — | no test |
| POST | /api/v1/property-valuations/{valuation_id}/reports | create_report | partial | — | no test |
| PUT | /api/v1/property-valuations/reports/{report_id} | update_report | partial | — | no test |
| PUT | /api/v1/property-valuations/reports/{report_id}/sign | sign_report | partial | — | no test |
| GET | /api/v1/property-valuations/{valuation_id}/audit-logs | get_audit_logs | done | property_valuation_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |

## reports.rs  (mount: /api/v1/reports)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/reports/faults | get_fault_statistics_report | done | reports_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| GET | /api/v1/reports/voting | get_voting_participation_report | done | reports_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| GET | /api/v1/reports/occupancy | get_occupancy_report | done | reports_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| GET | /api/v1/reports/consumption | get_consumption_report | done | reports_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| POST | /api/v1/reports/export | export_report | done | reports_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| GET | /api/v1/reports/export/{job_id}/status | get_export_job_status | done | reports_export_org_scope_tests.rs | happy-path: get_export_job_status_for_own_org_is_allowed (200) |
| PUT | /api/v1/reports/schedules/{id} | update_schedule | done | report_schedule_cron_roundtrip_tests.rs | happy-path: PUT roundtrip asserts 200 |
| PUT | /api/v1/reports/schedules/{id}/pause | pause_schedule | done | reports_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| PUT | /api/v1/reports/schedules/{id}/resume | resume_schedule | done | reports_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| GET | /api/v1/reports/schedules/{id}/executions | list_schedule_executions | done | report_execution_download_retry_e2e_tests.rs | happy-path: same-org list returns 200 |
| GET | /api/v1/reports/executions/{id} | get_execution | partial | report_schedule_sibling_scope_tests.rs | IDOR-only; no happy path |
| GET | /api/v1/reports/executions/{id}/download | get_execution_download_url | done | report_execution_download_retry_e2e_tests.rs | happy-path: same-org presign returns 200 + non-empty URL |
| POST | /api/v1/reports/executions/{id}/retry | retry_execution | done | report_execution_download_retry_e2e_tests.rs | happy-path: retry_failed_execution_resets_to_pending (200) |

## accounting/ (mount: /api/v1/accounting; sub-routers nested in accounting/mod.rs)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/accounting/invoices/ | invoices::list_invoices | done | accounting_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| POST | /api/v1/accounting/invoices/ | invoices::create_invoice | done | accounting_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| GET | /api/v1/accounting/invoices/{id} | invoices::get_invoice | done | accounting_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| PATCH | /api/v1/accounting/invoices/{id} | invoices::update_invoice | done | accounting_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| DELETE | /api/v1/accounting/invoices/{id} | invoices::delete_invoice | done | accounting_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| GET | /api/v1/accounting/invoices/{id}/items | invoices::list_invoice_items | done | accounting_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| GET | /api/v1/accounting/contacts/ | contacts::list_contacts | done | accounting_contacts_authz_tests.rs | happy-path: manager_can_list_contacts (200) |
| GET | /api/v1/accounting/statements/ | statements::list_statements | done | accounting_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| POST | /api/v1/accounting/statements/ | statements::upload_statement | partial | — | no test |
| GET | /api/v1/accounting/statements/{id}/lines | statements::list_statement_lines | done | accounting_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| GET | /api/v1/accounting/lines/{id}/matches | matches::list_matches | done | accounting_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| POST | /api/v1/accounting/matches/{id}/confirm | matches::confirm_match | done | accounting_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |
| POST | /api/v1/accounting/matches/{id}/reject | matches::reject_match | done | accounting_happy_path_tests.rs | happy-path 2xx (BIT-415 reconcile) |

## Summary
- done: 8 | partial: 226 | stub: 0 | missing: 0 | total: 234

Per-module endpoint counts: financial 33, budgets 37, multi_currency 28, subscriptions 32 (30 + 2 admin), person_months 10 (7 unit + 3 building), market_pricing 31, property_valuation 37, reports 13, accounting 13.
