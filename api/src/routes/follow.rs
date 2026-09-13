//! 关注：登记、确认、退订、「我的」（DESIGN §3.6、§3.10）。
//!
//! 这一组和 `events` 一样**不带开发者令牌**，但调用方不是玩家的浏览器而是边缘：
//! 玩家把邮箱交给自己所在的那个域，边缘再内网转给这里（DESIGN §4.1 两个域不混）。
//! 所以这里的防线不是 Origin，而是限速——按调用方 IP 和邮箱各一个桶（DESIGN §4.8）。
//!
//! 两条铁律：
//!
//! - **邮箱一律要双重确认。** 谁都能替别人填一个邮箱，所以「已经确认过的人又用邮箱关注」
//!   也走确认信，不直接生效。
//! - **不泄露一个邮箱在不在库里。** 「把链接寄给我」不管有没有这个人都回同一句话。

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::{Extension, Json};
use playtest_common::follow::{
    looks_like_email, mask_email, ConfirmRequest, ConfirmResponse, FollowChannel, FollowRequest,
    FollowResponse, FollowTarget, FollowView, MeRequest, MeView, SendLinkRequest, UnfollowRequest,
    UnsubscribeRequest,
};
use playtest_common::hash;
use rusqlite::Connection;
use time::OffsetDateTime;

use crate::clock;
use crate::db;
use crate::error::{ApiError, ApiResult};
use crate::notify;
use crate::routes::events::{header_str, Limiter};
use crate::routes::JsonBody;
use crate::state::AppState;

/// 令牌的字节数，和开发者令牌一样：32 字节随机量，base64url 之后 43 个字符。
const TOKEN_BYTES: usize = 32;

const BAD_EMAIL: &str = "这个邮箱的写法不对，检查一下有没有打错。";
const NO_SUCH_SITE: &str = "没有这个作品，或者它的链接已经失效了。";
const BAD_ME_TOKEN: &str = "这个链接已经不能用了。到信箱里找最近那封信，或者重新填一次邮箱。";
const BAD_CONFIRM: &str = "这个确认链接已经用过或者过期了。重新填一次邮箱，我们再发一封。";
const TOO_MANY: &str = "试得太频繁了，等一分钟再来。";

