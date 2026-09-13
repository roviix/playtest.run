use axum::http::{header, uri::Authority, HeaderMap, HeaderValue};
use playtest_common::{GATE_COOKIE, ME_COOKIE, SESSION_COOKIE};

pub const SECURE_ME_COOKIE: &str = "__Host-pt_me";

pub fn cookie_name(secure: bool) -> &'static str {
    if secure {
        SECURE_ME_COOKIE
    } else {
        ME_COOKIE
    }
}

pub fn token<'a>(headers: &'a HeaderMap, secure: bool) -> Option<&'a str> {
    let names = if secure {
        [SECURE_ME_COOKIE, "__Host-pt_session"]
    } else {
        [ME_COOKIE, "pt_session"]
    };
    for name in names {
        let mut found = None;
        let mut ambiguous = false;
        for header in headers.get_all(header::COOKIE) {
            let Ok(raw) = header.to_str() else { continue };
            for part in raw.split(';') {
                if let Some((key, value)) = part.trim().split_once('=') {
                    if key.trim() == name {
                        if found.is_some() {
                            ambiguous = true;
                        }
                        found = Some(value.trim());
                    }
                }
            }
        }
        if ambiguous {
            return None;
        }
        if found.is_some() {
            return found;
        }
    }
    None
}

fn reserved(name: &str) -> bool {
    matches!(
        name,
        ME_COOKIE | SECURE_ME_COOKIE | GATE_COOKIE | SESSION_COOKIE | "pt_session" | "__Host-pt_oauth" | "pt_oauth" | "__Host-pt_session"
    )
}

pub fn strip_request(headers: &mut HeaderMap) {
    let cookies: Vec<_> = headers.get_all(header::COOKIE).iter().cloned().collect();
    headers.remove(header::COOKIE);
    headers.remove("cookie2");
    for cookie in cookies {
        let Ok(raw) = cookie.to_str() else { continue };
        let kept: Vec<_> = raw
            .split(';')
            .filter(|part| {
                part.trim()
                    .split_once('=')
                    .is_some_and(|(name, _)| !reserved(name.trim()))
            })
            .map(str::trim)
            .collect();
        if !kept.is_empty() {
            if let Ok(value) = HeaderValue::from_str(&kept.join("; ")) {
                headers.append(header::COOKIE, value);
            }
        }
    }
}

pub fn strip_response(headers: &mut HeaderMap, authority: &str) {
    let cookies: Vec<_> = headers
        .get_all(header::SET_COOKIE)
        .iter()
        .cloned()
        .collect();
    headers.remove(header::SET_COOKIE);
    headers.remove("set-cookie2");
    let Ok(authority) = authority.parse::<Authority>() else {
        return;
    };
    for cookie in cookies {
        let Ok(raw) = cookie.to_str() else { continue };
        let mut fields = raw.split(';');
        let Some((name, _)) = fields.next().and_then(|field| field.split_once('=')) else {
            continue;
        };
        if reserved(name.trim()) {
            continue;
        }
        let escapes_host = fields.any(|field| {
            field.split_once('=').is_some_and(|(key, value)| {
                key.trim().eq_ignore_ascii_case("domain")
                    && !value
                        .trim()
                        .trim_start_matches('.')
                        .eq_ignore_ascii_case(authority.host())
            })
        });
        if !escapes_host {
            headers.append(header::SET_COOKIE, cookie);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secure_identity_never_accepts_a_legacy_or_ambiguous_cookie() {
        let mut headers = HeaderMap::new();
        headers.append(header::COOKIE, HeaderValue::from_static("pt_me=old"));
        assert_eq!(token(&headers, true), None);
        assert_eq!(token(&headers, false), Some("old"));
        headers.append(header::COOKIE, HeaderValue::from_static("__Host-pt_me=new"));
        assert_eq!(token(&headers, true), Some("new"));
        headers.append(
            header::COOKIE,
            HeaderValue::from_static("__Host-pt_me=other"),
        );
        assert_eq!(token(&headers, true), None);
    }

    #[test]
    fn only_game_cookies_reach_an_untrusted_backend() {
        let mut headers = HeaderMap::new();
        headers.append(
            header::COOKIE,
            HeaderValue::from_static("game=level=2; pt_me=old; pt_sid=session"),
        );
        headers.append(
            header::COOKIE,
            HeaderValue::from_static("__Host-pt_me=secret; pt_gate=1; theme=dark"),
        );
        headers.insert("cookie2", HeaderValue::from_static("pt_me=old"));
        strip_request(&mut headers);
        let values: Vec<_> = headers
            .get_all(header::COOKIE)
            .iter()
            .map(|value| value.to_str().unwrap())
            .collect();
        assert_eq!(values, ["game=level=2", "theme=dark"]);
        assert!(!headers.contains_key("cookie2"));
        strip_request(&mut headers);
        assert_eq!(headers.get_all(header::COOKIE).iter().count(), 2);
    }

    #[test]
    fn upstream_cannot_set_platform_or_parent_domain_cookies() {
        let mut headers = HeaderMap::new();
        for value in [
            "pt_me=stolen; Path=/",
            "__Host-pt_me=stolen; Secure; Path=/",
            "pt_sid=forged; Path=/",
            "pt_gate=1; Path=/",
            "shared=bad; Domain=.playtest.run",
            "shared=bad; dOmAiN=playtest.run",
            "shared=bad; Domain=game.playtest.run; Domain=playtest.run",
            "game=ok; HttpOnly; Path=/",
            "theme=dark; Domain=.GAME.playtest.run; Path=/",
        ] {
            headers.append(header::SET_COOKIE, HeaderValue::from_static(value));
        }
        strip_response(&mut headers, "game.playtest.run:443");
        let values: Vec<_> = headers
            .get_all(header::SET_COOKIE)
            .iter()
            .map(|value| value.to_str().unwrap())
            .collect();
        assert_eq!(
            values,
            [
                "game=ok; HttpOnly; Path=/",
                "theme=dark; Domain=.GAME.playtest.run; Path=/"
            ]
        );
    }
}
