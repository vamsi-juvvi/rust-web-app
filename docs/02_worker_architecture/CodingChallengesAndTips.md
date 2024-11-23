<!-- TOC -->

- [Worker and RPC coding challenges and tips](#worker-and-rpc-coding-challenges-and-tips)
    - [Tip: Struggles while implementing RPC handlers](#tip-struggles-while-implementing-rpc-handlers)
    - [Tip: axum::debug_handler](#tip-axumdebug_handler)

<!-- /TOC -->
# Worker and RPC coding challenges and tips

## Tip: Struggles while implementing RPC handlers

Started out with a generic 

```rust
pub fn rpc_router_builder() -> RouterBuilder {
    router_builder!(
        one_shot_msg,
    )
}

pub async fn one_shot_msg(
    ctx: Ctx,
    mm: ModelManager,    
    prompt: &String,) 
-> Result<DataRpcResult<String>> {
    todo!()
}
```

Which promptly failed with an IDE error saying that _no method named into_dyn() for one_shot_msg_. Playing around, I replaced `prompt: &String` with `params: ParamsIded,` and the error went away. Surprisingly `rust into_dyn` gave me hardly any search hits! Related to `dyn traits` likely and maybe all arguments need to derive from a Trait for them to be Dynd! Ctx certainly does not but then it has a bunch of `#[derive()]` on it so maybe it does indirectly.

Digging around the RPC code, it looks like

 - The contract is that the the macro `router_builder!` calls `RouterBuilder::append_dyn` 
 - `RouterBuilder::append_dyn` wants a `Box<dyn RpcHandlerWrapperTrait>` which means the function itself should be castable to `dyn RpcHandlerWrapperTrait`

Looks like I can get all this to work by handling some simple rules

 - Params to be wrapped in a `ParamsForCreate<RPCCallPayloadStruct>` 
   - which implements the `IntoParam` trait 
   - that simply allows an `Option<Value>`, i.e., json to be deserialized into `RPCCallPayloadStruct`
 - Instead of `ParamsForCreate` which is used by `build_common_rpc_func!` to simply destructure it's payload, Seems easy enough to create a `PassthroughParams` which implements IntoParams and the name is meaningful.

```rust
//-- Handler message --------------------------------
pub async fn one_shot_msg(
    ctx: Ctx,
    mm: ModelManager,    
    params: ParamsW<OneShotMsg>,) 
-> Result<DataRpcResult<String>> {
    todo!()
}

//-- Infra extension for wrapping params ------------
/// Params structure for any RPC pass-through call.
#[derive(Deserialize)]
pub struct ParamsW<D> {
	pub data: D,
}

impl<D> IntoParams for ParamsW<D> where D: DeserializeOwned + Send {}

//-- Handler message params --------------------------
#[derive(Debug, Clone, Serialize)]
pub enum ChatUserMode {
    System,
    User,
    Assistant
}

#[derive(Debug, Clone, Serialize)]
pub struct OneShotMsg {
    pub mode: ChatUserMode,
    pub prompt: String,
}
```

the above still fails saying `one_shot_msg` does not impement `into_dyn`. I am taking that to believe it does not satisfy the `RpcHandlerWrapper` signature or bounds. Wonder if the `DeserializeOwned` bounds conflicts with `String`?

Experimenting shows that if I use `ParamsForCreate<ConvMsgforCreate>`, it all passes. So one issue for sure is the difference between `OneShotMsg` and `ConvMsgForCreate`.

 - Added `Fields` and `FromRow` to it (_had to push bunch more dependencies up to workspace level_)
 - Still error with unable to handle `Fields` on enum.
 - After going through some other gnarly modql and sqlx related stuff, turns out I needed a `Deserialize` on the struct. Ofc!! The `Option<Value>` needs to be deserialized into the struct!
 - the return value also needs to be `Serialied`
 - with those two changes, it all works.


## Tip: axum::debug_handler

When you are creation new axum handlers of middleware, sometimes you get incomrehensible error vomit. For instance, I got the following

```log
error[E0277]: the trait bound `axum::middleware::FromFn<fn(axum::http::Request<Body>, Json<serde_json::value::Value>, Next) -> impl Future<Output = std::result::Result<Response<Body>, lib_web::error::Error>> {mw_rpc_resolver}, (), Route, _>: tower_service::Service<axum::http::Request<Body>>` is not satisfied

──────────────impl Future<Output = std::result::Result<Response<Body>, lib_web::error::Error>> {mw_rpc_resolver}, (), Route, _>: tower_service::Service<axum::http::Request<Body>>` is not satisfied
   --> crates/services/llm-worker/src/main.rs:40:16
    |
40  |         .route_layer(middleware::from_fn(mw_rpc_resolver));
    |          ----------- ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ the trait `tower_service::Service<axum::http::Request<Body>>` is not implemented for `FromFn<fn(Request<Body>, Json<Value>, Next) -> ... {mw_rpc_resolver}, ..., ..., ...>`
    |          |
    |          required by a bound introduced by this call
    |
    = help: the following other types implement trait `tower_service::Service<Request>`:
              axum::middleware::FromFn<F, S, I, (T1, T2)>
              axum::middleware::FromFn<F, S, I, (T1, T2, T3)>
              axum::middleware::FromFn<F, S, I, (T1, T2, T3, T4)>
              axum::middleware::FromFn<F, S, I, (T1, T2, T3, T4, T5)>
              axum::middleware::FromFn<F, S, I, (T1, T2, T3, T4, T5, T6)>
              axum::middleware::FromFn<F, S, I, (T1, T2, T3, T4, T5, T6, T7)>
              axum::middleware::FromFn<F, S, I, (T1, T2, T3, T4, T5, T6, T7, T8)>
              axum::middleware::FromFn<F, S, I, (T1, T2, T3, T4, T5, T6, T7, T8, T9)>
            and 8 others
note: required by a bound in `Router::<S>::route_layer`
   --> /home/vamsi/.cargo/registry/src/index.crates.io-6f17d22bba15001f/axum-0.7.5/src/routing/mod.rs:296:21
    |
293 |     pub fn route_layer<L>(self, layer: L) -> Self
    |            ----------- required by a bound in this associated function
...
296 |         L::Service: Service<Request> + Clone + Send + 'static,
    |                     ^^^^^^^^^^^^^^^^ required by this bound in `Router::<S>::route_layer`
```

One hint at https://stackoverflow.com/questions/76086106/axum-pass-value-from-middleware-to-route was to decorate the handler with a `#[axum::debug_handler]`

Which is **awesome**. Now I get a 

```rust
// rpc_req will be of type rpc_router::Request
#[axum::debug_handler]
pub async fn mw_rpc_resolver(
	mut req: Request<Body>,	
	Json(rpc_req): Json<Value>,
	next: Next	
) -> Result<Response> {
	debug!("{:<12} - mw_rpc_resolver", "MIDDLEWARE");	
    ...
}
```

```log
error: Can't have two extractors that consume the request body. `Request<_>` and `Json<_>` both do that.
  --> crates/libs/lib-web/src/middleware/mw_rpc.rs:32:2
   |
32 |     mut req: Request<Body>,
   |     ^^^

warning: unused variable: `rpc_req`
  --> crates/libs/lib-web/src/middleware/mw_rpc.rs:33:7
   |
33 |     Json(rpc_req): Json<Value>,
   |          ^^^^^^^ help: if this is intentional, prefix it with an underscore: `_rpc_req`
   |
   = note: `#[warn(unused_variables)]` on by default

warning: variable does not need to be mutable
  --> crates/libs/lib-web/src/middleware/mw_rpc.rs:32:2
   |
32 |     mut req: Request<Body>,
   |     ----^^^
   |     |
   |     help: remove this `mut`
   |
   = note: `#[warn(unused_mut)]` on by default

warning: `lib-web` (lib) generated 2 warnings
```