/// `POST /v1/follow`
pub async fn follow(
    State(state): State<AppState>,
    Extension(limiter): Extension<Limiter>,
    headers: HeaderMap,
    JsonBody(request): JsonBody<FollowRequest>,
) -> ApiResult<Json<FollowResponse>> {
    let now = clock::now();
    let (kind, slug) = split_target(&request.target);
    if let Some(slug) = slug {
        // 关注一个不存在的作品没有意义，而且会让「关注数」上出现查不到的行。
        let conn = state.db().lock().await;
        let exists = if kind == "collection" {
            crate::collections::public_one(&conn, slug, &clock::format(now))?.is_some()
        } else {
            db::find_site(&conn, slug)?.is_some()
        };
        if !exists {
            return Err(ApiError::not_found(NO_SUCH_SITE));
        }
    }
    if kind == "collection" {
        if !state.notify().email_on() {
            return Err(ApiError::invalid("邮件摘要目前不可用，请稍后再关注。"));
        }
        match &request.channel {
            FollowChannel::Push { .. } => {
                return Err(ApiError::invalid("合集只发送邮件摘要，请使用邮箱关注。"))
            }
            FollowChannel::Me { me_token } => {
                let conn = state.db().read().await;
                if player_by_me_token(&conn, me_token)?
                    .email_verified_at
                    .is_none()
                {
                    return Err(ApiError::invalid(
                        "合集摘要需要已确认的邮箱，请填写邮箱关注。",
                    ));
                }
            }
            _ => {}
        }
    }

    match request.channel {
        FollowChannel::Email { email } => {
            let email = email.trim().to_ascii_lowercase();
            if !looks_like_email(&email) {
                return Err(ApiError::invalid(BAD_EMAIL));
            }
            spend(&limiter, &headers, Some(&email))?;
            let token = new_token();
            {
                let conn = state.db().lock().await;
                let player = find_or_create_by_email(&conn, &email, now)?;
                // 关注的目标挂在令牌上，点了确认才写进 follows：
                // 谁都能替别人填邮箱，没点过的那一下不算数（DESIGN §4.8）。
                db::insert_follow_token(
                    &conn,
                    &db::NewFollowToken {
                        token_hash: &hash::hash_bytes(token.as_bytes()),
                        player_id: &player,
                        purpose: notify::KIND_CONFIRM,
                        target_kind: Some(kind),
                        target_slug: slug,
                        source: request.from.as_deref(),
                        created_at: &clock::format(now),
                        expires_at: &clock::format(clock::plus_hours(
                            now,
                            notify::CONFIRM_TOKEN_HOURS,
                        )),
                    },
                )?;
                let title = match slug {
                    Some(slug) if kind == "collection" => {
                        crate::collections::public_one(&conn, slug, &clock::format(now))?
                            .map(|collection| collection.title)
                    }
                    Some(slug) => db::find_site(&conn, slug)?.map(|s| s.title),
                    None => None,
                };
                if kind == "collection" {
                    crate::collections::enqueue_confirmation(
                        &conn,
                        &player,
                        title.as_deref().unwrap_or("合集"),
                        &state.notify().confirm_url(&token),
                        now,
                    )?;
                } else {
                    notify::enqueue_confirm(
                        &conn,
                        &player,
                        title.as_deref(),
                        &state.notify().confirm_url(&token),
                        now,
                    )?;
                }
            }
            Ok(Json(FollowResponse::ConfirmSent))
        }
        FollowChannel::Push { subscription } => {
            spend(&limiter, &headers, None)?;
            let json = serde_json::to_string(&subscription)?;
            let (added, slugs) = {
                let conn = state.db().lock().await;
                let player = find_or_create_by_push(&conn, &subscription.endpoint, &json, now)?;
                let added = db::insert_follow(
                    &conn,
                    &player,
                    kind,
                    slug,
                    request.from.as_deref(),
                    &clock::format(now),
                )?;
                (added, slug.map(str::to_string))
            };
            if let Some(slug) = slugs {
                crate::live::publish(&state, &slug).await;
            }
            Ok(Json(if added {
                FollowResponse::Subscribed
            } else {
                FollowResponse::AlreadyFollowing
            }))
        }
        FollowChannel::Me { me_token } => {
            spend(&limiter, &headers, None)?;
            let added = {
                let conn = state.db().lock().await;
                let player = player_by_me_token(&conn, &me_token)?;
                db::insert_follow(
                    &conn,
                    &player.id,
                    kind,
                    slug,
                    request.from.as_deref(),
                    &clock::format(now),
                )?
            };
            if let Some(slug) = slug {
                crate::live::publish(&state, slug).await;
            }
            Ok(Json(if added {
                FollowResponse::Subscribed
            } else {
                FollowResponse::AlreadyFollowing
            }))
        }
    }
}

