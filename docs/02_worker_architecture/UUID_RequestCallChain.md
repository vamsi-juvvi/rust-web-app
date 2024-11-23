
# UUID Request chain

In the stand-alone server case of `web-server`, Jeremy added a per-request `UUID` into the context so that the trace lines could be associated back with the originating request. I have extended this format so that when the gateway forwards a request to a worker, the original gateway request UUID is also sent and logged along with the worker's request UUID. In general this is a chain _(which can be used in either a EDA or Worker architecture to associate downstream requests with any upstream source request)_.

## Add a UUID Chain

New struct to form elements of a request chain

```rust
#[cfg_attr(feature = "with-rpc", derive(rpc_router::RpcResource))]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReqChainLink {
	// calling service
	pub service: String, 

	// req-id of service's request.
	pub uuid : Uuid,
}
```

Not sure if the Ctx is the best place to keep this (ReqStamp could be another)

```diff
pub struct Ctx {
	user_id: i64,	

	/// Note: For the future ACS (Access Control System)
	conv_id: Option<i64>,

+	/// Call chain for when this Ctx is sent from gateway down 
+	/// to a chain of workers
+	req_chain: Option<Vec<ReqChainLink>>,
}

impl Ctx {
    ...

+   pub fn add_req_chain_link(&self, req_link: ReqChainLink) -> Ctx {
+		let mut ctx = self.clone();		
+		ctx.req_chain = match ctx.req_chain {
+			Some(mut v) => { v.push(req_link); Some(v) },
+			None => Some(vec![req_link]),			
+		};		
+
+		ctx			
+	}

+	pub fn req_chain(&self) ->Option<&Vec<ReqChainLink>> {
+		self.req_chain.as_ref()
+	}
}
```

## lib-utils add new method to get proc name

 Added a new `lib_utils::proc::prog_name()` with some associated errors. This could be some other identifier instead of the proc-name, but for now, proc-name will do the job of identifying the upstream.

 ```rust
 pub fn prog_name() -> Result<String> {
    Ok(env::current_exe()?
        .file_name().ok_or(Error::ProcNoFile)?
        .to_str().ok_or(Error::ProcPathNotUtf8)?
        .to_owned())
}
```

## handler-rpc sends chain enhanced Ctx down

When calling a worker, a new Ctx object is created which adds the current ReqStamp's UUID to the ReqChain

```rust
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
```

```diff
async fn do_rpc_handler_dispatch_server(	
	ctx: Ctx,
+	req_stamp: ReqStamp,
	rpc_req: rpc_router::Request,
	service : &str,
	method : &str,
	rpc_info: &RpcInfo
) 
 -> Result<Json<Value>, rpc_router::CallError> 
 {	
	// build up the request chain by adding this current ReqStamp's UUID to 
	// the ctx which'll be sent downstream.
+	let ctx = _add_curr_req_to_chain(ctx, req_stamp);
    ...
}
```

## request logging now also logs the call chain

```diff
struct RequestLogLine {	
    ...
	// -- rpc info.
	rpc_id: Option<String>,
	rpc_method: Option<String>,
+	rpc_chain: Option<Vec<String>>,
    ...
}
```

which is initialized this way: `ctx = Option<Ctx>`

```rust
rpc_chain: ctx.and_then(
    |c| c.req_chain().and_then(
        |v| {
            let xform = v.iter()
            .map(|lnk| format!("{}@{}", lnk.uuid, lnk.service))
            .collect();
        Some(xform)
        }
))
```

## Finally

```console
DEBUG EXTRACTOR    - CtxW
DEBUG EXTRACTOR    - ReqStamp
DEBUG RPC          - one_shot_msg - Ctx { user_id: 1000, conv_id: None, req_chain: Some([ReqChainLink { service: "web-gateway", uuid: 3071c4e3-9852-4808-b7f7-617a7bb370d8 }]) }, ParamsW { data: OneShotMsg { mode: System, prompt: "why is the sky blue" } }DEBUG RES_MAPPER   - mw_reponse_map
DEBUG REQUEST LOG LINE:
{"duration_ms":360.474,"http_method":"POST","http_path":"/api/rpc","rpc_chain":["3071c4e3-9852-4808-b7f7-617a7bb370d8@web-gateway"],"rpc_id":"1","rpc_method":"one_shot_msg","time_in":"2024-07-24T01:37:45.365676665Z","timestamp":"2024-07-24T01:37:45.726151561Z","user_id":1000,"uuid":"e5d9858e-0a9b-495d-b7c1-bb09fcd289b6"}
```

Note the `"rpc_chain":["3071c4e3-9852-4808-b7f7-617a7bb370d8@web-gateway"]` in the log from `llm-worker` Which ties it to the log from `web-gteway` via `"uuid":"3071c4e3-9852-4808-b7f7-617a7bb370d8"`

```console
 INFO FOR-DEV-ONLY - Initialize service resolution from config.toml/env
 INFO Processing config key: LLM
 INFO FOR-DEV-ONLY - Service llm-worker → http://localhost:8081
 INFO Processing config key: VISION
 INFO FOR-DEV-ONLY - Service vision-worker → http://localhost:8082
DEBUG RPC Dispatch - Resolved "llm-worker" to "http://localhost:8081/api/rpc"
DEBUG RPC Dispatch - WebResponse status 200
DEBUG RES_MAPPER   - mw_reponse_map
DEBUG REQUEST LOG LINE:
{"duration_ms":370.237,"http_method":"POST","http_path":"/api/rpc","rpc_id":"1","rpc_method":"llm-worker/one_shot_msg","time_in":"2024-07-24T01:37:45.357669397Z","timestamp":"2024-07-24T01:37:45.72790661Z","user_id":1000,"uuid":"3071c4e3-9852-4808-b7f7-617a7bb370d8"}
```