use axum::{
    body::Bytes,
    extract::State,
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};

use crate::{certs::service, error::AppError, state::AppState};

pub async fn ca_chain(State(state): State<AppState>) -> Result<Response, AppError> {
    let pem = service::ca_chain(&state.pool).await?;
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/x-pem-file"),
    );
    Ok((StatusCode::OK, headers, pem).into_response())
}

pub async fn crl(State(state): State<AppState>) -> Result<Response, AppError> {
    let der = service::generate_crl(&state.pool, &state.config).await?;
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/pkix-crl"),
    );
    Ok((StatusCode::OK, headers, der).into_response())
}

pub async fn ocsp(State(state): State<AppState>, body: Bytes) -> Result<Response, AppError> {
    let der = match service::ocsp_response(&state.pool, &state.config, &body).await {
        Ok(der) => der,
        Err(AppError::BadRequest(_)) => {
            service::unsuccessful_ocsp(ocsp::response::OcspRespStatus::MalformedReq)?
        }
        Err(err) => {
            tracing::error!("OCSP response generation failed: {err}");
            service::unsuccessful_ocsp(ocsp::response::OcspRespStatus::InternalError)?
        }
    };
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/ocsp-response"),
    );
    Ok((StatusCode::OK, headers, der).into_response())
}