/// `POST /v1/follow/confirm`：确认信里的令牌换一个长期的 `me_token`。
pub async fn confirm(
    State(state): State<AppState>,
    Extension(limiter): Extension<Limiter>,
    headers: HeaderMap,
    JsonBody(request): JsonBody<ConfirmRequest>,
) -> ApiResult<Json<ConfirmResponse>> {
    spend(&limiter, &headers, None)?;
    let now = clock::now();
    let (me_token, me, slug) = {
        let conn = state.db().lock().await;
        let taken = db::take_follow_token(
            &conn,
            &hash::hash_bytes(request.token.trim().as_bytes()),
            &clock::format(now),
        )?
        .ok_or_else(|| ApiError::not_found(BAD_CONFIRM))?;

        db::confirm_player_email(&conn, &taken.player_id, &clock::format(now))?;
        let mut slug = None;
        if let Some(kind) = taken.target_kind.as_deref() {
            if kind == "collection"
                && crate::collections::public_one(
                    &conn,
                    taken.target_slug.as_deref().unwrap_or_default(),
                    &clock::format(now),
                )?
                .is_none()
            {
                return Err(ApiError::not_found("这个合集已停止公开，没有新增关注。"));
            }
            db::insert_follow(
                &conn,
                &taken.player_id,
                kind,
                taken.target_slug.as_deref(),
                taken.source.as_deref(),
                &clock::format(now),
            )?;
            slug = taken.target_slug.clone();
        }

        // 同一个人换了设备再确认一次，就换一把新的：旧设备上的那一把随之失效。
        // 这是「换设备」唯一的路，也是撤销一台丢失设备的路（DESIGN §3.10）。
        let user_id = crate::account::user_for_player(&conn, &taken.player_id)?;
        let me_token = crate::account::create_session(&conn, &user_id)?;
        db::set_player_me_token(
            &conn,
            &taken.player_id,
            &hash::hash_bytes(me_token.as_bytes()),
        )?;
        let player = db::find_player(&conn, &taken.player_id)?
            .ok_or_else(|| ApiError::not_found(BAD_CONFIRM))?;
        (me_token, view_of(&state, &conn, &player)?, slug)
    };
    if let Some(slug) = slug {
        crate::live::publish(&state, &slug).await;
    }
    Ok(Json(ConfirmResponse { me_token, me }))
}

pub async fn preview(State(state): State<AppState>, JsonBody(request): JsonBody<ConfirmRequest>) -> ApiResult<Json<serde_json::Value>> {
    use rusqlite::OptionalExtension;
    let conn = state.db().read().await;
    let email: Option<String> = conn.query_row("SELECT p.email FROM follow_tokens t JOIN players p ON p.id=t.player_id WHERE t.token_hash=?1 AND t.used_at IS NULL AND t.expires_at>?2",rusqlite::params![hash::hash_bytes(request.token.as_bytes()),clock::now_string()],|row|row.get(0)).optional()?;
    let email = email.ok_or_else(||ApiError::not_found(BAD_CONFIRM))?;
    Ok(Json(serde_json::json!({"email":mask_email(&email)})))
}

/// `POST /v1/follow/unsubscribe`：每封信底部那个链接。点了就退，不问为什么。
pub async fn unsubscribe(
    State(state): State<AppState>,
    JsonBody(request): JsonBody<UnsubscribeRequest>,
) -> ApiResult<Json<MeView>> {
    let slugs = {
        let conn = state.db().lock().await;
        let Some(player) = db::find_player_by_unsubscribe_token(&conn, request.token.trim())?
        else {
            // 退订链接不该报错：多半是点了两次。回一个空的「我的」，页面显示「已经退订了」。
            return Ok(Json(MeView::default()));
        };
        db::unsubscribe_player(&conn, &player.id, &clock::now_string())?
    };
    crate::live::publish_all(&state, &slugs).await;
    Ok(Json(MeView::default()))
}

/// `POST /v1/me/view`
pub async fn me_view(
    State(state): State<AppState>,
    JsonBody(request): JsonBody<MeRequest>,
) -> ApiResult<Json<MeView>> {
    let conn = state.db().read().await;
    let player = player_by_me_token(&conn, &request.me_token)?;
    Ok(Json(view_of(&state, &conn, &player)?))
}

/// `POST /v1/me/unfollow`
pub async fn unfollow(
    State(state): State<AppState>,
    JsonBody(request): JsonBody<UnfollowRequest>,
) -> ApiResult<Json<MeView>> {
    let (kind, slug) = split_target(&request.target);
    let view = {
        let conn = state.db().lock().await;
        let player = player_by_me_token(&conn, &request.me_token)?;
        db::delete_follow(&conn, &player.id, kind, slug)?;
        view_of(&state, &conn, &player)?
    };
    if let Some(slug) = slug {
        crate::live::publish(&state, slug).await;
    }
    Ok(Json(view))
}

