#![forbid(unsafe_code)]

use rhelma_core::{RequestContext, RhelmaError};

use crate::{
    domain::FileId,
    error::{ApiError, ApiResult},
};

/// Extract the tenant_id from RequestContext and parse a FileId from a route param.
///
/// This keeps route handlers small and ensures consistent error codes/messages.
#[allow(clippy::result_large_err)]
pub fn tenant_and_file_id(ctx: &RequestContext, file_id: &str) -> ApiResult<(String, FileId)> {
    let tenant_id = ctx
        .tenant_id()
        .map(|t| t.as_str().to_string())
        .ok_or_else(|| {
            ApiError::with_ctx(RhelmaError::BadRequest("missing tenant_id".into()), ctx)
        })?;

    let id = file_id
        .parse::<FileId>()
        .map_err(|_| ApiError::with_ctx(RhelmaError::BadRequest("invalid file id".into()), ctx))?;

    Ok((tenant_id, id))
}

/// Sanitize an untrusted filename for safe inclusion in a Content-Disposition header.
pub fn sanitize_filename(name: &str) -> String {
    name.replace(['"', '\r', '\n'], "_")
}

#[cfg(test)]
mod tests {
    use super::sanitize_filename;

    #[test]
    fn sanitize_filename_strips_header_breaks() {
        let input = "a\"b\r\nc";
        let out = sanitize_filename(input);
        assert!(!out.contains('"'));
        assert!(!out.contains('\r'));
        assert!(!out.contains('\n'));
    }
}
