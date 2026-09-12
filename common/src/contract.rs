//! 契约的单一来源：哪些类型跨进程、走哪些路径、带什么体、回什么体。
//!
//! 控制台的 TS 类型（`console/src/generated/api.ts`）和控制面的 `openapi.json` 都从这里生成，
//! Rust 改了字段，跑一次 `cargo run -p playtest-common --bin playtest-contract` 两边就跟上；
//! 忘了跑，`generated_typescript_is_current` 这条测试会红。手抄的那一层就此没有了。

use std::collections::BTreeMap;

use schemars::generate::{SchemaGenerator, SchemaSettings};
use schemars::JsonSchema;
use serde_json::{json, Map, Value};

use crate::api::routes as api_paths;
use crate::boost::routes as admin_paths;
use crate::collection::routes as collection_paths;
use crate::follow::routes as follow_paths;
use crate::ingest::routes as ingest_paths;
use crate::results::routes as result_paths;

/// 一条 HTTP 端点：方法、路径模板、一句话、请求体类型名、响应体类型名。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Endpoint {
    pub method: &'static str,
    pub path: &'static str,
    /// 给 agent 读的一句英文，`llms-full.txt` 与 OpenAPI `summary` 用同一句。
    pub summary: &'static str,
    /// 请求体的类型名（components/schemas 里的键）；没有体就是 `None`。
    pub request: Option<&'static str>,
    /// 成功响应体的类型名；`None` 是 204 或非 JSON。
    pub response: Option<&'static str>,
    pub auth: Auth,
}

/// 谁能调。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Auth {
    /// 不带令牌。
    Public,
    /// `Authorization: Bearer <开发者令牌>`。
    Bearer,
    /// 页面里的 SDK / 边缘发来的，按来源域名放行，不带令牌。
    Ingest,
    /// 运营者令牌。
    Admin,
}

const fn ep(
    method: &'static str,
    path: &'static str,
    summary: &'static str,
    request: Option<&'static str>,
    response: Option<&'static str>,
    auth: Auth,
) -> Endpoint {
    Endpoint {
        method,
        path,
        summary,
        request,
        response,
        auth,
    }
}

