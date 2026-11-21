//! ChatW Plugin Platform Backend Router & Submodules.

pub mod installations;
pub mod marketplace;
pub mod schema;
pub mod security;
pub mod webhooks;

use axum::{
    Router,
    routing::{delete, get, patch, post, put},
};

use crate::auth::AppState;

pub fn plugin_routes() -> Router<AppState> {
    Router::new()
        .route("/permissions", get(marketplace::get_permission_catalog))
        .route("/publishers/me", get(marketplace::get_my_publisher))
        .route("/publishers", post(marketplace::register_publisher))
        .route("/publishers/verify", post(marketplace::request_publisher_verification))
        .route("/packages", post(marketplace::publish_package))
        .route("/packages/{id}/versions", post(marketplace::publish_version))
        .route("/packages/{id}/rollback", post(marketplace::rollback_package_version))
        .route("/versions/{id}/revoke", post(marketplace::revoke_version))
        .route("/marketplace", get(marketplace::list_marketplace))
        .route("/marketplace/{id_or_key}", get(marketplace::get_marketplace_detail))
        .route("/marketplace/{id}/reviews", post(marketplace::upsert_review))
        .route("/marketplace/{id}/report", post(marketplace::report_package))
        .route("/install/preview", post(installations::preview_install))
        .route("/installations", get(installations::list_installations).post(installations::create_installation))
        .route(
            "/installations/{id}",
            patch(installations::update_installation).delete(installations::delete_installation),
        )
        .route("/installations/{id}/grants", post(installations::grant_permission))
        .route("/installations/{id}/grants/{grant_id}", delete(installations::revoke_permission))
        .route("/installations/{id}/audit", post(installations::record_plugin_audit))
        .route("/installations/{id}/storage", get(installations::list_plugin_storage))
        .route(
            "/installations/{id}/storage/{key}",
            put(installations::put_plugin_storage).delete(installations::delete_plugin_storage),
        )
        .route("/installations/{id}/net/fetch", post(installations::plugin_network_fetch))
        .route(
            "/installations/{id}/bots",
            get(installations::list_plugin_bots).post(installations::register_plugin_bot),
        )
        .route("/privacy", get(installations::privacy_dashboard))
        .route("/sync", get(installations::sync_plugins_state))
}

pub fn webhook_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(webhooks::list_webhooks).post(webhooks::create_webhook))
        .route(
            "/{id}",
            patch(webhooks::update_webhook).delete(webhooks::delete_webhook),
        )
        .route("/{id}/rotate-token", post(webhooks::rotate_webhook_token))
        .route("/{id}/deliveries", get(webhooks::list_webhook_deliveries))
        .route("/{id}/test", post(webhooks::test_webhook))
        // Public Discord-compatible Incoming Webhook execution endpoint
        .route("/{id}/{token}", post(webhooks::execute_incoming_webhook))
}

pub fn admin_plugin_routes() -> Router<AppState> {
    Router::new()
        .route("/publishers/{id}/{decision}", post(marketplace::admin_decide_publisher))
        .route("/versions/{id}/permissions", post(marketplace::admin_decide_version_permissions))
}
