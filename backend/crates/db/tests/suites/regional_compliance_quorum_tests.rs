//! `jurisdiction_rules` quorum-lookup tests (#1906 findings 1 + 2).
//!
//! These pin two things the post-merge review flagged:
//!   1. `get_quorum_rule` resolves a seeded rule ONLY by its snake_case
//!      `decision_type` key (`decision_type_key()`), NOT by `legal_reference()`
//!      — the validate handlers used the latter, so the seeded quorum was dead
//!      code that always fell back to the in-code default.
//!   2. The Czech three-quarters legal reference is consistent between the seed
//!      and the code (`SS 1209`) after the 00210 forward-fix.

use db::models::regional_compliance::{Jurisdiction, SlovakDecisionType};
use db::repositories::RegionalComplianceRepository;
use rust_decimal::Decimal;
use sqlx::PgPool;

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn quorum_rule_resolves_by_decision_type_key_not_legal_reference(pool: PgPool) {
    let repo = RegionalComplianceRepository::new(pool.clone());

    // (1a) Looking up by the snake_case key the column is seeded with resolves
    // the seeded rule — two_thirds_majority → 66.67% (this is the path the fixed
    // validate handlers now take via `decision_type_key()`).
    let by_key = repo
        .get_quorum_rule(
            &pool,
            Jurisdiction::Slovakia,
            SlovakDecisionType::TwoThirdsMajority.decision_type_key(),
        )
        .await
        .expect("query ok");
    let (quorum, _legal) = by_key.expect("seeded two_thirds_majority rule must resolve by key");
    assert_eq!(
        quorum,
        Decimal::new(6667, 2),
        "two_thirds_majority must resolve to the seeded 66.67% quorum"
    );

    // (1b) Looking up by `legal_reference()` — what the handlers passed BEFORE
    // the fix — never matches the seeded `decision_type` keys, proving that path
    // was dead code that always hit the in-code fallback.
    let by_legal_ref = repo
        .get_quorum_rule(
            &pool,
            Jurisdiction::Slovakia,
            SlovakDecisionType::TwoThirdsMajority.legal_reference(),
        )
        .await
        .expect("query ok");
    assert!(
        by_legal_ref.is_none(),
        "looking up by legal_reference() must NOT match a seeded rule (the dead-code bug)"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn czech_three_quarters_statute_is_consistent_after_forward_fix(pool: PgPool) {
    let repo = RegionalComplianceRepository::new(pool.clone());

    let rule = repo
        .get_quorum_rule(&pool, Jurisdiction::Czechia, "three_quarters_majority")
        .await
        .expect("query ok")
        .expect("seeded czech three_quarters_majority rule");

    assert_eq!(rule.0, Decimal::new(7500, 2), "75% quorum");
    // Migration 00210 aligned the seed to the code value (SS 1209), not SS 1210.
    assert_eq!(
        rule.1, "SS 1209 zakona 89/2012 Sb.",
        "seed legal_reference must match CzechDecisionType::ThreeQuartersMajority::legal_reference() (#1906 finding-2)"
    );
}
