use derive_more::From;
use lib_core::model;
use rpc_router::RpcHandlerError;
use serde::Serialize;
use serde_with::serde_as;
pub type Result<T> = core::result::Result<T, Error>;

#[serde_as]
#[derive(Debug, From, Serialize, RpcHandlerError)]
pub enum Error {
	// -- Modules
	#[from]
	Model(model::Error),

	#[from]	
	GenAI(
		#[serde_as(as = "serde_with::DisplayFromStr")]
		genai::Error
	),

	RpcError
}

// region:    --- Error Boilerplate
impl core::fmt::Display for Error {
	fn fmt(
		&self,
		fmt: &mut core::fmt::Formatter,
	) -> core::result::Result<(), core::fmt::Error> {
		write!(fmt, "{self:?}")
	}
}

impl std::error::Error for Error {}
// endregion: --- Error Boilerplate
