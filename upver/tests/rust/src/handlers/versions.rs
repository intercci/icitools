use anyhow::Result;
use iciaws_router::{
    addons::AddonHolder,
    input::RouteHandlerInput,
    output::{RouteHandlerOutput, ok_json},
    types::RouteHandler,
};
use iciaws_router_macros::route;
use std::future::Future;
use std::pin::Pin;
use serde_json;

const VERSION: &'static str = "2.0.0";

#[route("GET/version")] // this macro generates GetVersionHandler struct with get_key() method
/// This function handles the http request GET /version
pub async fn get_version(_input: RouteHandlerInput, _addons: &AddonHolder) -> Result<RouteHandlerOutput> {
    let ds = serde_json::json!({"state": "Running", "version": VERSION});
    let js = serde_json::to_string(&ds)?;
    Ok(ok_json(js))
}