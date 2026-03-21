use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use tower_cookies::{Cookie, Cookies};

use crate::db::AppState;

const COOKIE_NAME: &str = "analytics_session";

/// AppState extended with auth config
#[derive(Clone)]
pub struct AuthConfig {
    pub password_hash: String,
    pub cookie_secret: Vec<u8>,
}

/// Create an HMAC-signed session token
fn create_session_token(secret: &[u8]) -> String {
    let timestamp = chrono::Utc::now().timestamp().to_string();
    let mut mac =
        Hmac::<Sha256>::new_from_slice(secret).expect("HMAC can take key of any size");
    mac.update(timestamp.as_bytes());
    let signature = hex::encode(mac.finalize().into_bytes());
    format!("{timestamp}.{signature}")
}

/// Verify an HMAC-signed session token
fn verify_session_token(token: &str, secret: &[u8]) -> bool {
    let Some((timestamp, signature)) = token.split_once('.') else {
        return false;
    };

    let mut mac =
        Hmac::<Sha256>::new_from_slice(secret).expect("HMAC can take key of any size");
    mac.update(timestamp.as_bytes());
    let expected = hex::encode(mac.finalize().into_bytes());

    // Constant-time comparison
    expected == signature
}

/// Middleware that requires a valid session cookie.
/// Redirects to /dashboard/login if not authenticated.
pub async fn require_auth(
    State(state): State<AppState>,
    cookies: Cookies,
    request: Request,
    next: Next,
) -> Response {
    if let Some(cookie) = cookies.get(COOKIE_NAME) {
        if verify_session_token(cookie.value(), &state.auth.cookie_secret) {
            return next.run(request).await;
        }
    }

    // Check if this is an API request (return 401 instead of redirect)
    if request.uri().path().starts_with("/api/") {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

    Redirect::to("/dashboard/login").into_response()
}

/// Render login page (plain HTML, will be replaced by Askama template in Phase 5)
pub async fn login_page(
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let error_msg = if params.contains_key("error") {
        r#"<p style="color: #e74c3c; margin-bottom: 1rem;">Invalid password. Please try again.</p>"#
    } else {
        ""
    };

    axum::response::Html(format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Login - Blog Analytics</title>
    <style>
        body {{ font-family: system-ui, -apple-system, sans-serif; display: flex; justify-content: center; align-items: center; min-height: 100vh; margin: 0; background: #f5f5f5; }}
        .login-box {{ background: white; padding: 2rem; border-radius: 8px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); width: 100%; max-width: 360px; }}
        h1 {{ margin: 0 0 1.5rem; font-size: 1.5rem; text-align: center; }}
        input[type="password"] {{ width: 100%; padding: 0.75rem; border: 1px solid #ddd; border-radius: 4px; font-size: 1rem; box-sizing: border-box; }}
        button {{ width: 100%; padding: 0.75rem; background: #2563eb; color: white; border: none; border-radius: 4px; font-size: 1rem; cursor: pointer; margin-top: 1rem; }}
        button:hover {{ background: #1d4ed8; }}
    </style>
</head>
<body>
    <div class="login-box">
        <h1>Blog Analytics</h1>
        {error_msg}
        <form method="POST" action="/dashboard/login">
            <input type="password" name="password" placeholder="Password" autofocus required>
            <button type="submit">Sign In</button>
        </form>
    </div>
</body>
</html>"#
    ))
}

/// Handle login form submission
pub async fn login_submit(
    State(state): State<AppState>,
    cookies: Cookies,
    axum::Form(form): axum::Form<LoginForm>,
) -> Response {
    match bcrypt::verify(&form.password, &state.auth.password_hash) {
        Ok(true) => {
            let token = create_session_token(&state.auth.cookie_secret);
            let cookie = Cookie::build((COOKIE_NAME, token))
                .path("/")
                .http_only(true)
                .same_site(tower_cookies::cookie::SameSite::Lax)
                .max_age(tower_cookies::cookie::time::Duration::days(30))
                .build();
            cookies.add(cookie);
            Redirect::to("/dashboard").into_response()
        }
        _ => {
            // Return to login with error
            Redirect::to("/dashboard/login?error=1").into_response()
        }
    }
}

/// Handle logout
pub async fn logout(cookies: Cookies) -> impl IntoResponse {
    let cookie = Cookie::build((COOKIE_NAME, ""))
        .path("/")
        .max_age(tower_cookies::cookie::time::Duration::seconds(0))
        .build();
    cookies.remove(cookie);
    Redirect::to("/dashboard/login")
}

#[derive(serde::Deserialize)]
pub struct LoginForm {
    pub password: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_token_roundtrip() {
        let secret = b"test-secret-key-1234567890";
        let token = create_session_token(secret);
        assert!(verify_session_token(&token, secret));
    }

    #[test]
    fn test_session_token_invalid() {
        let secret = b"test-secret-key-1234567890";
        assert!(!verify_session_token("invalid", secret));
        assert!(!verify_session_token("123.abc", secret));
    }
}
