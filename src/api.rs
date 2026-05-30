use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use utoipa::{OpenApi, ToSchema};

use crate::client::{self, DepError};

pub const SERVICE: &str = "srvcs-inrange";
pub const CONCERN: &str = "range: is value within [lo, hi]";
pub const DEPENDS_ON: &[&str] = &["srvcs-between"];

/// Dependency endpoints, injected as router state so tests can point them at
/// mock services.
#[derive(Clone)]
pub struct Deps {
    pub between_url: String,
}

#[derive(Serialize, ToSchema)]
pub struct Info {
    pub service: &'static str,
    pub concern: &'static str,
    pub depends_on: Vec<&'static str>,
}

/// `GET /` — service identity (srvcs service standard).
#[utoipa::path(get, path = "/", responses((status = 200, body = Info)))]
pub async fn index() -> Json<Info> {
    Json(Info {
        service: SERVICE,
        concern: CONCERN,
        depends_on: DEPENDS_ON.to_vec(),
    })
}

#[derive(Deserialize, ToSchema)]
pub struct EvalRequest {
    #[schema(value_type = Object)]
    pub value: Value,
    #[schema(value_type = Object)]
    pub lo: Value,
    #[schema(value_type = Object)]
    pub hi: Value,
}

#[derive(Serialize, ToSchema)]
pub struct ResultResponse {
    #[schema(value_type = Object)]
    pub value: Value,
    #[schema(value_type = Object)]
    pub lo: Value,
    #[schema(value_type = Object)]
    pub hi: Value,
    /// `true` iff `lo <= value <= hi`.
    pub result: bool,
}

fn degraded(dependency: &str) -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({ "error": "dependency unavailable", "dependency": dependency })),
    )
        .into_response()
}

/// Forward a dependency's response verbatim (used to propagate `422` for invalid
/// input, so inrange reports the same rejection a leaf dependency did).
fn forward(status: u16, body: Value) -> Response {
    let code = StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY);
    (code, Json(body)).into_response()
}

/// Ask one boolean dependency with `payload` for its `result`, mapping its
/// failures to the response this service should return.
///
/// - unreachable / non-`200`/`422` -> `503` degraded
/// - `422` -> forwarded `422` (the dependency rejected the input)
async fn ask(url: &str, payload: &Value, dependency: &str) -> Result<bool, Response> {
    match client::call(url, payload).await {
        Err(DepError::Unreachable) => Err(degraded(dependency)),
        Ok((200, body)) => Ok(body.get("result").and_then(Value::as_bool).unwrap_or(false)),
        // Invalid input propagates from the leaf dependency; forward it.
        Ok((422, body)) => Err(forward(422, body)),
        Ok(_) => Err(degraded(dependency)),
    }
}

/// `POST /` — decide whether `value` is within `[lo, hi]`.
///
/// This service is a thin orchestrator over `srvcs-between`: it does no
/// comparison of its own and delegates entirely, forwarding
/// `{"value", "lo", "hi"}` to `srvcs-between` and returning its boolean
/// `result`. Invalid operands are rejected by `srvcs-between` and the resulting
/// `422` is forwarded unchanged.
#[utoipa::path(
    post,
    path = "/",
    request_body = EvalRequest,
    responses(
        (status = 200, body = ResultResponse),
        (status = 422, description = "an operand is invalid (forwarded from srvcs-between)"),
        (status = 500, description = "a dependency returned a malformed result"),
        (status = 503, description = "a dependency is unavailable")
    )
)]
pub async fn evaluate(State(deps): State<Deps>, Json(req): Json<EvalRequest>) -> Response {
    let result = match ask(
        &deps.between_url,
        &json!({ "value": req.value, "lo": req.lo, "hi": req.hi }),
        "srvcs-between",
    )
    .await
    {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    (
        StatusCode::OK,
        Json(json!({ "value": req.value, "lo": req.lo, "hi": req.hi, "result": result })),
    )
        .into_response()
}

#[derive(OpenApi)]
#[openapi(
    paths(index, evaluate),
    components(schemas(Info, EvalRequest, ResultResponse))
)]
pub struct ApiDoc;

/// Serve OpenAPI document
pub async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openapi_documents_routes() {
        let doc = ApiDoc::openapi();
        let root = doc.paths.paths.get("/").expect("path / present");
        assert!(root.get.is_some());
        assert!(root.post.is_some());
    }

    #[tokio::test]
    async fn index_reports_dependency() {
        let Json(info) = index().await;
        assert_eq!(info.service, "srvcs-inrange");
        assert_eq!(info.concern, "range: is value within [lo, hi]");
        assert_eq!(info.depends_on, vec!["srvcs-between"]);
    }
}