/// `POST /v1/me/push-off`：关掉这台设备的浏览器通知。只清推送订阅，关注留着。
pub async fn push_off(
    State(state): State<AppState>,
    JsonBody(request): JsonBody<MeRequest>,
) -> ApiResult<Json<MeView>> {
    let conn = state.db().lock().await;
    let mut player = player_by_me_token(&conn, &request.me_token)?;
    db::clear_player_push(&conn, &player.id)?;
    player.push_subscription = None;
    player.push_endpoint = None;
    Ok(Json(view_of(&state, &conn, &player)?))
}

/// `POST /v1/me/send-link`：换了设备，把能打开关注页的链接寄给自己。
///
/// 不管这个邮箱在不在库里，回的都是同一句话——否则这个端点就成了一台
/// 「这个邮箱注册过没有」的查询机（DESIGN §4.8）。
pub async fn send_link(
    State(state): State<AppState>,
    Extension(limiter): Extension<Limiter>,
    headers: HeaderMap,
    JsonBody(request): JsonBody<SendLinkRequest>,
) -> ApiResult<Json<FollowResponse>> {
    let email = request.email.trim().to_ascii_lowercase();
    if !looks_like_email(&email) {
        return Err(ApiError::invalid(BAD_EMAIL));
    }
    spend(&limiter, &headers, Some(&email))?;

    let now = clock::now();
    {
        let conn = state.db().lock().await;
        if let Some(player) = db::find_player_by_email(&conn, &email)? {
            if player.email_verified_at.is_some() && player.unsubscribed_at.is_none() {
                let token = new_token();
                db::insert_follow_token(
                    &conn,
                    &db::NewFollowToken {
                        token_hash: &hash::hash_bytes(token.as_bytes()),
                        player_id: &player.id,
                        purpose: notify::KIND_SEND_LINK,
                        // 不带目标：这封信只是换一把 me_token，不新增关注。
                        target_kind: None,
                        target_slug: None,
                        source: None,
                        created_at: &clock::format(now),
                        expires_at: &clock::format(clock::plus_hours(
                            now,
                            notify::CONFIRM_TOKEN_HOURS,
                        )),
                    },
                )?;
                notify::enqueue_send_link(
                    &conn,
                    &player.id,
                    &state.notify().confirm_url(&token),
                    now,
                )?;
            }
        }
    }
    Ok(Json(FollowResponse::ConfirmSent))
}

// ---------------------------------------------------------------- 零件

fn split_target(target: &FollowTarget) -> (&'static str, Option<&str>) {
    match target {
        FollowTarget::Site { slug } => ("site", Some(slug.as_str())),
        FollowTarget::Collection { slug } => ("collection", Some(slug.as_str())),
        FollowTarget::Plaza => ("plaza", None),
    }
}

