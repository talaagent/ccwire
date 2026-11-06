//! The client against a local mock server, with the fixtures.

#![allow(
    clippy::disallowed_methods,
    reason = "#[tokio::test] expands to the runtime's block_on"
)]
#![allow(clippy::unwrap_used, reason = "test bodies")]

use core::time::Duration;
use std::time::SystemTime;

use ccwire_oauth::{Client, Error, Plan, RefreshRequest, Secret};
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TOKEN: &str = "fake-access-token-1";

fn fixture(name: &str) -> &'static str {
    match name {
        "usage" => include_str!("fixtures/usage.json"),
        "usage_idle" => include_str!("fixtures/usage_idle.json"),
        "profile" => include_str!("fixtures/profile.json"),
        "profile_email_address" => include_str!("fixtures/profile_email_address.json"),
        "roles" => include_str!("fixtures/roles.json"),
        "prepaid_credits" => include_str!("fixtures/prepaid_credits.json"),
        "overage_spend_limit" => include_str!("fixtures/overage_spend_limit.json"),
        "token" => include_str!("fixtures/token.json"),
        "token_no_rotation" => include_str!("fixtures/token_no_rotation.json"),
        "token_invalid_grant" => include_str!("fixtures/token_invalid_grant.json"),
        "api_error_auth" => include_str!("fixtures/api_error_auth.json"),
        "api_error_rate_limit" => include_str!("fixtures/api_error_rate_limit.json"),
        other => panic!("no fixture {other}"),
    }
}

fn json(status: u16, name: &str) -> ResponseTemplate {
    ResponseTemplate::new(status).set_body_raw(fixture(name), "application/json")
}

fn client(server: &MockServer) -> Client {
    Client::builder()
        .base_url(server.uri())
        .token_url(format!("{}/v1/oauth/token", server.uri()))
        .build()
        .unwrap()
}

/// A GET with the OAuth headers, answered with `name`.
async fn mount_get(server: &MockServer, p: &str, name: &str) {
    Mock::given(method("GET"))
        .and(path(p))
        .and(header("authorization", format!("Bearer {TOKEN}").as_str()))
        .and(header("anthropic-beta", "oauth-2025-04-20"))
        .and(header(
            "user-agent",
            concat!("ccwire-oauth/", env!("CARGO_PKG_VERSION")),
        ))
        .respond_with(json(200, name))
        .expect(1)
        .mount(server)
        .await;
}

#[tokio::test]
async fn usage_full() {
    let server = MockServer::start().await;
    mount_get(&server, "/api/oauth/usage", "usage").await;
    let u = client(&server).usage(&Secret::new(TOKEN)).await.unwrap();
    let five = u.five_hour.unwrap();
    assert_eq!(five.utilization, Some(35.0));
    assert_eq!(
        five.resets_at.as_deref(),
        Some("2030-01-02T03:40:00.123456+00:00")
    );
    assert_eq!(u.seven_day.unwrap().utilization, Some(100.0));
    assert_eq!(u.seven_day_opus.unwrap().resets_at, None);
    assert_eq!(u.seven_day_sonnet.unwrap().utilization, Some(12.5));
    assert!(u.seven_day_oauth_apps.is_none());
    let extra = u.extra_usage.unwrap();
    assert_eq!(extra.is_enabled, Some(true));
    assert_eq!(extra.monthly_limit, Some(5000.0));
    assert_eq!(extra.used_credits, Some(1234.0));
    assert_eq!(extra.currency.as_deref(), Some("USD"));
    assert_eq!(u.limits.len(), 2);
    assert_eq!(u.limits[0].is_active, Some(true));
    let model = u.limits[1].scope.as_ref().unwrap().model.as_ref().unwrap();
    assert_eq!(model.display_name.as_deref(), Some("Example"));
    // An untyped window is kept.
    assert!(u.extra.contains_key("seven_day_cowork"));
}

#[tokio::test]
async fn usage_idle() {
    let server = MockServer::start().await;
    mount_get(&server, "/api/oauth/usage", "usage_idle").await;
    let u = client(&server).usage(&Secret::new(TOKEN)).await.unwrap();
    assert_eq!(u.five_hour.unwrap().resets_at, None);
    assert!(u.seven_day.is_none());
    assert!(u.extra_usage.is_none());
    assert_eq!(u.limits, []);
}

