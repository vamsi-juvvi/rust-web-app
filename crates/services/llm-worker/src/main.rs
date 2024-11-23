// region:    --- Modules

mod error;
mod web;
mod rpc;

pub use self::error::{Error, Result};

use lib_web::middleware::mw_auth::{mw_ctx_require, mw_ctx_leaf_resolver};
use lib_web::middleware::mw_req_stamp::mw_req_stamp_resolver;
use lib_web::middleware::mw_res_map::mw_reponse_map;

use axum::{middleware, Router};
use lib_core::model::ModelManager;
use tokio::net::TcpListener;
use tower_cookies::CookieManagerLayer;
use tracing::info;
use tracing_subscriber::EnvFilter;

// endregion: --- Modules

#[tokio::main]
async fn main() -> Result<()> {
	tracing_subscriber::fmt()
		.without_time() // For early local development.
		.with_target(false)
		.with_env_filter(
			EnvFilter::try_from_default_env()
			.unwrap_or_else(|_| EnvFilter::new("debug")))
		.init();

	let mm = ModelManager::new().await?;

	// -- Define Routes
	let routes_rpc = web::routes_rpc::routes(mm.clone())
		.route_layer(middleware::from_fn(mw_ctx_require));

	let routes_all = Router::new()		
		.nest("/api", routes_rpc)
		.layer(middleware::map_response(mw_reponse_map))
		.layer(middleware::from_fn(mw_ctx_leaf_resolver))
		.layer(CookieManagerLayer::new())
		.layer(middleware::from_fn(mw_req_stamp_resolver));

	// region:    --- Start Server
	// Note: For this block, ok to unwrap.
	let listener = TcpListener::bind("127.0.0.1:8081").await.unwrap();
	info!("{:<12} - {:?}\n", "LISTENING", listener.local_addr());
	axum::serve(listener, routes_all.into_make_service())
		.await
		.unwrap();
	// endregion: --- Start Server

	Ok(())
}