/// 端点表。`llms-full.txt`、`openapi.json`、路由一致性测试都读这一份。
pub fn endpoints() -> Vec<Endpoint> {
    use Auth::*;
    vec![
        ep(
            "GET",
            api_paths::HEALTH,
            "is this control plane up",
            None,
            None,
            Public,
        ),
        ep(
            "GET",
            collection_paths::COLLECTIONS,
            "the caller's collections",
            None,
            Some("CollectionList"),
            Bearer,
        ),
        ep(
            "POST",
            collection_paths::COLLECTIONS,
            "create a collection or a creative challenge",
            Some("CollectionDraft"),
            Some("Collection"),
            Bearer,
        ),
        ep(
            "GET",
            collection_paths::OPEN,
            "published collections and challenges",
            None,
            Some("CollectionList"),
            Bearer,
        ),
        ep(
            "GET",
            collection_paths::COLLECTION,
            "view a collection",
            None,
            Some("Collection"),
            Bearer,
        ),
        ep(
            "PUT",
            collection_paths::COLLECTION,
            "update collection details and visibility",
            Some("CollectionDraft"),
            Some("Collection"),
            Bearer,
        ),
        ep(
            "DELETE",
            collection_paths::COLLECTION,
            "remove a collection, not its projects",
            None,
            None,
            Bearer,
        ),
        ep(
            "POST",
            collection_paths::ENTRIES,
            "submit an owned public project with self-reported creation notes",
            Some("EntryDraft"),
            Some("Collection"),
            Bearer,
        ),
        ep(
            "DELETE",
            collection_paths::ENTRY,
            "withdraw or moderate a submission",
            None,
            None,
            Bearer,
        ),
        ep(
            "DELETE",
            collection_paths::BLOCK,
            "allow a moderated author to submit again",
            None,
            None,
            Bearer,
        ),
        ep(
            "PUT",
            collection_paths::MODERATE,
            "hide or restore a collection",
            Some("ModerateCollectionRequest"),
            None,
            Admin,
        ),
        ep(
            "POST",
            api_paths::ANON_SESSIONS,
            "an anonymous token; the links it makes expire",
            None,
            Some("AnonSessionResponse"),
            Public,
        ),
        ep(
            "POST",
            api_paths::LOGIN_DEVICE_START,
            "start a GitHub device login; show the user the code",
            None,
            Some("DeviceLoginStart"),
            Public,
        ),
        ep(
            "POST",
            api_paths::LOGIN_DEVICE_POLL,
            "poll until the user has entered the code",
            Some("DeviceLoginPoll"),
            Some("LoginPollResponse"),
            Public,
        ),
        ep(
            "POST",
            api_paths::LOGIN_WEB_EXCHANGE,
            "trade the code GitHub sent the browser back with for a token",
            Some("WebLoginExchange"),
            Some("LoginResponse"),
            Public,
        ),
        ep(
            "GET",
            api_paths::ME,
            "who this token belongs to",
            None,
            Some("Me"),
            Bearer,
        ),
        ep(
            "DELETE",
            api_paths::ME_TOKEN,
            "revoke only this API token; keep projects and other devices",
            None,
            None,
            Bearer,
        ),
        ep(
            "GET",
            api_paths::SITES,
            "the caller's projects",
            None,
            Some("SiteList"),
            Bearer,
        ),
        ep(
            "POST",
            api_paths::SITES,
            "create a project",
            Some("CreateSiteRequest"),
            Some("Site"),
            Bearer,
        ),
        ep(
            "GET",
            api_paths::SITE,
            "one project as it is right now",
            None,
            Some("Site"),
            Bearer,
        ),
        ep(
            "PATCH",
            api_paths::SITE,
            "change plaza listing, seats, community link, public feedback",
            Some("UpdateSiteRequest"),
            Some("Site"),
            Bearer,
        ),
        ep(
            "DELETE",
            api_paths::SITE,
            "delete it; the link stops working",
            None,
            None,
            Bearer,
        ),
        ep(
            "POST",
            api_paths::SITE_UPLOADS,
            "declare the files, learn which hashes are missing",
            Some("PrepareUploadRequest"),
            Some("PrepareUploadResponse"),
            Bearer,
        ),
        ep(
            "PUT",
            api_paths::BLOB,
            "upload one file by content hash (raw bytes, not JSON)",
            None,
            None,
            Bearer,
        ),
        ep(
            "POST",
            api_paths::SITE_UPLOAD_COMMIT,
            "make the uploaded files the live version",
            None,
            Some("CommitUploadResponse"),
            Bearer,
        ),
        ep(
            "GET",
            api_paths::SITE_VERSIONS,
            "every version, newest first",
            None,
            Some("VersionList"),
            Bearer,
        ),
        ep(
            "POST",
            api_paths::SITE_VERSION_ACTIVATE,
            "roll back: point players at an older version",
            None,
            Some("Site"),
            Bearer,
        ),
        ep(
            "GET",
            api_paths::SITE_VERSION_FILES,
            "what is actually in a version: path, size, sha256, direct url",
            None,
            Some("VersionFiles"),
            Bearer,
        ),
        ep(
            "POST",
            api_paths::SITE_TUNNEL,
            "a signed token for the tunnel (the CLI uses this)",
            Some("TunnelRequest"),
            Some("TunnelGrant"),
            Bearer,
        ),
        ep(
            "GET",
            result_paths::SITE_RESULTS,
            "per-version roster: opened, started, stayed, where they came from",
            None,
            Some("SiteResults"),
            Bearer,
        ),
        ep(
            "GET",
            result_paths::SITE_VERSION_SESSIONS,
            "one row per player for a version",
            None,
            Some("VersionSessions"),
            Bearer,
        ),
        ep(
            "GET",
            result_paths::SITE_FEEDBACK,
            "what players wrote",
            None,
            Some("FeedbackList"),
            Bearer,
        ),
        ep(
            "PATCH",
            result_paths::SITE_FEEDBACK_ITEM,
            "mark a note seen/done, or hide it",
            Some("UpdateFeedbackRequest"),
            Some("FeedbackItem"),
            Bearer,
        ),
        ep(
            "POST",
            ingest_paths::EVENTS,
            "the in-page SDK reports errors and milestones here",
            Some("EventBatch"),
            Some("Accepted"),
            Ingest,
        ),
        ep(
            "POST",
            ingest_paths::FEEDBACK,
            "the in-page SDK submits a player's note here",
            Some("FeedbackRequest"),
            Some("FeedbackAccepted"),
            Ingest,
        ),
        ep(
            "POST",
            follow_paths::FOLLOW,
            "a player follows a project or a developer (the gate page calls this)",
            Some("FollowRequest"),
            Some("FollowResponse"),
            Ingest,
        ),
        ep(
            "POST",
            follow_paths::CONFIRM,
            "confirm an e-mail follow with the token from the mail",
            Some("ConfirmRequest"),
            Some("ConfirmResponse"),
            Public,
        ),
        ep(
            "POST",
            follow_paths::UNSUBSCRIBE,
            "one-click unsubscribe from the mail footer",
            Some("UnsubscribeRequest"),
            Some("MeView"),
            Public,
        ),
        ep(
            "POST",
            follow_paths::ME_VIEW,
            "what a player follows (by the player's own token)",
            Some("MeRequest"),
            Some("MeView"),
            Public,
        ),
        ep(
            "POST",
            follow_paths::ME_UNFOLLOW,
            "stop following one thing",
            Some("UnfollowRequest"),
            Some("MeView"),
            Public,
        ),
        ep(
            "POST",
            follow_paths::ME_PUSH_OFF,
            "turn browser notifications off, keep the follows",
            Some("MeRequest"),
            Some("MeView"),
            Public,
        ),
        ep(
            "POST",
            follow_paths::ME_SEND_LINK,
            "mail the player a link to their own page",
            Some("SendLinkRequest"),
            Some("FollowResponse"),
            Public,
        ),
        ep(
            "GET",
            admin_paths::BOOSTS,
            "every boost, newest first",
            None,
            Some("BoostList"),
            Admin,
        ),
        ep(
            "POST",
            admin_paths::BOOSTS,
            "grant a boost without payment",
            Some("GrantBoostRequest"),
            Some("Boost"),
            Admin,
        ),
        ep(
            "POST",
            admin_paths::BOOST_REVIEW,
            "approve or reject a pending boost",
            Some("ReviewBoostRequest"),
            Some("Boost"),
            Admin,
        ),
        ep(
            "DELETE",
            admin_paths::BOOST,
            "end a boost early",
            None,
            None,
            Admin,
        ),
        ep(
            "POST",
            admin_paths::PLAZA_HIDE,
            "take a project off the plaza (or put it back)",
            None,
            None,
            Admin,
        ),
        ep(
            "GET",
            admin_paths::NOTIFICATIONS,
            "how the notification queue looks",
            None,
            Some("NotificationQueue"),
            Admin,
        ),
        ep(
            "GET",
            admin_paths::JOBS,
            "background jobs: when each last ran and how it went",
            None,
            Some("JobList"),
            Admin,
        ),
    ]
}

