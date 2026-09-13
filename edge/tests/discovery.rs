use playtest_common::{capabilities::Capabilities, collection::Collection, plaza::Plaza};
use playtest_edge::{
    discovery::{self, Query},
    plaza::View,
};
use serde_json::json;
use time::macros::datetime;

fn fixture() -> Plaza {
    serde_json::from_value(json!({"schema":1,"generated_at":"2026-09-12T00:00:00Z","items":[
        {"slug":"first-pelican","url":"https://first-pelican.playtest.run","title":"红色鹈鹕","developer":"小雨","summary":"骑单车","version":2,"updated_at":"2026-09-12T00:00:00Z","players":1,"is_game":false},
        {"slug":"second-pelican","url":"https://second-pelican.playtest.run","title":"蓝色鹈鹕","developer":"阿木","version":1,"updated_at":"2026-09-11T00:00:00Z","players":20,"is_game":false}
    ],"collections":[{"slug":"pelican","title":"鹈鹕骑单车","summary":"一起创作","kind":"challenge","prompt":"<script>unsafe()</script>","rules":"只提交自己的作品","closes_at":null,"public":true,"hidden":false,"creator":"组织者","created_at":"2026-09-11T00:00:00Z","updated_at":"2026-09-11T00:00:00Z","entries":[
        {"slug":"first-pelican","title":"红色鹈鹕","submitted_version":1,"submitted_at":"2026-09-12T00:00:00Z","note":"<script>unsafe()</script>"},
        {"slug":"second-pelican","title":"蓝色鹈鹕","submitted_version":1,"submitted_at":"2026-09-11T00:00:00Z","note":""}
    ]}]})).unwrap()
}

#[test]
fn search_is_shareable_escaped_and_available_without_scripts() {
    let plaza = fixture();
    let view = View {
        plaza: &plaza,
        now: datetime!(2026-09-12 12:00 UTC),
    };
    let query = Query::parse("q=%E5%B0%8F%E9%9B%A8&sort=latest&page=0");
    assert_eq!(query.search, "小雨");
    assert_eq!(query.page, 1);
    let html = discovery::home(&view, &query, false);
    assert!(html.contains("红色鹈鹕"));
    assert!(!html.contains("蓝色鹈鹕"));
    assert!(html.contains("method=\"get\""));
    assert!(!html.contains("<script"));
    let html = discovery::home(
        &view,
        &Query::parse("q=%22%3E%3Cscript%3Ebad%3C/script%3E"),
        false,
    );
    assert!(!html.contains("<script>bad"));
    assert!(html.contains("清空搜索"));
}

#[test]
fn popularity_is_separate_and_does_not_buy_search_rank() {
    let mut plaza = fixture();
    plaza.items[0].boosted = true;
    let view = View {
        plaza: &plaza,
        now: datetime!(2026-09-12 12:00 UTC),
    };
    let html = discovery::home(&view, &Query::parse("sort=hot"), false);
    assert!(
        html.find("data-slug=\"second-pelican\"").unwrap()
            < html.find("data-slug=\"first-pelican\"").unwrap()
    );
    assert!(!html.contains("class=\"tag ad\""));
    assert!(html.contains("会话数不等于真人数"));
}

#[test]
fn collection_explains_provenance_and_keeps_player_paths_on_player_domain() {
    let plaza = fixture();
    let view = View {
        plaza: &plaza,
        now: datetime!(2026-09-12 12:00 UTC),
    };
    let html = discovery::collection(
        &view,
        &plaza.collections[0],
        &Query::parse("q=红色"),
        &Capabilities::default(),
        None,
    );
    assert!(html.contains("红色鹈鹕"));
    assert!(!html.contains("<h2>蓝色鹈鹕</h2>"));
    assert!(html.contains("投稿 v1 · 当前 v2"));
    assert!(!html.contains("<script>unsafe()"));
    assert!(html.contains("&lt;script&gt;unsafe()"));
    assert!(!html.contains("playtest.roviix.com"));
    assert!(!html.contains("<iframe"));
    assert!(html.contains("?collection=pelican&amp;from=collection"));
    let context = discovery::context(&plaza, "first-pelican", Some("pelican"));
    assert!(context.contains("/c/pelican"));
    assert!(context.contains("返回"));
    assert!(discovery::context(&plaza, "first-pelican", Some("missing")).is_empty());
}

#[test]
fn expired_entries_disappear_from_cards_filters_and_context() {
    let mut plaza = fixture();
    plaza.items[0].expires_at = Some("2020-01-01T00:00:00Z".into());
    let view = View {
        plaza: &plaza,
        now: datetime!(2026-09-12 12:00 UTC),
    };
    let html = discovery::collection(
        &view,
        &plaza.collections[0],
        &Query::default(),
        &Capabilities::default(),
        None,
    );
    assert!(!html.contains("Model A"));
    assert!(!html.contains("红色鹈鹕"));
    assert!(discovery::context(&plaza, "first-pelican", Some("pelican")).is_empty());
    let mut closed: Collection = plaza.collections[0].clone();
    closed.closes_at = Some("2020-01-01T00:00:00Z".into());
    let html = discovery::collection(
        &view,
        &closed,
        &Query::default(),
        &Capabilities::default(),
        None,
    );
    assert!(html.contains("已结束"));
    assert!(!html.contains("我也来做一个"));
}

#[test]
fn pages_are_bounded_and_queries_survive_navigation() {
    let mut plaza = fixture();
    for number in 0..28 {
        let mut item = plaza.items[0].clone();
        item.slug = format!("item-{number}");
        plaza.items.push(item);
    }
    let view = View {
        plaza: &plaza,
        now: datetime!(2026-09-12 12:00 UTC),
    };
    let html = discovery::home(
        &view,
        &Query::parse("q=%E9%B9%88%E9%B9%95&sort=hot&page=99999"),
        false,
    );
    assert_eq!(html.matches("class=\"tile\"").count(), 6);
    assert!(html.contains("第 2 / 2 页"));
    assert!(html.contains("sort=hot&amp;page=1"));
}

#[test]
fn latest_does_not_put_older_recruiting_work_ahead_of_new_work() {
    let mut plaza = fixture();
    plaza.items[1].seeking = true;
    plaza.items.reverse();
    let view = View {
        plaza: &plaza,
        now: datetime!(2026-09-12 12:00 UTC),
    };
    let html = discovery::home(&view, &Query::default(), false);
    assert!(
        html.find("data-slug=\"first-pelican\"").unwrap()
            < html.find("data-slug=\"second-pelican\"").unwrap()
    );
}
