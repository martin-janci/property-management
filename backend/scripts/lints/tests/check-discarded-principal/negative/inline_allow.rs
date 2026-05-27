// Negative: inline `lint-allow` marker disables the warning.
async fn delete_legit_thing(
    State(state): State<AppState>,
    // lint-allow: discarded-principal — separate `RequireOrgAdmin` extractor enforces ownership
    _principal: RequestPrincipal,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    state.repo.delete(id).await?;
    Ok(StatusCode::NO_CONTENT)
}
