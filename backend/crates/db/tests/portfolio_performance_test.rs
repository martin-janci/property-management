//! Integration tests for Portfolio Performance Analytics (Epic 144).

use chrono::NaiveDate;
use db::models::portfolio_performance::*;
use rust_decimal::Decimal;
use uuid::Uuid;

/// Test Story 144.1: Portfolio Configuration
#[cfg(test)]
mod portfolio_configuration_tests {
    use super::*;

    #[test]
    fn test_create_performance_portfolio_request() {
        let req = CreatePerformancePortfolio {
            name: "Test Portfolio".to_string(),
            description: Some("A test investment portfolio".to_string()),
            target_return_pct: Some(Decimal::from(12)),
            target_exit_year: Some(2030),
            investment_strategy: Some("Value-add".to_string()),
            currency: "EUR".to_string(),
        };

        assert_eq!(req.name, "Test Portfolio");
        assert!(req.description.is_some());
        assert_eq!(req.target_return_pct, Some(Decimal::from(12)));
    }

    #[test]
    fn test_create_portfolio_property_request() {
        let req = CreatePortfolioProperty {
            building_id: Uuid::new_v4(),
            property_name: Some("Downtown Office".to_string()),
            acquisition_date: NaiveDate::from_ymd_opt(2023, 6, 15).unwrap(),
            acquisition_price: Decimal::from(500000),
            acquisition_costs: Some(Decimal::from(25000)),
            financing_type: FinancingType::Mortgage,
            down_payment: Some(Decimal::from(125000)),
            loan_amount: Some(Decimal::from(400000)),
            interest_rate: Some(Decimal::new(450, 2)), // 4.50%
            loan_term_years: Some(30),
            monthly_payment: Some(Decimal::from(2027)),
            loan_start_date: Some(NaiveDate::from_ymd_opt(2023, 7, 1).unwrap()),
            ownership_percentage: Decimal::from(100),
            current_value: Some(Decimal::from(550000)),
            currency: "EUR".to_string(),
            notes: None,
        };

        assert_eq!(req.acquisition_price, Decimal::from(500000));
        assert_eq!(req.financing_type, FinancingType::Mortgage);
        assert!(req.loan_amount.is_some());
    }

    #[test]
    fn test_financing_type_enum() {
        assert_eq!(format!("{:?}", FinancingType::Cash), "Cash");
        assert_eq!(format!("{:?}", FinancingType::Mortgage), "Mortgage");
        assert_eq!(format!("{:?}", FinancingType::Commercial), "Commercial");
        assert_eq!(
            format!("{:?}", FinancingType::PrivateLending),
            "PrivateLending"
        );
        assert_eq!(format!("{:?}", FinancingType::Partnership), "Partnership");
        assert_eq!(format!("{:?}", FinancingType::Syndication), "Syndication");
        assert_eq!(format!("{:?}", FinancingType::Mixed), "Mixed");
    }
}

/// Test Story 144.2: Income & Expense Tracking
#[cfg(test)]
mod income_expense_tests {
    use super::*;

