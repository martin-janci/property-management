use crate::AppState;
use axum::routing::{delete, get, post};
use axum::Router;

pub mod contacts;
pub mod invoices;
pub mod matches;
pub mod statements;

pub fn router() -> Router<AppState> {
    Router::new()
        .nest(
            "/invoices",
            Router::new()
                .route("/", get(invoices::list_invoices).post(invoices::create_invoice))
                .route(
                    "/:id",
                    get(invoices::get_invoice)
                        .patch(invoices::update_invoice)
                        .delete(invoices::delete_invoice),
                )
                .route("/:id/items", get(invoices::list_invoice_items)),
        )
        .nest(
            "/contacts",
            Router::new().route("/", get(contacts::list_contacts)),
        )
        .nest(
            "/statements",
            Router::new()
                .route(
                    "/",
                    get(statements::list_statements).post(statements::upload_statement),
                )
                .route("/:id/lines", get(statements::list_statement_lines)),
        )
        .nest(
            "/lines",
            Router::new().route("/:id/matches", get(matches::list_matches)),
        )
        .nest(
            "/matches",
            Router::new()
                .route("/:id/confirm", post(matches::confirm_match))
                .route("/:id/reject", post(matches::reject_match)),
        )
}
