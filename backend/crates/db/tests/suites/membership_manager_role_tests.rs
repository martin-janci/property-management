//! `MembershipRepository::is_manager_in_org` / `list_manager_ids` must accept
//! the snake_case roles that are actually stored in `user_memberships`
//! (`manager`, `org_admin`, `property_manager`, ...) as well as the legacy
//! PascalCase form. The checks used to match PascalCase only, so every real
//! manager was denied manager-only fault actions and never received
//! new-fault notifications.

use crate::common::seed_org;
use db::repositories::MembershipRepository;
use sqlx::PgPool;
use uuid::Uuid;

async fn member(pool: &PgPool, org: Uuid, email: &str, role: &str) -> Uuid {
    let user: Uuid = sqlx::query_scalar(
        "INSERT INTO users (email, password_hash, name, status, email_verified_at)
         VALUES ($1, 'test_hash', 'Member', 'active', NOW()) RETURNING id",
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user");
    sqlx::query(
        "INSERT INTO user_memberships (user_id, organization_id, role) VALUES ($1, $2, $3)",
    )
    .bind(user)
    .bind(org)
    .bind(role)
    .execute(pool)
    .await
    .expect("seed membership");
    user
}

#[sqlx::test(migrations = "./migrations")]
async fn manager_tier_roles_match_in_both_spellings(pool: PgPool) {
    let org = seed_org(&pool, "mgr-roles").await;
    let repo = MembershipRepository::new(pool.clone());

    let mut managers = Vec::new();
    for (i, role) in [
        "manager",
        "org_admin",
        "property_manager",
        "technical_manager",
        "Manager",
        "OrgAdmin",
    ]
    .iter()
    .enumerate()
    {
        let id = member(&pool, org, &format!("mgr{i}@roles.test"), role).await;
        assert!(
            repo.is_manager_in_org(id, org).await.expect("query"),
            "role {role} must count as manager"
        );
        managers.push(id);
    }

    for (i, role) in ["owner", "tenant", "resident", "owner_delegate"]
        .iter()
        .enumerate()
    {
        let id = member(&pool, org, &format!("res{i}@roles.test"), role).await;
        assert!(
            !repo.is_manager_in_org(id, org).await.expect("query"),
            "role {role} must not count as manager"
        );
    }

    let mut listed = repo.list_manager_ids(org).await.expect("list");
    listed.sort();
    managers.sort();
    assert_eq!(listed, managers);
}
