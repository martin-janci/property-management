// Positive: classic _principal discard on a mutating handler.
async fn delete_thing(
    State(state): State<AppState>,
    _principal: RequestPrincipal,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    state.repo.delete(id).await?;
    Ok(StatusCode::NO_CONTENT)
}
