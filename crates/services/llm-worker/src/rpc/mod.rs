use lib_rpc_core::prelude::*;

use serde::Deserialize;
use rpc_router::IntoParams;
use serde::de::DeserializeOwned;

mod genai_chat_rpc;
use genai_chat_rpc::one_shot_msg;

mod genai_model_rpc;
use genai_model_rpc::get_model_list;

/// Equivalent to 
///  RouterBuilder::default()
///     .append_dyn("one_shot_msg", one_shot_msg.into_box())
///
pub fn rpc_router_builder() -> RouterBuilder {
    router_builder!(
        one_shot_msg,
        get_model_list
    )
}


//-- Infra extension for wrapping params ------------
/// Params structure for any RPC pass-through call.
#[derive(Debug, Deserialize)]
pub struct ParamsW<D> {
	pub data: D,
}

impl<D> IntoParams for ParamsW<D> where D: DeserializeOwned + Send {}