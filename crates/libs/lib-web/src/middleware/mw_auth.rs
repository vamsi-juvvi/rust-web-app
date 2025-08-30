use crate::error::{Error, Result};
use crate::utils::token::{set_token_cookie, AUTH_TOKEN};
use axum::body::Body;
use axum::extract::{FromRequestParts, State};
use axum::http::request::Parts;
use axum::http::{header, Request, HeaderMap};
use axum::middleware::Next;
use axum::response::Response;
use lib_auth::token::{validate_web_token, Token};
use lib_core::ctx::Ctx;
use lib_core::model::user::{UserBmc, UserForAuth};
use lib_core::model::ModelManager;
use serde::Serialize;
use tower_cookies::{Cookie, Cookies};
use tracing::debug;

// region:    ---  Ctx Requre and Resolve
pub async fn mw_ctx_require(
	ctx: Result<CtxW>,
	req: Request<Body>,
	next: Next,
) -> Result<Response> {
	debug!("{:<12} - mw_ctx_require - {ctx:?}", "MIDDLEWARE");

	ctx?;

	Ok(next.run(req).await)
}

// IMPORTANT: This resolver must never fail, but rather capture the potential Auth error and put in in the
//            request extension as CtxExtResult.
//            This way it won't prevent downstream middleware to be executed, and will still capture the error
//            for the appropriate middleware (.e.g., mw_ctx_require which forces successful auth) or handler
//            to get the appropriate information.
pub async fn mw_ctx_root_resolver(
	State(mm): State<ModelManager>,
	cookies: Cookies,
	mut req: Request<Body>,
	next: Next,
) -> Response {
	debug!("{:<12} - mw_ctx_root_resolver", "MIDDLEWARE");

	let ctx_ext_result = ctx_root_resolve(mm, &cookies).await;

	if ctx_ext_result.is_err()
		&& !matches!(ctx_ext_result, Err(CtxExtError::TokenNotInCookie))
	{
		cookies.remove(Cookie::from(AUTH_TOKEN))
	}

	// Store the ctx_ext_result in the request extension
	// (for Ctx extractor).
	req.extensions_mut().insert(ctx_ext_result);

	next.run(req).await
}

pub async fn mw_ctx_leaf_resolver(
	mut req: Request<Body>,
	next: Next,
) -> Response {
	debug!("{:<12} - mw_ctx_leaf_resolver", "MIDDLEWARE");

	let ctx = ctx_from_req_header(&req.headers())		
		.map_err(|ex| CtxExtError::CtxCreateFail(ex.to_string()));	

	// store the ctx in request extension
	// Note that this can be an Error. Will 
	// be validated later
	req.extensions_mut().insert(ctx);

	next.run(req).await
}
// endregion:    ---  Ctx Requre and Resolve

async fn ctx_root_resolve(mm: ModelManager, cookies: &Cookies) -> CtxExtResult {
	// -- Get Token String
	let token = cookies
		.get(AUTH_TOKEN)
		.map(|c| c.value().to_string())
		.ok_or(CtxExtError::TokenNotInCookie)?;

	// -- Parse Token
	let token: Token = token.parse().map_err(|_| CtxExtError::TokenWrongFormat)?;

	// -- Get UserForAuth
	let user: UserForAuth =
		UserBmc::first_by_username(&Ctx::root_ctx(), &mm, &token.ident)
			.await
			.map_err(|ex| CtxExtError::ModelAccessError(ex.to_string()))?
			.ok_or(CtxExtError::UserNotFound)?;

	// -- Validate Token
	validate_web_token(&token, user.token_salt)
		.map_err(|_| CtxExtError::FailValidate)?;

	// -- Update Token
	set_token_cookie(cookies, &user.username, user.token_salt)
		.map_err(|_| CtxExtError::CannotSetTokenCookie)?;

	// -- Create CtxExtResult
	Ctx::new(user.id)
		.map(CtxW)
		.map_err(|ex| CtxExtError::CtxCreateFail(ex.to_string()))
}

// region:    --- Ctx Extractor
#[derive(Debug, Clone)]
pub struct CtxW(pub Ctx);

impl<S: Send + Sync> FromRequestParts<S> for CtxW {
	type Rejection = Error;

	async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self> {
		debug!("{:<12} - CtxW", "EXTRACTOR");

		parts
			.extensions
			.get::<CtxExtResult>()
			.ok_or(Error::CtxExt(CtxExtError::CtxNotInRequestExt))?
			.clone()
			.map_err(Error::CtxExt)
	}
}
// endregion: --- Ctx Extractor

// region:    --- Ctx to/from Auth1 header
pub(crate) const AUTH1_HEADER_KEY: &str = "X-WORKER-POSTAUTH";

pub fn set_auth1_header(ctx: &Ctx, headers: &mut HeaderMap) -> Result<()> {
	let mut header_val = header::HeaderValue
		::from_str(&serde_json::to_string(&ctx)?)
		.unwrap();

	header_val.set_sensitive(true);
	headers.insert(AUTH1_HEADER_KEY, header_val);

	Ok(())
}

pub fn get_ctx_headers(ctx: &Ctx) -> Result<(String,String)> {
	Ok(
		( AUTH1_HEADER_KEY.to_string(), serde_json::to_string(&ctx)?)	
	)
}

pub fn ctx_from_req_header(headers: &HeaderMap) -> Result<CtxW> {
	let pa_tok_val = headers
		.get(AUTH1_HEADER_KEY)
		.ok_or(Error::CtxExt(CtxExtError::CtxPostAuthTokenNotInReqHeader))?
		.to_str()
		.map_err(|_| Error::CtxExt(CtxExtError::CtxPostAuthTokenBadFormat))?;

	let ctx:Ctx = serde_json::from_str(pa_tok_val)?;
	Ok(CtxW(ctx))
}
// endregion: --- Ctx to/from Auth1 header


// region:    --- Ctx Extractor Result/Error
type CtxExtResult = core::result::Result<CtxW, CtxExtError>;

#[derive(Clone, Serialize, Debug)]
pub enum CtxExtError {
	TokenNotInCookie,
	TokenWrongFormat,

	UserNotFound,
	ModelAccessError(String),
	FailValidate,
	CannotSetTokenCookie,

	CtxNotInRequestExt,
	CtxPostAuthTokenNotInReqHeader,
	CtxPostAuthTokenBadFormat,
	CtxCreateFail(String),
}
// endregion: --- Ctx Extractor Result/Error
