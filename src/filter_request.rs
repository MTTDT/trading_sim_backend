use serde::{Serialize, Deserialize};
use utoipa::IntoParams;

#[derive(Debug, Clone, Serialize, Deserialize, Default, IntoParams)]
#[serde(default)]

pub struct FilterRequest {
    pub page: Option<i64>,
    pub page_length: Option<i64>,
    pub search: Option<String>,
}

const DEFAULT_PAGE_LENGTH: i64 = 20;
const MAX_PAGE_LENGTH: i64 = 100;

impl FilterRequest {
    pub fn resolve(&self) -> (Option<String>, i64, i64) {
        let limit = self
            .page_length
            .unwrap_or(DEFAULT_PAGE_LENGTH)
            .clamp(1, MAX_PAGE_LENGTH);
        let offset = self.page.unwrap_or(0).max(0) * limit;
        let pattern = self
            .search
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| format!("%{s}%"));
        (pattern, limit, offset)
    }
}