    #[test]
    fn test_create_property_transaction() {
        let req = CreatePropertyTransaction {
            property_id: Uuid::new_v4(),
            transaction_type: PortfolioTransactionType::RentalIncome,
            category: Some("Unit 101".to_string()),
            amount: Decimal::from(1500),
            currency: "EUR".to_string(),
            transaction_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            period_start: Some(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
            period_end: Some(NaiveDate::from_ymd_opt(2024, 1, 31).unwrap()),
            description: Some("January rent".to_string()),
            vendor_name: Some("John Doe".to_string()),
            reference_number: None,
            document_id: None,
            is_recurring: true,
            recurrence_frequency: Some("monthly".to_string()),
        };

        assert_eq!(req.transaction_type, PortfolioTransactionType::RentalIncome);
        assert_eq!(req.amount, Decimal::from(1500));
        assert!(req.is_recurring);
    }

    #[test]
    fn test_transaction_types() {
        let income_types = [
            PortfolioTransactionType::RentalIncome,
            PortfolioTransactionType::OtherIncome,
        ];

        let expense_types = [
            PortfolioTransactionType::OperatingExpense,
            PortfolioTransactionType::MortgagePayment,
            PortfolioTransactionType::CapitalExpenditure,
            PortfolioTransactionType::TaxPayment,
            PortfolioTransactionType::Insurance,
            PortfolioTransactionType::PropertyManagement,
            PortfolioTransactionType::Maintenance,
            PortfolioTransactionType::Utilities,
            PortfolioTransactionType::VacancyCost,
            PortfolioTransactionType::LeasingCost,
            PortfolioTransactionType::LegalProfessional,
            PortfolioTransactionType::Other,
        ];

        assert_eq!(income_types.len(), 2);
        assert_eq!(expense_types.len(), 12);
    }

    #[test]
    fn test_upsert_cash_flow() {
        let req = UpsertPropertyCashFlow {
            property_id: Uuid::new_v4(),
            period_year: 2024,
            period_month: 1,
            gross_rental_income: Decimal::from(5000),
            other_income: Some(Decimal::from(200)),
            operating_expenses: Decimal::from(1500),
            mortgage_payment: Some(Decimal::from(2000)),
            capital_expenditures: Some(Decimal::from(500)),
            vacancy_rate: Some(Decimal::from(5)),
            vacancy_cost: Some(Decimal::from(250)),
            currency: "EUR".to_string(),
        };

        assert_eq!(req.period_year, 2024);
        assert_eq!(req.period_month, 1);
        assert_eq!(req.gross_rental_income, Decimal::from(5000));
    }
}

/// Test Story 144.3: ROI & Financial Metrics Calculator
#[cfg(test)]
mod financial_metrics_tests {
    use super::*;

    #[test]
    fn test_calculate_metrics_request() {
        let req = CalculateMetricsRequest {
            property_id: Some(Uuid::new_v4()),
            period_type: MetricPeriod::Annual,
            period_start: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            period_end: NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
            property_value: Some(Decimal::from(500000)),
            total_investment: Some(Decimal::from(125000)),
            discount_rate: Some(Decimal::new(10, 2)),
        };

        assert!(req.property_id.is_some());
        assert_eq!(req.period_type, MetricPeriod::Annual);
    }

    #[test]
    fn test_metric_periods() {
        assert_eq!(format!("{:?}", MetricPeriod::Monthly), "Monthly");
        assert_eq!(format!("{:?}", MetricPeriod::Quarterly), "Quarterly");
        assert_eq!(format!("{:?}", MetricPeriod::Annual), "Annual");
        assert_eq!(format!("{:?}", MetricPeriod::Ytd), "Ytd");
        assert_eq!(
            format!("{:?}", MetricPeriod::SinceInception),
            "SinceInception"
        );
    }

    #[test]
    fn test_cap_rate_calculation() {
        // Cap Rate = NOI / Property Value * 100
        let noi = Decimal::from(50000);
        let property_value = Decimal::from(500000);
        let cap_rate = (noi / property_value) * Decimal::from(100);

        assert_eq!(cap_rate, Decimal::from(10)); // 10%
    }

    #[test]
    fn test_cash_on_cash_calculation() {
        // Cash-on-Cash = Annual Cash Flow / Total Cash Invested * 100
        let annual_cash_flow = Decimal::from(15000);
        let total_cash_invested = Decimal::from(125000);
        let coc = (annual_cash_flow / total_cash_invested) * Decimal::from(100);

        assert_eq!(coc, Decimal::from(12)); // 12%
    }

    #[test]
    fn test_dscr_calculation() {
        // DSCR = NOI / Annual Debt Service
        let noi = Decimal::from(50000);
        let annual_debt_service = Decimal::from(40000);
        let dscr = noi / annual_debt_service;

        assert_eq!(dscr, Decimal::new(125, 2)); // 1.25x
    }
}

/// Test Story 144.4: Performance Benchmarking
#[cfg(test)]
mod benchmarking_tests {
    use super::*;

