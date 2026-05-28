// Positive: `_: RequestPrincipal` is the same discard pattern.
async fn update_thing(
    State(state): State<AppState>,
    _: RequestPrincipal,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    state.repo.update(id).await?;
    Ok(StatusCode::NO_CONTENT)
}
