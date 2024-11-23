use lib_rpc_core::prelude::*;
use std::collections::HashMap;
use crate::rpc::ParamsW;
use crate::error::Result;

// Main code from `examples/c05-model-names.rs` from https://github.com/jeremychone/rust-genai/
use genai::adapter::AdapterKind;
use genai::Client;
use lib_core::{ctx::Ctx, model::ModelManager};
use serde::{Deserialize, Serialize};
use tracing::debug;

//-- Handler message params --------------------------
// Can I somehow wrap the incoming adapter into this 
// How will it look when serialized though ? Maybe ok to 
// be explicit about this.
// - Removed Ollama
// - Added All
#[derive(Debug, Clone, Deserialize)]
pub enum GenAIProviderType {
    OpenAI,	
	Gemini,
	Anthropic,
	Groq,
	Cohere,
    All
}

#[derive(Debug, Clone, Deserialize)]
pub struct GetModelListRequest {
    pub provider: GenAIProviderType,    
}

#[derive(Debug, Clone, Serialize)]
pub struct GetModelListResponse {
    // { ProviderTypeName, Vec<ModelName> }
    pub models: HashMap<String, Vec<String>>,    
}

//-- Handler RPC  --------------------------------
pub async fn get_model_list (
    ctx: Ctx,
    _mm: ModelManager,    
    params: ParamsW<GetModelListRequest>,) 
-> Result<DataRpcResult<GetModelListResponse>> {
    
    debug!("{:<12} - model_list - {ctx:?}, {params:?}", "RPC");

    // process input args
    let ParamsW{data: gmlr} = params;
    let al = resolve_adapter_list(&gmlr);

    // prepare output
    let client = Client::default();
    let mut model_map = HashMap::<String, Vec<String>>::new();
	for kind in al {
		//debug!("\n--- Models for {kind}");
		
        // If this fails (Like in Ollama) send an empty list
        let models = client.all_model_names(kind)            
            .await
            .map_or(vec![], |v| v);

        //debug!("{models:?}");
        model_map.insert(kind.to_string(), models);		
	}    

    // Pack response
    let ret = GetModelListResponse{
        models : model_map,
    };

    Ok(ret.into())
}

fn resolve_adapter_list(req: &GetModelListRequest) -> Vec<AdapterKind>
{
    // Don't query for Ollama
    match &req.provider {
        GenAIProviderType::OpenAI => vec![AdapterKind::OpenAI],
	    GenAIProviderType::Gemini => vec![AdapterKind::Gemini],
	    GenAIProviderType::Anthropic => vec![AdapterKind::Anthropic],
	    GenAIProviderType::Groq => vec![AdapterKind::Groq],
	    GenAIProviderType::Cohere => vec![AdapterKind::Cohere],
        GenAIProviderType::All => vec![
            AdapterKind::OpenAI,		    
		    AdapterKind::Gemini,
		    AdapterKind::Anthropic,
		    AdapterKind::Groq,
		    AdapterKind::Cohere,
        ]
    }
}