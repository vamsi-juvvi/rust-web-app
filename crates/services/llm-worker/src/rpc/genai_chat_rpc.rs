use lib_rpc_core::prelude::*;
use crate::error::{Result, Error};
use crate::rpc::ParamsW;

use serde::{Deserialize, Serialize};

use genai::chat::{ChatMessage, ChatRequest};
use genai::Client;

use tracing::debug;

//-- Handler message params --------------------------
#[derive(Debug, Clone, Deserialize)]
pub enum ChatUserMode {
    System,
    User,
    Assistant
}

#[derive(Debug, Clone, Deserialize)]
pub struct OneShotMsg {
    pub mode: ChatUserMode,
    pub prompt: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct OneShotMsgResponse {
    pub response: String,    
}

// Models -------------------------------------------
// I have converted the genai model query into an RPC call
// at genai_model_rpc::get_model_list and it gave me the following
//
// "Cohere": [
//   "command-r-plus",
//   "command-r",
//   "command",
//   "command-nightly",
//   "command-light",
//   "command-light-nightly"
// ],
// "Gemini": [
//   "gemini-1.5-pro",
//   "gemini-1.5-flash",
//   "gemini-1.0-pro",
//   "gemini-1.5-flash-latest"
// ],
// "Groq": [
//   "llama3-8b-8192",     -- Free. (per 1M Tokens, input/output)$0.05/$0.08
//   "llama3-70b-8192",    -- Free. (per 1M Tokens, input/output)$0.59/$0.79
//   "mixtral-8x7b-32768", -- Free. (per 1M Tokens, input/output)$0.24/$0.24
//   "gemma-7b-it",        -- Free. (per 1M Tokens, input/output)$0.07/$0.07
//   "gemma2-9b-it"
// ],
// "OpenAI": [
//   "gpt-4o",
//   "gpt-4o-mini",        -- 0.15 / 1M input tokens, 0.6 / 1M Output tokens
//   "gpt-4-turbo",
//   "gpt-4",
//   "gpt-3.5-turbo"       -- 0.5 / 1M input tokens, 1.5 / 1M Output
// ]
//
//  Since groq is currently free with rate limits. Use it
// otw switch to "gpt-4o-mini"
//-- Handler RPC  -----------------------------------
pub async fn one_shot_msg(
    ctx: Ctx,
    _mm: ModelManager,
    params: ParamsW<OneShotMsg>,) 
-> Result<DataRpcResult<OneShotMsgResponse>> {
    
    debug!("{:<12} - one_shot_msg - {ctx:?}, {params:?}", "RPC");

    let ParamsW{data: osm} = params;

    // Ugly for now but can cache behind a conv_id later
    let client = Client::default();
	let mut chat_req = ChatRequest::default().with_system("Answer with one sentence");        
    chat_req = chat_req.append_message(ChatMessage::user("Why is the sky blue"));

    // Default chose the free Groq one. Later can allow choosing of provider/model.
    //let model = "gpt-4o-mini";
    let model = "llama3-70b-8192";

     // Add the incoming msg
     chat_req = chat_req.append_message(
        match osm.mode {
            ChatUserMode::System => ChatMessage::system(osm.prompt),
            ChatUserMode::User => ChatMessage::user(osm.prompt),
            ChatUserMode::Assistant => ChatMessage::assistant(osm.prompt),
        }
    );
    
    // see https://github.com/jeremychone/rust-genai/blob/HEAD/examples/c00-readme.rs
    // for examples
    let chat_response = client.exec_chat(model, chat_req.clone(), None)
        .await
        .map_err(|_| Error::RpcError)?;    

    let response_text = chat_response.content_text_as_str()
        .unwrap_or("No Naswer from LLM");    

    // Pack response
    let ret = OneShotMsgResponse{
        response: response_text.to_string(),
    };

    Ok(ret.into())
}
