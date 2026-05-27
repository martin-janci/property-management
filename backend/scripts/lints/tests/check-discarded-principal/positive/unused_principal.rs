// Positive: `principal: RequestPrincipal` bound without underscore but never
// referenced in the body — same discard shape.
async fn deactivate_thing(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    state.repo.deactivate(id).await?;
    Ok(StatusCode::NO_CONTENT)
}
