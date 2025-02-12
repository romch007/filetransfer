use axum::http::StatusCode;

pub trait InternalErrExt<T> {
    fn map_internal_err(self) -> Result<T, StatusCode>;
}

impl<T, E> InternalErrExt<T> for Result<T, E>
where
    E: std::fmt::Display,
{
    fn map_internal_err(self) -> Result<T, StatusCode> {
        self.inspect_err(|e| tracing::error!("internal err: {e}"))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    }
}

pub trait NotFoundExt<T> {
    fn map_not_found(self) -> Result<T, StatusCode>;
}

impl<T> NotFoundExt<T> for Option<T> {
    fn map_not_found(self) -> Result<T, StatusCode> {
        self.ok_or(StatusCode::NOT_FOUND)
    }
}
