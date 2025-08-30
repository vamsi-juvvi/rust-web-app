use crate::middleware::mw_auth;
use crate::middleware::mw_auth::CtxW;
use crate::middleware::mw_req_stamp::ReqStamp;
use crate::utils::web_client::WebClient;
use crate::utils::service_resolution::resolve_service;

use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::Json;
use lib_core::ctx::{Ctx, ReqChainLink};
use lib_utils::proc;
use rpc_router::resources_builder;
use serde_json::{json, Value};
use tracing::{error, debug};
use std::sync::Arc;

/// RPC ID and Method Capture
/// Note: This will be injected into the Axum Response extensions so that
///       it can be used downstream by the `mw_res_map` for logging and eventual
///       error client JSON-RPC serialization
#[derive(Debug)]
pub struct RpcInfo {
	pub id: Option<Value>,
	pub method: String,
}

pub async fn rpc_axum_handler(
	State(rpc_router): State<rpc_router::Router>,
	ctx: CtxW,
	req_stamp: ReqStamp,
	Json(rpc_req): Json<Value>,
) -> Response {	
	let ctx = ctx.0;	

	// -- Parse and RpcRequest validate the rpc_request
	let rpc_req = match rpc_router::RpcRequest::try_from(rpc_req) {
		Ok(rpc_req) => rpc_req,
		Err(rpc_req_error) => {
			let res = crate::Error::RpcRequestParsing(rpc_req_error).into_response();
			return res;
		}
	};

	// -- Create the RPC Info
	//    (will be set to the response.extensions)
	let rpc_info = RpcInfo {
		id: Some(rpc_req.id.to_value()),
		method: rpc_req.method.clone(),
	};

	// -- Dispatch the call based on the form of the requested rpc method 
	let json_result = match rpc_info.method.clone().split_once('/') {
		Some((service,method)) => {
			// Split per the "service / method" pattern
			debug!("{:<12} - Split {:?} into service={service:?} and method={method:?}", "RPC Dispatch", &rpc_req.method);
			do_rpc_handler_dispatch_server(ctx, req_stamp, rpc_req, service, method, &rpc_info).await
		},
		None => {
			// No split so assume in-proc RPC call.
			do_rpc_handler_dispatch_inproc(ctx, rpc_router, rpc_req).await
		}
	};	

	// -- Create and Update Axum Response
	// Note: We store data in the Axum Response extensions so that
	//       we can unpack it in the `mw_res_map` for client-side rendering.
	//       This approach centralizes error handling for the client at the `mw_res_map` module
	let res: crate::error::Result<_> = json_result.map_err(crate::error::Error::from);
	let mut res = res.into_response();
	// Note: Here, add the capture RpcInfo (RPC ID and method) into the Axum response to be used
	//       later in the `mw_res_map` for RequestLineLogging, and eventual JSON-RPC error serialization.
	res.extensions_mut().insert(Arc::new(rpc_info));

	res
}

async fn do_rpc_handler_dispatch_inproc(	
	ctx: Ctx,
	rpc_router: rpc_router::Router,
	rpc_req: rpc_router::Request,
) 
-> Result<Json<Value>, rpc_router::CallError> {	
	//-- Add the request specific resources
	// Note: Since Ctx is per axum request, we construct additional RPC resources.
	//       These additional resources will be "overlayed" on top of the base router services,
	//       meaning they will take precedence over the base router ones, but won't replace them.
	let additional_resources = resources_builder![ctx].build();

	// -- Exec Rpc Route
	let rpc_call_result = rpc_router
		.call_with_resources(rpc_req, additional_resources)
		.await;

	// -- Build Json Rpc Success Response
	// Note: Error Json response will be generated in the mw_res_map as wil other error.
	rpc_call_result.map(|rpc_call_response| {
		let body_response = json!({
			"jsonrpc": "2.0",
			"id": rpc_call_response.id,
			"result": rpc_call_response.value
		});
		Json(body_response)
	})
}

async fn do_rpc_handler_dispatch_server(	
	ctx: Ctx,
	req_stamp: ReqStamp,
	rpc_req: rpc_router::Request,
	service : &str,
	method : &str,
	rpc_info: &RpcInfo
) 
-> Result<Json<Value>, rpc_router::CallError> {	

	// build up the request chain by adding this current ReqStamp's UUID to 
	// the ctx which'll be sent downstream.
	let ctx = _add_curr_req_to_chain(ctx, req_stamp);

	// Build the headers which will simply contain the serialized Ctx for now
	// Workers are expected to resolve it frmo the headers	
	// FIXME - Mark the headers as secure via HeaderVal.set_sensitive()..
	// FIXME - Better error, for now map-to unknown method error.
	let web_req_headers = mw_auth::get_ctx_headers(&ctx)
		.map(|h| vec![h])
		.map_err(|_| 
			_to_rpc_error("Converting Ctx to header", "Dispatch RPC", &rpc_info)
		)?;

	// Resolve the service to a URL or Fail
	let srd = resolve_service(service)
		.map_err(|e| 
			_to_rpc_error(&e.to_string(), "Dispatch RPC", &rpc_info)
		)?;

	// FIXME: Hardcode for now
	let url = format!("http://{}:{}/api/rpc", &srd.host, &srd.port);
	debug!("{:<12} - Resolved {service:?} to {url:?}", "RPC Dispatch");

	// Repack the request into a jrpc block with the resolved method name
	let web_payload = json!({
		"jsonrpc": "2.0",
		"id": rpc_req.id,
		"method": method,
		"params": rpc_req.params.unwrap(),
	});

	// FIXME: Validate that the params are not empty. Or is it already done!
	// FIXME: Proper error in case web-req fails (Simulate with false URL)
	let web_res = WebClient
		::default()
		.do_post(url.as_str(), &web_req_headers, web_payload)
		.await
		.map_err(|e| 
			_to_rpc_error(&e.to_string(), "Dispatch RPC", &rpc_info)
		)?;

	debug!("{:<12} - WebResponse status {:?}", "RPC Dispatch", web_res.status);

	Ok(Json(web_res.body))
}

fn _to_rpc_error(
	err_msg: &str, 
	err_cat: &str,
	rpc_info: &RpcInfo) 
-> rpc_router::CallError 
	{		
		error!("{:<12} - rpc dispatch error {err_msg:?}", err_cat);

		rpc_router::CallError {
			id: rpc_info.id.clone().unwrap_or("".into()),
			method: rpc_info.method.clone(),
			error: rpc_router::Error::MethodUnknown
		}
}

fn _add_curr_req_to_chain(
	ctx: Ctx, 
	req_stamp: ReqStamp) 
-> Ctx {
	let proc_name = proc::prog_name()
		.unwrap_or("Unknown".to_string());

	let ctx = ctx.add_req_chain_link(ReqChainLink{
		service: proc_name,
		uuid: req_stamp.uuid
	});

	ctx
}