use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

#[derive(Debug)]
pub struct Error(pub &'static str);

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        (
            StatusCode::OK,
            format!(
                "d14:failure reason{}:{}8:intervali5400e12:min intervali5400ee",
                self.0.to_string().chars().count(),
                self.0,
            ),
        )
            .into_response()
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        self.0
    }
}