/// 路径常量表：名字 + 模板。TS 里生成成函数（有 `{参数}` 的）或字符串。
pub fn paths() -> Vec<(&'static str, &'static str)> {
    vec![
        ("health", api_paths::HEALTH),
        ("anonSessions", api_paths::ANON_SESSIONS),
        ("loginDeviceStart", api_paths::LOGIN_DEVICE_START),
        ("loginDevicePoll", api_paths::LOGIN_DEVICE_POLL),
        ("loginWebStart", api_paths::LOGIN_WEB_START),
        ("loginWebExchange", api_paths::LOGIN_WEB_EXCHANGE),
        ("me", api_paths::ME),
        ("meToken", api_paths::ME_TOKEN),
        ("collections", collection_paths::COLLECTIONS),
        ("openCollections", collection_paths::OPEN),
        ("collection", collection_paths::COLLECTION),
        ("collectionEntries", collection_paths::ENTRIES),
        ("collectionEntry", collection_paths::ENTRY),
        ("collectionBlock", collection_paths::BLOCK),
        ("projects", api_paths::SITES),
        ("project", api_paths::SITE),
        ("projectUploads", api_paths::SITE_UPLOADS),
        ("projectUploadCommit", api_paths::SITE_UPLOAD_COMMIT),
        ("blob", api_paths::BLOB),
        ("projectTunnel", api_paths::SITE_TUNNEL),
        ("projectVersions", api_paths::SITE_VERSIONS),
        ("projectVersionActivate", api_paths::SITE_VERSION_ACTIVATE),
        ("projectVersionFiles", api_paths::SITE_VERSION_FILES),
        ("projectResults", result_paths::SITE_RESULTS),
        (
            "projectVersionSessions",
            result_paths::SITE_VERSION_SESSIONS,
        ),
        ("projectFeedback", result_paths::SITE_FEEDBACK),
        ("projectFeedbackItem", result_paths::SITE_FEEDBACK_ITEM),
        ("adminBoosts", admin_paths::BOOSTS),
        ("adminBoost", admin_paths::BOOST),
        ("adminBoostReview", admin_paths::BOOST_REVIEW),
        ("adminPlazaHide", admin_paths::PLAZA_HIDE),
        ("adminNotifications", admin_paths::NOTIFICATIONS),
        ("adminJobs", admin_paths::JOBS),
    ]
}

