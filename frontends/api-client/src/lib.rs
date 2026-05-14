/// Shared API client for frontend applications (GUI, TUI, WebUI)
///
/// Provides typed HTTP client with SSE event stream handling.

pub struct ApiClient {
    base_url: String,
}

impl ApiClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    // Phase 3: typed request/response models, SSE reconnect, offline buffer
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_client_creation() {
        let client = ApiClient::new("http://127.0.0.1:7700");
        assert_eq!(client.base_url(), "http://127.0.0.1:7700");
    }
}
