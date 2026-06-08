use super::validate_browser_url;

#[test]
fn validates_safe_browser_urls() {
    assert!(validate_browser_url("http://localhost:3000"));
    assert!(validate_browser_url("https://example.com"));
    assert!(!validate_browser_url("javascript:alert(1)"));
}
