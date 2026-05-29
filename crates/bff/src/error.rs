use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use domain::DomainError;
use serde_json::json;

/// HTTP wrapper over `DomainError`. Maps domain failures to status codes and a
/// JSON `{ "error": ... }` body, so handlers can just `?` on use-case calls.
pub struct ApiError(DomainError);

impl From<DomainError> for ApiError {
    fn from(e: DomainError) -> Self {
        ApiError(e)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match self.0 {
            DomainError::DeviceNotFound(_) => StatusCode::NOT_FOUND,
            DomainError::NoDeviceSelected => StatusCode::CONFLICT,
            DomainError::Cast(_) | DomainError::Discovery(_) | DomainError::Audio(_) => {
                StatusCode::BAD_GATEWAY
            }
        };
        (status, Json(json!({ "error": self.0.to_string() }))).into_response()
    }
}