#[tokio::test]
async fn profile() {
    let server = MockServer::start().await;
    mount_get(&server, "/api/oauth/profile", "profile").await;
    let p = client(&server).profile(&Secret::new(TOKEN)).await.unwrap();
    assert_eq!(p.account.uuid, "00000000-0000-4000-8000-000000000001");
    assert_eq!(p.account.email.as_deref(), Some("ada@example.com"));
    assert_eq!(p.plan(), Some(Plan::Max));
    let org = p.organization.unwrap();
    assert_eq!(org.name.as_deref(), Some("Example Org"));
    assert_eq!(
        org.rate_limit_tier.as_deref(),
        Some("default_claude_max_20x")
    );
    assert_eq!(org.has_extra_usage_enabled, Some(true));
    assert!(org.extra.contains_key("subscription_created_at"));
    assert!(p.extra.contains_key("application"));
}

#[tokio::test]
async fn profile_email_address_and_no_org() {
    let server = MockServer::start().await;
    mount_get(&server, "/api/oauth/profile", "profile_email_address").await;
    let p = client(&server).profile(&Secret::new(TOKEN)).await.unwrap();
    assert_eq!(p.account.email.as_deref(), Some("bo@example.org"));
    assert_eq!(p.plan(), Some(Plan::Pro));
    assert!(p.organization.is_none());
}

#[tokio::test]
async fn roles() {
    let server = MockServer::start().await;
    mount_get(&server, "/api/oauth/claude_cli/roles", "roles").await;
    let r = client(&server).roles(&Secret::new(TOKEN)).await.unwrap();
    assert_eq!(r.organization_role.as_deref(), Some("admin"));
    assert_eq!(r.workspace_role, None);
    assert_eq!(r.organization_name.as_deref(), Some("Example Org"));
}

#[tokio::test]
async fn prepaid_credits_and_overage() {
    let server = MockServer::start().await;
    let org = "00000000-0000-4000-8000-0000000000a1";
    mount_get(
        &server,
        &format!("/api/oauth/organizations/{org}/prepaid/credits"),
        "prepaid_credits",
    )
    .await;
    mount_get(
        &server,
        &format!("/api/oauth/organizations/{org}/overage_spend_limit"),
        "overage_spend_limit",
    )
    .await;
    let c = client(&server);
    let token = Secret::new(TOKEN);
    let credits = c.prepaid_credits(&token, org).await.unwrap();
    assert_eq!(credits.amount, Some(2500.0));
    assert_eq!(credits.auto_reload_settings.unwrap().enabled, Some(false));
    assert_eq!(credits.expiry_policy_months, Some(12));
    let limit = c.overage_spend_limit(&token, org).await.unwrap();
    assert_eq!(limit.is_enabled, Some(true));
    assert_eq!(limit.monthly_credit_limit, Some(5000.0));
    assert_eq!(limit.used_credits, Some(1234.0));
    assert_eq!(limit.disabled_until, None);
}

#[tokio::test]
async fn validate() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/oauth/validate"))
        .and(header("authorization", "Bearer good"))
        .respond_with(ResponseTemplate::new(200).set_body_raw("{}", "application/json"))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/oauth/validate"))
        .and(header("authorization", "Bearer expired"))
        .respond_with(json(401, "api_error_auth"))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/oauth/validate"))
        .and(header("authorization", "Bearer inference-only"))
        .respond_with(ResponseTemplate::new(403))
        .mount(&server)
        .await;
    let c = client(&server);
    c.validate(&Secret::new("good")).await.unwrap();
    let e = c.validate(&Secret::new("expired")).await.unwrap_err();
    assert!(
        matches!(&e, Error::Unauthorized { status: 401, code: Some(c) } if c == "authentication_error"),
        "{e:?}"
    );
    let e = c
        .validate(&Secret::new("inference-only"))
        .await
        .unwrap_err();
    assert!(
        matches!(
            e,
            Error::Unauthorized {
                status: 403,
                code: None
            }
        ),
        "{e:?}"
    );
}