    #[test]
    fn test_create_market_benchmark() {
        let req = CreateMarketBenchmark {
            name: "Regional Multi-Family Index".to_string(),
            description: Some(
                "Average performance for multi-family properties in the region".to_string(),
            ),
            source: BenchmarkSource::Industry,
            source_name: Some("Real Estate Research Institute".to_string()),
            source_url: Some("https://example.com/benchmark".to_string()),
            source_date: Some(NaiveDate::from_ymd_opt(2024, 6, 30).unwrap()),
            property_type: Some("Multi-Family".to_string()),
            region: Some("Central Europe".to_string()),
            market: Some("Prague".to_string()),
            period_year: 2024,
            period_quarter: Some(2),
            avg_cap_rate: Some(Decimal::new(650, 2)), // 6.50%
            avg_cash_on_cash: Some(Decimal::new(980, 2)), // 9.80%
            avg_noi_per_unit: Some(Decimal::from(8500)),
            avg_price_per_unit: Some(Decimal::from(130000)),
            avg_price_per_sqm: Some(Decimal::from(3200)),
            avg_occupancy: Some(Decimal::new(9520, 2)), // 95.20%
            avg_rent_growth: Some(Decimal::new(350, 2)), // 3.50%
            avg_expense_ratio: Some(Decimal::new(3500, 2)), // 35.00%
            avg_irr: Some(Decimal::new(1200, 2)),       // 12.00%
            avg_equity_multiple: Some(Decimal::new(185, 2)), // 1.85x
            currency: "EUR".to_string(),
        };

        assert_eq!(req.name, "Regional Multi-Family Index");
        assert_eq!(req.source, BenchmarkSource::Industry);
        assert_eq!(req.period_year, 2024);
    }

    #[test]
    fn test_benchmark_sources() {
        assert_eq!(format!("{:?}", BenchmarkSource::Industry), "Industry");
        assert_eq!(format!("{:?}", BenchmarkSource::Regional), "Regional");
        assert_eq!(
            format!("{:?}", BenchmarkSource::PropertyType),
            "PropertyType"
        );
        assert_eq!(format!("{:?}", BenchmarkSource::Custom), "Custom");
        assert_eq!(format!("{:?}", BenchmarkSource::NcreifOdce), "NcreifOdce");
        assert_eq!(format!("{:?}", BenchmarkSource::MsciIpd), "MsciIpd");
        assert_eq!(
            format!("{:?}", BenchmarkSource::NarreitIndex),
            "NarreitIndex"
        );
    }

    #[test]
    fn test_create_benchmark_comparison() {
        let req = CreateBenchmarkComparison {
            benchmark_id: Uuid::new_v4(),
            property_id: Some(Uuid::new_v4()),
            comparison_date: NaiveDate::from_ymd_opt(2024, 6, 30).unwrap(),
        };

        assert!(req.property_id.is_some());
    }
}

/// Test Story 144.5: Portfolio Analytics Dashboard
#[cfg(test)]
mod dashboard_tests {
    use super::*;

    #[test]
    fn test_dashboard_query() {
        let query = DashboardQuery {
            as_of_date: Some(NaiveDate::from_ymd_opt(2024, 6, 30).unwrap()),
            period_months: Some(12),
            include_benchmark: Some(true),
            benchmark_id: Some(Uuid::new_v4()),
        };

        assert!(query.as_of_date.is_some());
        assert_eq!(query.period_months, Some(12));
        assert_eq!(query.include_benchmark, Some(true));
    }

