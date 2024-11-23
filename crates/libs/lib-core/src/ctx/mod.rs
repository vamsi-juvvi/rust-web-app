// region:    --- Modules

mod error;

use serde::{Serialize, Deserialize};
use uuid;
use uuid::Uuid;

pub use self::error::{Error, Result};

// endregion: --- Modules
#[cfg_attr(feature = "with-rpc", derive(rpc_router::RpcResource))]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReqChainLink {
	// calling service
	pub service: String, 

	// req-id of service's request.
	pub uuid : Uuid,
}

#[cfg_attr(feature = "with-rpc", derive(rpc_router::RpcResource))]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ctx {
	user_id: i64,	

	/// Note: For the future ACS (Access Control System)
	conv_id: Option<i64>,

	/// Call chain for when this Ctx is sent from gateway down 
	/// to a chain of workers
	req_chain: Option<Vec<ReqChainLink>>,
}

// Constructors.
impl Ctx {
	pub fn root_ctx() -> Self {
		Ctx {
			user_id: 0,
			conv_id: None,
			req_chain : None,
		}
	}

	pub fn new(user_id: i64) -> Result<Self> {
		if user_id == 0 {
			Err(Error::CtxCannotNewRootCtx)
		} else {
			Ok(Self {
				user_id,
				conv_id: None,
				req_chain: None,
			})
		}
	}

	/// Note: For the future ACS (Access Control System)
	pub fn add_conv_id(&self, conv_id: i64) -> Ctx {
		let mut ctx = self.clone();
		ctx.conv_id = Some(conv_id);
		ctx
	}

	pub fn add_req_chain_link(&self, req_link: ReqChainLink) -> Ctx {
		let mut ctx = self.clone();		
		ctx.req_chain = match ctx.req_chain {
			Some(mut v) => { v.push(req_link); Some(v) },
			None => Some(vec![req_link]),			
		};		

		ctx			
	}
}

// Property Accessors.
impl Ctx {
	pub fn user_id(&self) -> i64 {
		self.user_id
	}

	//. /// Note: For the future ACS (Access Control System)
	pub fn conv_id(&self) -> Option<i64> {
		self.conv_id
	}

	pub fn req_chain(&self) ->Option<&Vec<ReqChainLink>> {
		self.req_chain.as_ref()
	}
}
