use serde::Deserialize;
use crate::error::Result;

#[cfg(feature="dev-utils")]
use crate::_dev_utils;

#[derive(Deserialize, Debug)]
pub(crate) struct ServiceResolutionData {
	pub name: String,
	pub host : String,
	pub port : i32,
}

pub(crate) fn resolve_service(srv_name: &str) -> Result<&ServiceResolutionData> {
    do_resolve_service(srv_name)
}

#[cfg(feature="dev-utils")]
fn do_resolve_service(srv_name: &str) -> Result<&ServiceResolutionData> {
    _dev_utils::resolve_service(srv_name)
}

#[cfg(not(feature="dev-utils"))]
fn do_resolve_service(srv_name: &str) -> Result<&ServiceResolutionData> {
    todo!()    
}