/// 所有跨进程的类型，按名字收成一张 `$defs` 表。
///
/// 列表型响应（`Vec<Site>` 等）在这里起一个名字，OpenAPI 与 TS 才有东西可引用。
pub fn definitions() -> BTreeMap<String, Value> {
    let mut settings = SchemaSettings::draft2020_12();
    settings.inline_subschemas = false;
    let mut generator = SchemaGenerator::new(settings);

    fn add<T: JsonSchema>(g: &mut SchemaGenerator) {
        g.subschema_for::<T>();
    }
    use crate::{api, boost, capabilities, follow, ingest, plan, plaza, project, results, tunnel};
    add::<api::ErrorBody>(&mut generator);
    add::<api::AnonSessionResponse>(&mut generator);
    add::<api::DeviceLoginStart>(&mut generator);
    add::<api::DeviceLoginPoll>(&mut generator);
    add::<api::LoginPollResponse>(&mut generator);
    add::<api::LoginResponse>(&mut generator);
    add::<api::WebLoginExchange>(&mut generator);
    add::<api::Me>(&mut generator);
    add::<api::CreateSiteRequest>(&mut generator);
    add::<api::Site>(&mut generator);
    add::<api::UpdateSiteRequest>(&mut generator);
    add::<api::VersionList>(&mut generator);
    add::<api::VersionFiles>(&mut generator);
    add::<api::PrepareUploadRequest>(&mut generator);
    add::<api::PrepareUploadResponse>(&mut generator);
    add::<api::CommitUploadResponse>(&mut generator);
    add::<tunnel::TunnelRequest>(&mut generator);
    add::<tunnel::TunnelGrant>(&mut generator);
    add::<results::SiteResults>(&mut generator);
    add::<results::VersionSessions>(&mut generator);
    add::<results::FeedbackList>(&mut generator);
    add::<results::UpdateFeedbackRequest>(&mut generator);
    add::<ingest::EventBatch>(&mut generator);
    add::<ingest::Accepted>(&mut generator);
    add::<ingest::FeedbackRequest>(&mut generator);
    add::<ingest::FeedbackAccepted>(&mut generator);
    add::<follow::FollowRequest>(&mut generator);
    add::<follow::FollowResponse>(&mut generator);
    add::<follow::ConfirmRequest>(&mut generator);
    add::<follow::ConfirmResponse>(&mut generator);
    add::<follow::UnsubscribeRequest>(&mut generator);
    add::<follow::MeRequest>(&mut generator);
    add::<follow::UnfollowRequest>(&mut generator);
    add::<follow::SendLinkRequest>(&mut generator);
    add::<follow::MeView>(&mut generator);
    add::<boost::GrantBoostRequest>(&mut generator);
    add::<boost::ReviewBoostRequest>(&mut generator);
    add::<boost::NotificationQueue>(&mut generator);
    add::<boost::JobStatus>(&mut generator);
    add::<project::Project>(&mut generator);
    add::<project::ProjectCard>(&mut generator);
    add::<project::ProjectLive>(&mut generator);
    add::<plaza::Plaza>(&mut generator);
    add::<crate::collection::CollectionDraft>(&mut generator);
    add::<crate::collection::EntryDraft>(&mut generator);
    add::<crate::collection::ModerateCollectionRequest>(&mut generator);
    add::<capabilities::Capabilities>(&mut generator);
    add::<plan::Limits>(&mut generator);

    let mut defs: BTreeMap<String, Value> = generator.take_definitions(true).into_iter().collect();
    for (name, of) in [
        ("SiteList", "Site"),
        ("CollectionList", "Collection"),
        ("BoostList", "Boost"),
        ("JobList", "JobStatus"),
    ] {
        defs.insert(
            name.to_string(),
            json!({ "type": "array", "items": { "$ref": format!("#/$defs/{of}") } }),
        );
    }
    defs
}

