// Negative: GET-shape handler (non-mutating fn name) may legitimately
// discard the principal — the lint should NOT flag this.
async fn get_thing(
    State(state): State<AppState>,
    _principal: RequestPrincipal,
    Path(id): Path<Uuid>,
) -> Result<Json<Thing>, AppError> {
    let t = state.repo.find(id).await?;
    Ok(Json(t))
}
