//! Synthetic optional evidence: no production conversion or clinical scoring.
use rusqlite::Connection;
use serde_json::{Value, json};
use skarma_api::store;
use skarma_core::{
    auth::{AccountRole, User},
    foundation::observations::NEEDS_ID,
};
use uuid::Uuid;
fn id() -> String {
    Uuid::new_v4().to_string()
}
fn fixture() -> (Connection, User, String) {
    let mut db = Connection::open_in_memory().unwrap();
    store::initialize_foundation(&db).unwrap();
    let u = User {
        id: id(),
        username: "test".into(),
        display_name: "测试家人".into(),
        role: AccountRole::Member,
        active: true,
    };
    let goal = id();
    store::write_foundation(&mut db,&u,serde_json::from_value(json!({"id":goal,"operation_id":id(),"expected_version":null,"change_note":"","mutation":{"kind":"goal","value":{"title":"测试轮替","level":"short"}}})).unwrap()).unwrap();
    (db, u, goal)
}
fn call(db: &mut Connection, u: &User, request: Value) -> store::Result<Value> {
    store::foundation_process(
        db,
        u,
        serde_json::from_value(json!({"kind":"observe","value":request})).unwrap(),
    )
}
fn write(kind: &str, identifier: &str, version: Option<u64>, input: Value) -> Value {
    json!({"id":identifier,"operation_id":id(),"kind":kind,"action":"save","expected_version":version,"input":input,"note":if version.is_some(){"测试修订"}else{""}})
}
fn read(db: &mut Connection, u: &User, kind: &str, goal: &str) -> Value {
    store::foundation_process(
        db,
        u,
        serde_json::from_value(
            json!({"kind":"read","value":{"view":"observations","object_kind":kind,"id":goal}}),
        )
        .unwrap(),
    )
    .unwrap()
}
fn metric(goal: &str) -> Value {
    json!({"goal_id":goal,"goal_version":1,"name":"轮替机会","template":"count","criterion":"5次机会","support":"提醒一次","unit":""})
}
#[test]
fn opportunities_are_not_progress_and_metric_versions_are_immutable() {
    let (mut db, u, g) = fixture();
    let m = id();
    let r = write("metric", &m, None, metric(&g));
    let original = call(&mut db, &u, r.clone()).unwrap();
    assert_eq!(original, call(&mut db, &u, r).unwrap());
    let record = id();
    let input = json!({"metric_id":m,"metric_version":1,"business_date":"2020-01-01","opportunities":0,"achieved":0,"support":"提醒一次","text":"没有合适机会"});
    let saved = call(
        &mut db,
        &u,
        write("measurement", &record, None, input.clone()),
    )
    .unwrap();
    assert_eq!(saved["metric"]["version"], 1);
    assert_eq!(saved["input"]["opportunities"], 0);
    let mut invalid = input.clone();
    invalid["achieved"] = json!(1);
    assert!(call(&mut db, &u, write("measurement", &id(), None, invalid)).is_err());
    let mut changed = metric(&g);
    changed["support"] = json!("不提醒");
    call(&mut db, &u, write("metric", &m, Some(1), changed)).unwrap();
    let measurements = read(&mut db, &u, "measurement", &g);
    assert_eq!(
        measurements["items"][0]["metric"]["input"]["support"],
        "提醒一次"
    );
    let mut revision = input;
    revision["opportunities"] = json!(5);
    revision["achieved"] = json!(3);
    call(
        &mut db,
        &u,
        write("measurement", &record, Some(1), revision),
    )
    .unwrap();
    assert_eq!(read(&mut db, &u, "measurement", &g)["total"], 1);
    let stale = write("measurement", &record, Some(1), saved["input"].clone());
    assert_eq!(
        call(&mut db, &u, stale).unwrap_err().0,
        axum::http::StatusCode::CONFLICT
    );
}
#[test]
fn moments_link_to_review_evidence_without_becoming_training_and_sources_do_not_duplicate() {
    let (mut db, u, g) = fixture();
    let mid = id();
    let before = store::foundation_coverage_at(
        &mut db,
        skarma_core::foundation::coverage::Request {
            days: 30,
            source: "family".into(),
        },
        "2020-01-02",
    )
    .unwrap();
    let moment = call(
        &mut db,
        &u,
        write(
            "moment",
            &mid,
            None,
            json!({"business_date":"2020-01-01","text":"主动邀请一起玩","goal_ids":[g]}),
        ),
    )
    .unwrap();
    let after = store::foundation_coverage_at(
        &mut db,
        skarma_core::foundation::coverage::Request {
            days: 30,
            source: "family".into(),
        },
        "2020-01-02",
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(before).unwrap(),
        serde_json::to_value(after).unwrap()
    );
    let m = id();
    call(&mut db, &u, write("metric", &m, None, metric(&g))).unwrap();
    let source = json!({"kind":"moment","id":mid,"version":1});
    let measurement = json!({"metric_id":m,"metric_version":1,"business_date":"2020-01-01","opportunities":5,"achieved":3,"support":"提醒一次","source":source});
    let measured = call(
        &mut db,
        &u,
        write("measurement", &id(), None, measurement.clone()),
    )
    .unwrap();
    assert_eq!(measured["source_snapshot"]["input"], moment["input"]);
    assert_eq!(
        call(
            &mut db,
            &u,
            write("measurement", &id(), None, measurement.clone())
        )
        .unwrap_err()
        .0,
        axum::http::StatusCode::CONFLICT
    );
    let mut wrong_day = measurement;
    wrong_day["business_date"] = json!("2020-01-02");
    assert!(call(&mut db, &u, write("measurement", &id(), None, wrong_day)).is_err());
    let context = store::foundation_process(
        &mut db,
        &u,
        serde_json::from_value(
            json!({"kind":"read","value":{"view":"review_context","object_kind":"goal","id":g}}),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(context["family"], 0);
    assert_eq!(context["total"], 2);
    assert!(
        context["evidence"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["source"]["kind"] == "moment")
    );
}
#[test]
fn shared_needs_and_observations_use_versions_authorship_and_admin_deletion() {
    let (mut db, u, g) = fixture();
    let needs = write(
        "needs",
        NEEDS_ID,
        None,
        json!({"communication":"用手势表达休息","calming":"安静陪伴"}),
    );
    let saved = call(&mut db, &u, needs.clone()).unwrap();
    assert_eq!(saved, call(&mut db, &u, needs).unwrap());
    let mut other = u.clone();
    other.id = id();
    other.display_name = "另一位家人".into();
    let edited = call(
        &mut db,
        &other,
        write(
            "needs",
            NEEDS_ID,
            Some(1),
            json!({"communication":"手势或词语"}),
        ),
    )
    .unwrap();
    assert_eq!(edited["audit"]["actor_name"], "另一位家人");
    assert_eq!(
        call(&mut db, &u, write("needs", NEEDS_ID, Some(1), json!({})))
            .unwrap_err()
            .0,
        axum::http::StatusCode::CONFLICT
    );
    let m = id();
    call(
        &mut db,
        &u,
        write(
            "moment",
            &m,
            None,
            json!({"business_date":"2020-01-01","text":"测试瞬间","goal_ids":[g]}),
        ),
    )
    .unwrap();
    let mut delete = write("moment", &m, Some(1), Value::Null);
    delete["action"] = json!("delete");
    assert_eq!(
        call(&mut db, &u, delete.clone()).unwrap_err().0,
        axum::http::StatusCode::FORBIDDEN
    );
    let mut admin = u.clone();
    admin.role = AccountRole::Admin;
    let deleted = call(&mut db, &admin, delete).unwrap();
    assert_eq!(deleted["deleted"], true);
    assert_eq!(read(&mut db, &u, "moment", &g)["items"][0]["deleted"], true);
    let mut inactive = u;
    inactive.active = false;
    assert!(
        call(
            &mut db,
            &inactive,
            write(
                "moment",
                &id(),
                None,
                json!({"business_date":"2020-01-01","text":"拒绝停用用户"})
            )
        )
        .is_err()
    );
}

#[test]
fn deleting_a_source_marks_current_measurement_but_preserves_its_original_snapshot() {
    let (mut db, u, g) = fixture();
    let mid = id();
    let m = id();
    call(
        &mut db,
        &u,
        write(
            "moment",
            &mid,
            None,
            json!({"business_date":"2020-01-01","text":"保留当时的观察","goal_ids":[g]}),
        ),
    )
    .unwrap();
    call(&mut db, &u, write("metric", &m, None, metric(&g))).unwrap();
    let record = id();
    call(&mut db,&u,write("measurement",&record,None,json!({"metric_id":m,"metric_version":1,"business_date":"2020-01-01","opportunities":5,"achieved":3,"source":{"kind":"moment","id":mid,"version":1}}))).unwrap();
    let before: String = db
        .query_row(
            "SELECT data FROM prd_observations WHERE id=?1",
            [&record],
            |r| r.get(0),
        )
        .unwrap();
    let mut admin = u.clone();
    admin.role = AccountRole::Admin;
    let mut delete = write("moment", &mid, Some(1), Value::Null);
    delete["action"] = json!("delete");
    call(&mut db, &admin, delete).unwrap();
    let current = read(&mut db, &u, "measurement", &g);
    assert_eq!(current["items"][0]["source_deleted"], true);
    assert_eq!(
        current["items"][0]["source_snapshot"]["input"]["text"],
        "保留当时的观察"
    );
    let after: String = db
        .query_row(
            "SELECT data FROM prd_observations WHERE id=?1",
            [&record],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(before, after);
}