fn token_mock() -> wiremock::MockBuilder {
    Mock::given(method("POST"))
        .and(path("/v1/oauth/token"))
        .and(header("content-type", "application/json"))
        .and(body_json(serde_json::json!({
            "grant_type": "refresh_token",
            "refresh_token": "fake-refresh-token-1",
            "client_id": "9d1c250a-e61b-44d9-88ed-5944d1962f5e",
            "scope": "user:profile user:inference",
        })))
}

fn refresh_request() -> RefreshRequest {
    RefreshRequest::new(Secret::new("fake-refresh-token-1"))
        .scopes(["user:profile", "user:inference"])
}

#[tokio::test]
async fn refresh_rotates() {
    let server = MockServer::start().await;
    token_mock()
        .respond_with(json(200, "token"))
        .expect(1)
        .mount(&server)
        .await;
    let before = SystemTime::now();
    let t = client(&server).refresh(&refresh_request()).await.unwrap();
    assert_eq!(t.access_token.expose(), "fake-access-token-2");
    assert_eq!(t.refresh_token.expose(), "fake-refresh-token-2");
    assert!(t.rotated);
    assert_eq!(t.expires_in, Duration::from_secs(28800));
    assert!(t.expires_at >= before + Duration::from_secs(28800));
    assert!(t.refresh_token_expires_at.is_some());
    assert_eq!(t.scopes, ["user:inference", "user:profile"]);
    assert_eq!(t.token_type.as_deref(), Some("Bearer"));
    let debug = format!("{t:?}");
    assert!(!debug.contains("fake-"), "{debug}");
}

#[tokio::test]
async fn refresh_without_rotation_keeps_the_posted_token() {
    let server = MockServer::start().await;
    token_mock()
        .respond_with(json(200, "token_no_rotation"))
        .mount(&server)
        .await;
    let t = client(&server).refresh(&refresh_request()).await.unwrap();
    assert_eq!(t.refresh_token.expose(), "fake-refresh-token-1");
    assert!(!t.rotated);
    assert_eq!(t.scopes, [] as [String; 0]);
}

#[tokio::test]
async fn refresh_invalid_grant_is_unauthorized() {
    let server = MockServer::start().await;
    token_mock()
        .respond_with(json(400, "token_invalid_grant"))
        .mount(&server)
        .await;
    let e = client(&server)
        .refresh(&refresh_request())
        .await
        .unwrap_err();
    assert!(e.is_unauthorized(), "{e:?}");
    assert_eq!(e.to_string(), "unauthorized (HTTP 400, invalid_grant)");
}

#[tokio::test]
async fn refresh_other_400_is_http() {
    let server = MockServer::start().await;
    token_mock()
        .respond_with(ResponseTemplate::new(400).set_body_raw(
            r#"{"error":"invalid_request","error_description":"bad scope"}"#,
            "application/json",
        ))
        .mount(&server)
        .await;
    let e = client(&server)
        .refresh(&refresh_request())
        .await
        .unwrap_err();
    assert!(
        matches!(&e, Error::Http { status: 400, code: Some(c), message: Some(m) } if c == "invalid_request" && m == "bad scope"),
        "{e:?}"
    );
    assert!(!e.is_transient());
}

#[tokio::test]
async fn refresh_decode_error_hides_tokens() {
    let server = MockServer::start().await;
    token_mock()
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            r#"{"access_token":"fake-access-token-9","refresh_token":"fake-refresh-token-9","expires_in":"sk-ant-oat01-fake-access-token-9"}"#,
            "application/json",
        ))
        .mount(&server)
        .await;
    let e = client(&server)
        .refresh(&refresh_request())
        .await
        .unwrap_err();
    let Error::Decode {
        status,
        message,
        snippet,
    } = &e
    else {
        panic!("{e:?}")
    };
    assert_eq!(*status, 200);
    for text in [
        message.as_str(),
        snippet.as_str(),
        &e.to_string(),
        &format!("{e:?}"),
    ] {
        assert!(!text.contains("fake-"), "{text}");
        assert!(!text.contains("sk-ant"), "{text}");
    }
    assert!(snippet.contains("expires_in"), "{snippet}");
}