    #[test]
    fn test_dashboard_summary_structure() {
        let summary = DashboardSummary {
            total_portfolio_value: Decimal::from(2500000),
            total_equity: Decimal::from(1200000),
            total_debt: Decimal::from(1300000),
            debt_to_equity_ratio: Some(Decimal::new(108, 2)), // 1.08
            ltv_ratio: Some(Decimal::from(52)),
            ytd_noi: Decimal::from(180000),
            ytd_cash_flow: Decimal::from(95000),
            ytd_return_pct: Some(Decimal::new(79, 1)), // 7.9%
            property_count: 4,
            total_units: Some(24),
            occupied_units: Some(23),
            occupancy_rate: Some(Decimal::new(9583, 2)), // 95.83%
            currency: "EUR".to_string(),
            as_of_date: NaiveDate::from_ymd_opt(2024, 6, 30).unwrap(),
        };

        assert_eq!(summary.property_count, 4);
        assert_eq!(summary.total_portfolio_value, Decimal::from(2500000));
    }

    #[test]
    fn test_property_performance_card_statuses() {
        let statuses = vec!["excellent", "good", "fair", "needs_attention", "no_data"];

        for status in &statuses {
            let card = PropertyPerformanceCard {
                property_id: Uuid::new_v4(),
                property_name: "Test Property".to_string(),
                building_address: None,
                current_value: Decimal::from(500000),
                equity: Decimal::from(250000),
                ltv: Some(Decimal::from(50)),
                noi: Decimal::from(40000),
                cap_rate: Some(Decimal::from(8)),
                cash_on_cash: Some(Decimal::from(12)),
                dscr: Some(Decimal::new(125, 2)),
                occupancy_rate: Some(Decimal::from(95)),
                monthly_cash_flow: Some(Decimal::from(2500)),
                vs_benchmark_pct: None,
                performance_status: status.to_string(),
                currency: "EUR".to_string(),
            };

            assert!(!card.performance_status.is_empty());
        }
    }

    #[test]
    fn test_cash_flow_trend_point() {
        let point = CashFlowTrendPoint {
            period: "2024-01".to_string(),
            period_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            gross_income: Decimal::from(18000),
            operating_expenses: Decimal::from(5000),
            noi: Decimal::from(13000),
            debt_service: Some(Decimal::from(8000)),
            net_cash_flow: Decimal::from(5000),
        };

        // Verify NOI = Gross Income - Operating Expenses
        assert_eq!(point.noi, point.gross_income - point.operating_expenses);

        // Verify Net Cash Flow = NOI - Debt Service
        if let Some(debt) = point.debt_service {
            assert_eq!(point.net_cash_flow, point.noi - debt);
        }
    }

    #[test]
    fn test_create_performance_alert() {
        let req = CreatePerformanceAlert {
            property_id: Some(Uuid::new_v4()),
            alert_type: "metric_threshold".to_string(),
            severity: "warning".to_string(),
            title: "DSCR Below Target".to_string(),
            message: "Property DSCR has fallen below your target of 1.25x".to_string(),
            metric_name: Some("DSCR".to_string()),
            current_value: Some(Decimal::new(115, 2)),
            threshold_value: Some(Decimal::new(125, 2)),
        };

        assert_eq!(req.alert_type, "metric_threshold");
        assert_eq!(req.severity, "warning");
        assert!(req.current_value.is_some());
    }
}

/// Test export functionality
#[cfg(test)]
mod export_tests {
    use super::*;

    #[test]
    fn test_export_portfolio_report_request() {
        let req = ExportPortfolioReport {
            period_start: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            period_end: NaiveDate::from_ymd_opt(2024, 6, 30).unwrap(),
            include_properties: true,
            include_transactions: true,
            include_metrics: true,
            include_benchmark: true,
            format: "pdf".to_string(),
        };

        assert!(req.include_properties);
        assert!(req.include_transactions);
        assert!(req.include_metrics);
        assert!(req.include_benchmark);
        assert_eq!(req.format, "pdf");
    }

    #[test]
    fn test_export_formats() {
        let formats = vec!["pdf", "xlsx", "csv"];

        for format in formats {
            let req = ExportPortfolioReport {
                period_start: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                period_end: NaiveDate::from_ymd_opt(2024, 6, 30).unwrap(),
                include_properties: true,
                include_transactions: false,
                include_metrics: true,
                include_benchmark: false,
                format: format.to_string(),
            };

            assert!(!req.format.is_empty());
        }
    }
}
