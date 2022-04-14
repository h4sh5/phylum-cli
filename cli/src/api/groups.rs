use super::common::API_PATH;

/// GET /groups/list
pub(crate) fn list(api_uri: &str) -> String {
    format!("{api_uri}/{API_PATH}/groups")
}

/// POST /groups/create
pub(crate) fn create(api_uri: &str) -> String {
    format!("{api_uri}/{API_PATH}/groups")
}