/// OpenAPI 3.1 的 `components.schemas`：同一张表，`$ref` 改指 `#/components/schemas/…`。
pub fn openapi_schemas() -> Value {
    let mut out = Map::new();
    for (name, schema) in definitions() {
        out.insert(name, rewrite_refs(schema));
    }
    Value::Object(out)
}

fn rewrite_refs(value: Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(k, v)| {
                    if k == "$ref" {
                        if let Value::String(s) = &v {
                            return (
                                k,
                                Value::String(s.replace("#/$defs/", "#/components/schemas/")),
                            );
                        }
                    }
                    (k, rewrite_refs(v))
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.into_iter().map(rewrite_refs).collect()),
        other => other,
    }
}

/// 一个端点在 OpenAPI 里的 operation 对象。
pub fn openapi_operation(endpoint: &Endpoint) -> Value {
    let mut op = Map::new();
    op.insert("summary".into(), json!(endpoint.summary));
    let params: Vec<Value> = path_params(endpoint.path)
        .into_iter()
        .map(|name| {
            json!({
                "name": name,
                "in": "path",
                "required": true,
                "schema": { "type": if name == "version" || name == "id" { "integer" } else { "string" } },
            })
        })
        .collect();
    if !params.is_empty() {
        op.insert("parameters".into(), Value::Array(params));
    }
    if let Some(request) = endpoint.request {
        op.insert(
            "requestBody".into(),
            json!({
                "required": true,
                "content": { "application/json": { "schema": { "$ref": format!("#/components/schemas/{request}") } } },
            }),
        );
    } else if endpoint.method == "PUT" {
        op.insert(
            "requestBody".into(),
            json!({
                "required": true,
                "content": { "application/octet-stream": { "schema": { "type": "string", "format": "binary" } } },
            }),
        );
    }
    let mut responses = Map::new();
    match endpoint.response {
        Some(response) => {
            responses.insert(
                "200".into(),
                json!({
                    "description": endpoint.summary,
                    "content": { "application/json": { "schema": { "$ref": format!("#/components/schemas/{response}") } } },
                }),
            );
        }
        None => {
            responses.insert(
                if endpoint.method == "GET" {
                    "200"
                } else {
                    "204"
                }
                .into(),
                json!({ "description": endpoint.summary }),
            );
        }
    }
    responses.insert(
        "default".into(),
        json!({
            "description": "an error: `code` is for programs, `message` for people",
            "content": { "application/json": { "schema": { "$ref": "#/components/schemas/ErrorBody" } } },
        }),
    );
    op.insert("responses".into(), Value::Object(responses));
    op.insert(
        "security".into(),
        match endpoint.auth {
            Auth::Bearer => json!([{ "bearer": [] }]),
            Auth::Admin => json!([{ "admin": [] }]),
            Auth::Public | Auth::Ingest => json!([]),
        },
    );
    op.into()
}

/// `{slug}`、`{version}` 这样的占位符名字，按出现顺序。
pub fn path_params(template: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        let after = &rest[start + 1..];
        let Some(end) = after.find('}') else { break };
        out.push(&after[..end]);
        rest = &after[end + 1..];
    }
    out
}

// ---------------------------------------------------------------- TypeScript

/// `console/src/generated/api.ts` 的全文。
pub fn typescript() -> String {
    let mut out = String::new();
    out.push_str(
        "// 由 `cargo run -p playtest-common --bin playtest-contract` 生成，不要手改。\n\
         // 来源是 common/src/*.rs 上的类型与路由常量；那边改了这里跟着重新生成，\n\
         // `playtest-common` 的测试会在两边不一致时变红。\n\
         // 两个进程不会同时升级，所以带 `?` 的字段在旧控制面上可能不存在。\n\n",
    );

    for (name, schema) in definitions() {
        if let Some(desc) = schema.get("description").and_then(Value::as_str) {
            out.push_str(&doc(desc, ""));
        }
        out.push_str(&format!(
            "export type {name} = {};\n\n",
            ts_type(&schema, "")
        ));
    }

    out.push_str(
        "/** 控制面的路径。有参数的是函数，参数会做 URL 编码。 */\nexport const paths = {\n",
    );
    for (name, template) in paths() {
        let params = path_params(template);
        if params.is_empty() {
            out.push_str(&format!("  {name}: \"{template}\",\n"));
        } else {
            let args = params
                .iter()
                .map(|p| format!("{p}: string | number"))
                .collect::<Vec<_>>()
                .join(", ");
            let mut body = template.to_string();
            for p in &params {
                body = body.replace(
                    &format!("{{{p}}}"),
                    &format!("${{encodeURIComponent(String({p}))}}"),
                );
            }
            out.push_str(&format!("  {name}: ({args}) => `{body}`,\n"));
        }
    }
    out.push_str("} as const;\n");
    out
}

