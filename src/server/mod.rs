use axum::Router;
use tokio_util::sync::CancellationToken;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::error::Result;
use crate::index::manager::IndexManager;
use crate::index::ops;

mod docs;
mod index;
mod maintenance;
mod search;
mod ui;

#[derive(Clone)]
pub struct AppState {
    pub manager: IndexManager,
}

pub async fn serve(manager: IndexManager, bind: &str) -> Result<()> {
    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();

    // Start background task to periodically commit indexes.
    let commit_manager = manager.clone();
    let commit_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let handles = commit_manager.iter_handles();
                    for handle in handles {
                        let name = handle.name.clone();
                        if let Err(e) = ops::commit_index(handle).await {
                            tracing::error!(index = %name, error = %e, "failed to commit index");
                        }
                    }
                }
                _ = cancel_clone.cancelled() => {
                    tracing::info!("commit task shutting down");
                    // Final commit before exit.
                    let handles = commit_manager.iter_handles();
                    for handle in handles {
                        let name = handle.name.clone();
                        if let Err(e) = ops::commit_index(handle).await {
                            tracing::error!(index = %name, error = %e, "final commit failed");
                        }
                    }
                    break;
                }
            }
        }
    });

    // Start background task to periodically clean up expired documents.
    let cleanup_manager = manager.clone();
    let cleanup_cancel = CancellationToken::new();
    let cleanup_cancel_clone = cleanup_cancel.clone();
    let cleanup_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let handles = cleanup_manager.iter_handles();
                    for handle in handles {
                        if handle.expired_at_field.is_some() {
                            let name = handle.name.clone();
                            if let Err(e) = ops::cleanup_expired(handle).await {
                                tracing::error!(index = %name, error = %e, "failed to cleanup expired documents");
                            }
                        }
                    }
                }
                _ = cleanup_cancel_clone.cancelled() => {
                    tracing::info!("cleanup task shutting down");
                    break;
                }
            }
        }
    });

    let state = AppState { manager };

    let app = Router::new()
        .route("/", axum::routing::get(ui::index))
        .route("/indexes", axum::routing::get(index::list_indexes))
        .route(
            "/indexes/{name}",
            axum::routing::post(index::create_index)
                .delete(index::delete_index)
                .get(index::get_index_info),
        )
        .route(
            "/indexes/{name}/docs",
            axum::routing::post(docs::add_doc).get(docs::list_docs),
        )
        .route(
            "/indexes/{name}/docs/_bulk",
            axum::routing::post(docs::bulk_add_docs),
        )
        .route(
            "/indexes/{name}/docs/{field}/{value}",
            axum::routing::get(docs::get_doc).delete(docs::delete_doc),
        )
        .route(
            "/indexes/{name}/search",
            axum::routing::get(search::search).post(search::search_post),
        )
        .route(
            "/indexes/{name}/stats",
            axum::routing::get(maintenance::index_stats),
        )
        .route(
            "/indexes/{name}/rebuild",
            axum::routing::post(maintenance::rebuild_index),
        )
        .route(
            "/indexes/{name}/compress",
            axum::routing::post(maintenance::compress_index),
        )
        .route("/{*path}", axum::routing::get(ui::static_file))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(bind).await?;
    tracing::info!(%bind, "server listening");

    // Run the server until graceful shutdown (Ctrl+C or SIGTERM).
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    // Signal background tasks to stop and wait for them.
    cancel.cancel();
    cleanup_cancel.cancel();
    if let Err(e) = commit_handle.await {
        tracing::error!(error = %e, "commit task panicked or was cancelled");
    }
    if let Err(e) = cleanup_handle.await {
        tracing::error!(error = %e, "cleanup task panicked or was cancelled");
    }

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        match tokio::signal::ctrl_c().await {
            Ok(()) => tracing::info!("received Ctrl+C"),
            Err(e) => tracing::error!(error = %e, "failed to listen for Ctrl+C"),
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut stream) => {
                stream.recv().await;
                tracing::info!("received SIGTERM");
            }
            Err(e) => {
                tracing::error!(error = %e, "failed to install SIGTERM handler");
                std::future::pending::<()>().await
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
