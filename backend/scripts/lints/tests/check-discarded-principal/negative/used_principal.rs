// Negative: principal IS used for tenant scoping — fine.
async fn delete_owned_thing(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    state.repo.delete_for_user(id, principal.user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