fn doc(text: &str, indent: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() == 1 {
        format!("{indent}/** {} */\n", lines[0].replace("*/", "*\\/"))
    } else {
        let mut s = format!("{indent}/**\n");
        for line in lines {
            s.push_str(&format!("{indent} * {}\n", line.replace("*/", "*\\/")));
        }
        s.push_str(&format!("{indent} */\n"));
        s
    }
}

fn ts_type(schema: &Value, indent: &str) -> String {
    let Some(map) = schema.as_object() else {
        // `true` 或别的什么：什么都行。
        return "unknown".into();
    };
    if map.is_empty() {
        return "unknown".into();
    }

    let mut parts: Vec<String> = Vec::new();

    if let Some(Value::String(r)) = map.get("$ref") {
        parts.push(r.rsplit('/').next().unwrap_or(r).to_string());
    }

    if let Some(one) = map.get("oneOf").or_else(|| map.get("anyOf")) {
        let variants: Vec<String> = one
            .as_array()
            .map(|items| items.iter().map(|v| ts_type(v, indent)).collect())
            .unwrap_or_default();
        parts.push(variants.join(" | "));
    }

    if let Some(Value::Array(all)) = map.get("allOf") {
        parts.extend(all.iter().map(|v| ts_type(v, indent)));
    }

    if let Some(c) = map.get("const") {
        parts.push(serde_json::to_string(c).unwrap_or_default());
    } else if let Some(Value::Array(values)) = map.get("enum") {
        parts.push(
            values
                .iter()
                .map(|v| serde_json::to_string(v).unwrap_or_default())
                .collect::<Vec<_>>()
                .join(" | "),
        );
    } else if let Some(t) = map.get("type") {
        let types: Vec<&str> = match t {
            Value::String(s) => vec![s.as_str()],
            Value::Array(items) => items.iter().filter_map(Value::as_str).collect(),
            _ => vec![],
        };
        let mut names: Vec<String> = Vec::new();
        for ty in types {
            names.push(match ty {
                "object" => ts_object(map, indent),
                "array" => {
                    let items = map.get("items").map(|i| ts_type(i, indent));
                    match items {
                        Some(inner) if inner.contains(' ') => format!("Array<{inner}>"),
                        Some(inner) => format!("{inner}[]"),
                        None => "unknown[]".into(),
                    }
                }
                "string" => "string".into(),
                "integer" | "number" => "number".into(),
                "boolean" => "boolean".into(),
                "null" => "null".into(),
                other => {
                    let _ = other;
                    "unknown".into()
                }
            });
        }
        if !names.is_empty() {
            parts.push(names.join(" | "));
        }
    } else if map.contains_key("properties") {
        parts.push(ts_object(map, indent));
    }

    if parts.is_empty() {
        "unknown".into()
    } else if parts.len() == 1 {
        parts.pop().unwrap()
    } else {
        parts
            .into_iter()
            .map(|p| {
                if p.contains(" | ") {
                    format!("({p})")
                } else {
                    p
                }
            })
            .collect::<Vec<_>>()
            .join(" & ")
    }
}