#[tokio::test]
async fn rate_limited_with_retry_after() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/oauth/usage"))
        .respond_with(json(429, "api_error_rate_limit").insert_header("retry-after", "42"))
        .mount(&server)
        .await;
    let e = client(&server)
        .usage(&Secret::new(TOKEN))
        .await
        .unwrap_err();
    assert!(
        matches!(e, Error::RateLimited { retry_after: Some(d) } if d == Duration::from_secs(42)),
        "{e:?}"
    );
    assert!(e.is_transient());
}

#[tokio::test]
async fn rate_limited_with_an_http_date() {
    let server = MockServer::start().await;
    let at = httpdate_in(Duration::from_secs(120));
    Mock::given(method("GET"))
        .and(path("/api/oauth/roles"))
        .respond_with(ResponseTemplate::new(429).insert_header("retry-after", at.as_str()))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/oauth/claude_cli/roles"))
        .respond_with(ResponseTemplate::new(429).insert_header("retry-after", at.as_str()))
        .mount(&server)
        .await;
    let e = client(&server)
        .roles(&Secret::new(TOKEN))
        .await
        .unwrap_err();
    let Error::RateLimited {
        retry_after: Some(d),
    } = e
    else {
        panic!("{e:?}")
    };
    assert!(
        d > Duration::from_secs(100) && d <= Duration::from_secs(120),
        "{d:?}"
    );
}

/// An HTTP date `ahead` from now.
fn httpdate_in(ahead: Duration) -> String {
    httpdate::fmt_http_date(SystemTime::now() + ahead)
}

#[tokio::test]
async fn server_error_is_transient_http() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/oauth/profile"))
        .respond_with(ResponseTemplate::new(503).set_body_raw("<html>down</html>", "text/html"))
        .mount(&server)
        .await;
    let e = client(&server)
        .profile(&Secret::new(TOKEN))
        .await
        .unwrap_err();
    assert!(
        matches!(
            e,
            Error::Http {
                status: 503,
                code: None,
                message: None
            }
        ),
        "{e:?}"
    );
    assert!(e.is_transient());
}

#[tokio::test]
async fn redirects_are_not_followed() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/oauth/usage"))
        .respond_with(
            ResponseTemplate::new(302).insert_header("location", "https://elsewhere.example/"),
        )
        .mount(&server)
        .await;
    let e = client(&server)
        .usage(&Secret::new(TOKEN))
        .await
        .unwrap_err();
    assert!(matches!(e, Error::Http { status: 302, .. }), "{e:?}");
}

#[tokio::test]
async fn oversized_body_is_refused() {
    let server = MockServer::start().await;
    let big = format!("{{\"x\":\"{}\"}}", "a".repeat(ccwire_oauth::MAX_BODY));
    Mock::given(method("GET"))
        .and(path("/api/oauth/usage"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(big, "application/json"))
        .mount(&server)
        .await;
    let e = client(&server)
        .usage(&Secret::new(TOKEN))
        .await
        .unwrap_err();
    assert!(matches!(e, Error::TooLarge { .. }), "{e:?}");
}

#[tokio::test]
async fn timeout_is_transport() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/oauth/usage"))
        .respond_with(json(200, "usage").set_delay(Duration::from_secs(5)))
        .mount(&server)
        .await;
    let c = Client::builder()
        .base_url(server.uri())
        .timeout(Duration::from_millis(100))
        .build()
        .unwrap();
    let e = c.usage(&Secret::new(TOKEN)).await.unwrap_err();
    assert!(matches!(e, Error::Transport(_)), "{e:?}");
    assert!(!format!("{e:?}").contains(&server.uri()));
}

#[tokio::test]
async fn custom_user_agent() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/oauth/roles"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/oauth/claude_cli/roles"))
        .and(header("user-agent", "my-app/1.0"))
        .respond_with(json(200, "roles"))
        .expect(1)
        .mount(&server)
        .await;
    let c = Client::builder()
        .base_url(server.uri())
        .user_agent("my-app/1.0")
        .build()
        .unwrap();
    c.roles(&Secret::new(TOKEN)).await.unwrap();
}