fn new_token() -> String {
    use base64::Engine;
    let mut bytes = [0u8; TOKEN_BYTES];
    rand::RngCore::fill_bytes(&mut rand::rng(), &mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn player_by_me_token(conn: &Connection, me_token: &str) -> ApiResult<db::PlayerRow> {
    let player = if let Some(identity) = crate::account::session_owner(conn, me_token)? {
        use rusqlite::OptionalExtension;
        let player_id: Option<String> = conn.query_row("SELECT id FROM players WHERE user_id=?1", rusqlite::params![identity.user_id], |row| row.get(0)).optional()?;
        player_id.and_then(|id| db::find_player(conn, &id).ok().flatten())
    } else {
        db::find_player_by_me_token(conn, &hash::hash_bytes(me_token.trim().as_bytes()))?
    };
    player
        .filter(|p| p.unsubscribed_at.is_none())
        .ok_or_else(|| ApiError::unauthorized(BAD_ME_TOKEN))
}

pub(crate) fn find_or_create_by_email(
    conn: &Connection,
    email: &str,
    now: OffsetDateTime,
) -> rusqlite::Result<String> {
    if let Some(player) = db::find_player_by_email(conn, email)? {
        return Ok(player.id);
    }
    let id = uuid::Uuid::new_v4().to_string();
    db::insert_player(
        conn,
        &db::NewPlayer {
            id: &id,
            email: Some(email),
            push_subscription: None,
            push_endpoint: None,
            unsubscribe_token: &new_token(),
            created_at: &clock::format(now),
        },
    )?;
    Ok(id)
}

fn find_or_create_by_push(
    conn: &Connection,
    endpoint: &str,
    json: &str,
    now: OffsetDateTime,
) -> rusqlite::Result<String> {
    if let Some(player) = db::find_player_by_push(conn, endpoint)? {
        db::set_player_push(conn, &player.id, json, endpoint)?;
        return Ok(player.id);
    }
    let id = uuid::Uuid::new_v4().to_string();
    db::insert_player(
        conn,
        &db::NewPlayer {
            id: &id,
            email: None,
            push_subscription: Some(json),
            push_endpoint: Some(endpoint),
            unsubscribe_token: &new_token(),
            created_at: &clock::format(now),
        },
    )?;
    Ok(id)
}

fn view_of(state: &AppState, conn: &Connection, player: &db::PlayerRow) -> ApiResult<MeView> {
    let collections = crate::collections::list(conn, None, &clock::now_string())?;
    let follows = db::follows_of(conn, &player.id)?
        .into_iter()
        .filter_map(|mut row| {
            let target = match (row.target_kind.as_str(), row.target_slug) {
                ("site", Some(slug)) => FollowTarget::Site { slug },
                ("collection", Some(slug)) => {
                    let collection = collections
                        .iter()
                        .find(|collection| collection.slug == slug)?;
                    row.title = Some(collection.title.clone());
                    FollowTarget::Collection { slug }
                }
                ("plaza", _) => FollowTarget::Plaza,
                _ => return None,
            };
            let url = match &target {
                FollowTarget::Site { slug } => Some(state.site_url(slug)),
                FollowTarget::Collection { slug } => {
                    Some(format!("{}/c/{slug}", state.notify().root_url()))
                }
                FollowTarget::Plaza => None,
            };
            // 作品已经删掉的那几条查不到标题，也就不显示（`follows_of` 用的是 LEFT JOIN）。
            if matches!(target, FollowTarget::Site { .. }) && row.title.is_none() {
                return None;
            }
            Some(FollowView {
                target,
                title: row.title,
                url,
                since: row.since,
            })
        })
        .collect();
    Ok(MeView {
        email_masked: player.email.as_deref().map(mask_email),
        push: player.push_endpoint.is_some(),
        follows,
    })
}

/// 两把桶：调用方 IP 一把，邮箱一把（DESIGN §4.8）。
///
/// 复用 `events` 那个令牌桶的实现。IP 那把挡「一台机器狂刷」，邮箱那把挡
/// 「同一个邮箱被反复轰确认信」——后者更重要，被拿来轰炸的是别人的信箱。
pub(crate) fn spend(limiter: &Limiter, headers: &HeaderMap, email: Option<&str>) -> ApiResult<()> {
    // 调用方是边缘，它把玩家 IP 放在 `x-forwarded-for` 里；没有这个头就退回
    // 一个共用的桶——宁可整条内网链路共享额度，也不要限速悄悄失效。
    let ip = header_str(headers, "x-forwarded-for")
        .and_then(|v| v.split(',').next())
        .map(|v| v.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    // 邮箱那把用紧的那个桶（几十次就见底）：被轰的是别人的信箱，那比一台机器刷我们更要紧。
    // 邮箱只存哈希，限流的桶键也不例外——进程内存里同样不该留明文邮箱。
    let ok = match email {
        Some(email) => limiter.take(
            &format!("em:{}", hash::hash_bytes(email.as_bytes())),
            &format!("ip:{ip}"),
        ),
        None => limiter.take_slug(&format!("ip:{ip}")),
    };
    if ok {
        Ok(())
    } else {
        Err(ApiError::public(
            StatusCode::TOO_MANY_REQUESTS,
            playtest_common::api::ErrorCode::QuotaExceeded,
            TOO_MANY,
        ))
    }
}
