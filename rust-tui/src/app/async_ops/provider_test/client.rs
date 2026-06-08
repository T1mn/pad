pub(super) fn provider_test_client() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent("pad-provider-test/0.1")
        .build()
}

pub(super) fn bearer_get<'a>(
    client: &'a reqwest::Client,
    url: &'a str,
    credential: Option<&str>,
) -> reqwest::RequestBuilder {
    let mut request = client.get(url);
    if let Some(token) = credential.filter(|token| !token.trim().is_empty()) {
        request = request.bearer_auth(token);
    }
    request
}
