use std::{collections::HashMap, sync::OnceLock};
use crate::error::{Error, Result};
use crate::utils::service_resolution::ServiceResolutionData;

use lib_utils::envs;
use regex::Regex;
use tracing::{info, warn};

#[cfg(feature="dev-utils")]
pub(crate) fn resolve_service(srv_name: &str) -> Result<&ServiceResolutionData> {
    service_registry()
        .table
        .get(srv_name)
        .ok_or(Error::ServiceResolutionFailed)
}

#[cfg(not(feature="dev-utils"))]
pub(crate) fn resolve_service(srv_name: &str) -> Result<&ServiceResolutionData> {
    Err(Error::ServiceResolutionFailed)
}

fn service_registry() -> &'static ServiceRegistry {
	static INSTANCE: OnceLock<ServiceRegistry> = OnceLock::new();

	INSTANCE.get_or_init(|| {
		ServiceRegistry::load_from_env().unwrap_or_else(|ex| {
			panic!("FATAL - WHILE LOADING Service Resolution Data - Cause: {ex:?}")
		})
	})
}

struct ServiceRegistry {
    pub table: HashMap<String, ServiceResolutionData>,
}

impl ServiceRegistry {
    fn load_from_env() -> Result<ServiceRegistry> {
        info!("{:<12} - Initialize service resolution from config.toml/env", "FOR-DEV-ONLY");

        let mut src_info_map = HashMap::<String, ServiceResolutionData>::new();

        let re = Regex::new(r"SERVICE_RESOLUTION_(.*)").unwrap();			
        if let Ok(hmap) = envs::get_matching(re) {
            for (k, v) in hmap {			
                match serde_json::from_str::<ServiceResolutionData>(&v) {
                    Ok(srd) => {
                        info!("Processing config key: {k}");
                        info!("{:<12} - Service {} → http://{}:{}", "FOR-DEV-ONLY", 
                            srd.name,
                            srd.host,
                            srd.port);					
                        
                        src_info_map.insert(srd.name.clone(), srd);
                    },
                    Err(e) => {
                        warn!("{:<12} - Wrong JSON format for {v:?}. Error = {e:?}", "FOR-DEV-ONLY");
                    },
                };			
            }
        }	
		Ok(ServiceRegistry { table: src_info_map})			
	}    
}
