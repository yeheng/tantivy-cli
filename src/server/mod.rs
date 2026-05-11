use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::error::Result;
use crate::index::manager::IndexManager;
use crate::index::ops;

mod docs;
mod index;
mod maintenance;
mod search;

#[derive(Clone)]
pub struct AppState {
    pub manager: IndexManager,
}

pub async fn serve(manager: IndexManager, bind: &str) -> Result<()> {
    // Start background task to periodically commit indexes.
    // Note: `ops::commit_index` delegates to the per-index writer actor, which
    // tracks a `dirty` flag and only performs an actual Tantivy commit when
    // there are pending changes. Idle indexes produce zero disk I/O.
    let commit_manager = manager.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            let handles = commit_manager.iter_handles();
            for handle in handles {
                if let Err(e) = ops::commit_index(&handle).await {
                    tracing::error!(index = %handle.name, error = %e, "failed to commit index");
                }
            }
        }
    });

    // Start background task to periodically clean up expired documents.
    let cleanup_manager = manager.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            let handles = cleanup_manager.iter_handles();
            for handle in handles {
                if handle.expired_at_field.is_some() {
                    if let Err(e) = ops::cleanup_expired(&handle).await {
                        tracing::error!(index = %handle.name, error = %e, "failed to cleanup expired documents");
                    }
                }
            }
        }
    });

    let state = AppState { manager };

    let app = Router::new()
        .route("/indexes", axum::routing::get(index::list_indexes))
        .route(
            "/indexes/{name}",
            axum::routing::post(index::create_index)
                .delete(index::delete_index)
                .get(index::get_index_info),
        )
        .route("/indexes/{name}/docs", axum::routing::post(docs::add_doc).get(docs::list_docs))
        .route(
            "/indexes/{name}/docs/{field}/{value}",
            axum::routing::get(docs::get_doc).delete(docs::delete_doc),
        )
        .route("/indexes/{name}/search", axum::routing::get(search::search).post(search::search_post))
        .route("/indexes/{name}/stats", axum::routing::get(maintenance::index_stats))
        .route("/indexes/{name}/rebuild", axum::routing::post(maintenance::rebuild_index))
        .route("/indexes/{name}/compress", axum::routing::post(maintenance::compress_index))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(bind).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
