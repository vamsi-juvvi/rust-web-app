#![allow(unused)] // For example code.

pub type Result<T> = core::result::Result<T, Error>;
pub type Error = Box<dyn std::error::Error>; // For examples.

use serde_json::{json, Value};

use reqwest::{Url, header, ClientBuilder, Client};

use lib_core::ctx::Ctx;

// Don't know enough to avoid this.
// This takes reqwest::http::header::HeaderMap while
// mw_auth::set_auth1_header takes an axum::http::header::HeaderMap
fn set_ctx_leaf_header(ctx: &Ctx, headers: &mut header::HeaderMap) -> Result<()> {
	let mut header_val = header::HeaderValue
		::from_str(&serde_json::to_string(&ctx)?)
		.unwrap();

	header_val.set_sensitive(true);
	headers.insert("X-WORKER-POSTAUTH", header_val);

	Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {

	// -- Send a RPC request with a post-auth header		
	let ctx_val = Ctx::new(1001)?;		

	// Send this via headers as a sensitive value
	// printout will just show 
	// {"x-worker-postauth": Sensitive}
	// https://play.rust-lang.org/?version=stable&mode=debug&edition=2021
	let mut request_headers = header::HeaderMap::new();
	set_ctx_leaf_header(&ctx_val, &mut request_headers);		

	let client_builder = ClientBuilder::new()
		.default_headers(request_headers);

	let hc = httpc_test::new_client_with_reqwest(
		"http://localhost:8081",
		client_builder
	)?;

	// - one_shot_msg
	{
		let req_llm_msg = hc.do_post(
			"/api/rpc",
			json!({
				"jsonrpc": "2.0",
				"id": 1,
				"method": "one_shot_msg",
				"params": {
					"data": {
						"prompt": "why is the sky blue",
						"mode" : "System"
					}
				}
			}),
		);
		let result = req_llm_msg.await?;
		result.print().await?;
	}

	//-- get_model_list
	if false {
		let req_get_model_list = hc.do_post(
			"/api/rpc",
			json!({
				"jsonrpc": "2.0",
				"id": 1,
				"method": "get_model_list",
				"params": {
					"data": {
						"provider": "All",						
					}
				}
			}),
		);
		let result = req_get_model_list.await?;
		result.print().await?;
	}

	Ok(())
}