fn ts_object(map: &Map<String, Value>, indent: &str) -> String {
    let Some(Value::Object(props)) = map.get("properties") else {
        // 没有属性：`HashMap` 之类。
        return match map.get("additionalProperties") {
            Some(extra) if !extra.is_boolean() => {
                format!("Record<string, {}>", ts_type(extra, indent))
            }
            _ => "Record<string, unknown>".into(),
        };
    };
    let required: Vec<&str> = map
        .get("required")
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    let inner = format!("{indent}  ");
    let mut out = String::from("{\n");
    for (name, prop) in props {
        if let Some(desc) = prop.get("description").and_then(Value::as_str) {
            out.push_str(&doc(desc, &inner));
        }
        // Rust 的 `Option` 字段没有值时整项不出现（`skip_serializing_if`），所以是 `?:` 而不是
        // `| null`；只有标了 `#[schemars(required)]` 的那几个会真的送 `null`（「不知道」不是 0）。
        let is_required = required.contains(&name.as_str());
        let mut ty = ts_type(prop, &inner);
        if !is_required {
            ty = without_null(&ty);
        }
        let optional = if is_required { "" } else { "?" };
        out.push_str(&format!("{inner}{name}{optional}: {ty};\n"));
    }
    out.push_str(&format!("{indent}}}"));
    out
}

/// 把顶层联合里的 `null` 去掉：`string | null` → `string`。
fn without_null(ty: &str) -> String {
    if !ty.contains("null") {
        return ty.to_string();
    }
    let mut depth = 0i32;
    let mut parts: Vec<String> = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = ty.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '{' | '(' | '[' | '<' => depth += 1,
            '}' | ')' | ']' | '>' => depth -= 1,
            _ => {}
        }
        if depth == 0 && c == '|' && i > 0 && chars[i - 1] == ' ' {
            current.pop();
            parts.push(current.trim().to_string());
            current.clear();
            i += 2;
            continue;
        }
        current.push(c);
        i += 1;
    }
    parts.push(current.trim().to_string());
    let kept: Vec<String> = parts.into_iter().filter(|p| p != "null").collect();
    if kept.is_empty() {
        "null".into()
    } else {
        kept.join(" | ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_schema_an_endpoint_names_exists() {
        let defs = definitions();
        for e in endpoints() {
            for name in [e.request, e.response].into_iter().flatten() {
                assert!(
                    defs.contains_key(name),
                    "{} {} 引用了不存在的类型 {name}",
                    e.method,
                    e.path
                );
            }
        }
    }

    #[test]
    fn null_is_dropped_only_at_the_top_level() {
        assert_eq!(without_null("string | null"), "string");
        assert_eq!(without_null("Boost | null"), "Boost");
        assert_eq!(without_null("Array<string | null>"), "Array<string | null>");
        assert_eq!(
            without_null("{\n  a: string | null;\n} | null"),
            "{\n  a: string | null;\n}"
        );
    }

    #[test]
    fn path_params_are_read_in_order() {
        assert_eq!(
            path_params("/v1/projects/{slug}/versions/{version}/files"),
            vec!["slug", "version"]
        );
        assert!(path_params("/v1/me").is_empty());
    }

    #[test]
    fn typescript_has_the_shapes_the_console_uses() {
        let ts = typescript();
        assert!(ts.contains("export type Site = {"));
        assert!(ts.contains("current_version?: number;"));
        assert!(ts.contains("dropped_before_first_frame: number | null;"));
        assert!(ts.contains("boost?: Boost;"));
        assert!(ts.contains("export type BoostKind = \"days3\" | \"days7\" | \"digest\";"));
        assert!(ts.contains("export type SiteList = Site[];"));
        assert!(ts.contains("project: (slug: string | number) => `/v1/projects/${encodeURIComponent(String(slug))}`"));
        // 带标签的枚举：标签对象与体的交集。
        assert!(ts.contains("export type LoginPollResponse = "));
    }

    #[test]
    fn openapi_refs_point_at_components() {
        let schemas = openapi_schemas();
        let text = serde_json::to_string(&schemas).unwrap();
        assert!(!text.contains("#/$defs/"), "还有 $defs 引用没改");
        assert!(text.contains("#/components/schemas/Listing"));
        let op = openapi_operation(&endpoints()[0]);
        assert_eq!(op["summary"], "is this control plane up");
    }

    /// 生成物和仓库里的那份必须一致；不一致就跑一次生成器再提交。
    #[test]
    fn generated_typescript_is_current() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../console/src/generated/api.ts"
        );
        let on_disk = std::fs::read_to_string(path).unwrap_or_default();
        assert!(
            on_disk == typescript(),
            "console/src/generated/api.ts 过时了：跑 `cargo run -p playtest-common --bin playtest-contract` 再提交"
        );
    }
}
