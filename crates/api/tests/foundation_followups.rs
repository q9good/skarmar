//! Synthetic-only event, calendar, revision and transaction tests.
use rusqlite::Connection;
use serde_json::{Value, json};
use skarma_api::store;
use skarma_core::{
    auth::{AccountRole, User},
    foundation::{process_model::*, reviews},
};
use uuid::Uuid;
fn id() -> String {
    Uuid::new_v4().to_string()
}
fn user() -> User {
    User {
        id: id(),
        username: "synthetic".into(),
        display_name: "测试家人".into(),
        role: AccountRole::Member,
        active: true,
    }
}
fn call(db: &mut Connection, u: &User, kind: &str, input: Value) -> store::Result<Value> {
    store::foundation_process(
        db,
        u,
        serde_json::from_value(json!({"kind":kind,"value":input})).unwrap(),
    )
}
fn read(db: &mut Connection, u: &User, view: &str, kid: &str) -> Value {
    call(db, u, "read", json!({"view":view,"id":kid})).unwrap()
}
fn fixture() -> (Connection, User, String) {
    let mut db = Connection::open_in_memory().unwrap();
    store::initialize_foundation(&db).unwrap();
    let u = user();
    let g = id();
    store::write_foundation(&mut db,&u,serde_json::from_value(json!({"id":g,"operation_id":id(),"expected_version":null,"change_note":"","mutation":{"kind":"goal","value":{"title":"测试近期本领","level":"short"}}})).unwrap()).unwrap();
    (db, u, g)
}
fn knowledge(db: &mut Connection, u: &User, g: &str, kind: &str) -> Value {
    let kid = id();
    call(db,u,"knowledge",json!({"id":kid,"operation_id":id(),"expected_version":null,"action":"create","input":{"id":kid,"kind":kind,"content":"测试做法或困难","goal_ids":[g],"stage":if kind=="experience"{json!("try")}else{Value::Null}},"details":{"title":format!("测试{kind}"),"basis":"5次机会","support":"示范一次","comparison_start":"2020-01-01"}})).unwrap()
}
fn feedback(kid: &str, result: &str) -> Value {
    let eid = id();
    json!({"id":eid,"operation_id":id(),"expected_version":null,"action":"record","input":{"knowledge_id":kid,"source":{"kind":"independent","id":eid,"version":1},"business_date":"2020-01-01","result":result,"observation":"测试实际观察","basis":"5次机会","support":"示范一次","comparable":false}})
}
fn review_input(db: &mut Connection, u: &User, g: &str) -> Value {
    let c = call(
        db,
        u,
        "read",
        json!({"view":"review_context","id":g,"object_kind":"goal"}),
    )
    .unwrap();
    json!({"id":id(),"operation_id":id(),"expected_version":null,"object_kind":"goal","object_id":g,"object_version":c["object_version"],"schedule_version":c["schedule"]["version"],"generation":c["schedule"]["generation"],"due_date":c["schedule"]["due_date"],"action":"complete","decision":"continue"})
}
#[test]
fn calendar_uses_phase_and_real_months_not_thirty_days() {
    let s = Settings {
        version: 1,
        weekday: 6,
        fortnight_anchor: "2026-10-10".into(),
        monthly_ordinal: 1,
        audit: Audit {
            actor_id: "".into(),
            actor_name: "".into(),
            at: "".into(),
            note: "".into(),
        },
    };
    assert_eq!(
        reviews::next_date(&s, "fortnightly", "2026-11-14", false).unwrap(),
        "2026-11-21"
    );
    assert_eq!(
        reviews::next_date(&s, "monthly", "2026-10-24", false).unwrap(),
        "2026-11-07"
    );
    let s = Settings {
        weekday: 7,
        monthly_ordinal: -1,
        ..s
    };
    assert_eq!(
        reviews::next_date(&s, "monthly", "2024-02-01", false).unwrap(),
        "2024-02-25"
    );
    assert_eq!(
        reviews::next_date(&s, "monthly", "2024-02-25", false).unwrap(),
        "2024-03-31"
    );
}
#[test]
fn annotations_never_change_main_result_and_revision_moves_only_one_usage() {
    let (mut db, u, g) = fixture();
    let k = knowledge(&mut db, &u, &g, "experience");
    let kid = k["input"]["id"].as_str().unwrap();
    let original = feedback(kid, "helpful");
    let first = call(&mut db, &u, "feedback", original.clone()).unwrap();
    call(&mut db, &u, "feedback", original.clone()).unwrap();
    let eid = first["id"].as_str().unwrap();
    let other = user();
    call(&mut db,&other,"feedback",json!({"id":eid,"operation_id":id(),"expected_version":1,"action":"annotate","note":"测试：另一种条件下没帮上"})).unwrap();
    let before = read(&mut db, &u, "knowledge", kid);
    assert_eq!(before["summary"]["counts"]["helpful"], 1);
    assert_eq!(before["summary"]["total"], 1);
    let mut duplicate = original.clone();
    duplicate["id"] = json!(id());
    duplicate["operation_id"] = json!(id());
    assert!(call(&mut db, &u, "feedback", duplicate).is_err());
    let mut revise = original;
    revise["operation_id"] = json!(id());
    revise["expected_version"] = json!(1);
    revise["action"] = json!("revise");
    revise["input"]["result"] = json!("uncertain");
    assert!(call(&mut db, &other, "feedback", revise.clone()).is_err());
    revise["note"] = json!("测试：核对后改为不确定");
    call(&mut db, &other, "feedback", revise).unwrap();
    let after = read(&mut db, &u, "knowledge", kid);
    assert_eq!(after["summary"]["counts"]["uncertain"], 1);
    assert_eq!(after["summary"]["total"], 1);
    assert_eq!(
        read(&mut db, &u, "event_history", eid)["history"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        read(&mut db, &u, "events", kid)["annotations"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let remove = json!({"id":eid,"operation_id":id(),"expected_version":2,"action":"delete"});
    assert!(call(&mut db, &u, "feedback", remove.clone()).is_err());
    let admin = User {
        role: AccountRole::Admin,
        ..user()
    };
    call(&mut db, &admin, "feedback", remove).unwrap();
    assert_eq!(read(&mut db, &u, "knowledge", kid)["summary"]["total"], 0);
}
#[test]
fn three_comparable_records_only_offer_weekly_review_and_do_not_promote() {
    let (mut db, u, g) = fixture();
    let k = knowledge(&mut db, &u, &g, "difficulty");
    let kid = k["input"]["id"].as_str().unwrap();
    for i in 0..3 {
        let mut e = feedback(kid, "same");
        e["input"]["comparable"] = json!(true);
        call(&mut db, &u, "feedback", e).unwrap();
        assert_eq!(
            read(&mut db, &u, "knowledge", kid)["summary"]["offer_weekly"],
            json!(i == 2)
        );
    }
    let unseen = feedback(kid, "unobserved");
    call(&mut db, &u, "feedback", unseen).unwrap();
    let detail = read(&mut db, &u, "knowledge", kid);
    assert_eq!(detail["summary"]["comparable_count"], 3);
    assert_eq!(detail["knowledge"]["version"], 1);
    assert_eq!(detail["knowledge"]["details"]["priority"], false);
    let mut edit = json!({"id":kid,"operation_id":id(),"expected_version":1,"action":"edit","input":detail["knowledge"]["input"],"details":detail["knowledge"]["details"],"note":"测试：明确每周回看"});
    edit["details"]["priority"] = json!(true);
    call(&mut db, &u, "knowledge", edit).unwrap();
    let board = read(&mut db, &u, "board", "");
    assert!(
        board["schedules"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["object_id"] == kid && s["interval"] == "weekly")
    );
}
#[test]
fn review_and_outputs_are_atomic_once_and_defer_is_not_completion() {
    let (mut db, u, g) = fixture();
    let mut r = review_input(&mut db, &u, &g);
    r["decision"] = json!("maintain");
    r["note"] = json!("测试：已有观察，先维持");
    r["next_actions"] =
        json!([{"id":id(),"content":"测试复盘下一步","goal_ids":[g],"assignee_id":null}]);
    db.execute_batch("CREATE TRIGGER fail_review_action BEFORE INSERT ON prd_actions BEGIN SELECT RAISE(ABORT,'synthetic failure'); END").unwrap();
    assert!(call(&mut db, &u, "review", r.clone()).is_err());
    assert_eq!(
        db.query_row::<i64, _, _>("SELECT COUNT(*) FROM prd_reviews", [], |r| r.get(0))
            .unwrap(),
        0
    );
    db.execute_batch("DROP TRIGGER fail_review_action").unwrap();
    let done = call(&mut db, &u, "review", r.clone()).unwrap();
    call(&mut db, &u, "review", r.clone()).unwrap();
    let mut stale = r.clone();
    stale["id"] = json!(id());
    stale["operation_id"] = json!(id());
    assert!(call(&mut db, &u, "review", stale).is_err());
    assert_eq!(
        db.query_row::<i64, _, _>("SELECT COUNT(*) FROM prd_reviews", [], |r| r.get(0))
            .unwrap(),
        1
    );
    assert_eq!(
        store::foundation_snapshot(&mut db).unwrap().actions.len(),
        1
    );
    let mut postpone = review_input(&mut db, &u, &g);
    postpone["action"] = json!("defer");
    postpone["next_date"] = json!("2099-01-01");
    call(&mut db, &u, "review", postpone).unwrap();
    assert_eq!(
        db.query_row::<i64, _, _>("SELECT COUNT(*) FROM prd_reviews", [], |r| r.get(0))
            .unwrap(),
        1
    );
    let before = read(&mut db, &u, "board", "")["schedules"].clone();
    let mut revise = r;
    revise["expected_version"] = json!(1);
    revise["operation_id"] = json!(id());
    revise["action"] = json!("revise");
    revise["note"] = json!("测试：补充历史说明");
    revise["next_actions"] = json!([]);
    call(&mut db, &u, "review", revise).unwrap();
    assert_eq!(read(&mut db, &u, "board", "")["schedules"], before);
    assert_eq!(
        done["schedule"]["last_completed"],
        json!(skarma_core::foundation::sessions::today())
    );
}
#[test]
fn settings_preserve_manual_dates_and_knowledge_pause_does_not_auto_restore() {
    let (mut db, u, g) = fixture();
    let k = knowledge(&mut db, &u, &g, "difficulty");
    let kid = k["input"]["id"].as_str().unwrap();
    let mut edit = json!({"id":kid,"operation_id":id(),"expected_version":1,"action":"edit","input":k["input"],"details":k["details"],"note":"测试：暂停，稍后确认"});
    edit["details"]["state"] = json!("paused");
    edit["details"]["pause_until"] = json!("2099-01-01");
    call(&mut db, &u, "knowledge", edit).unwrap();
    let b = read(&mut db, &u, "board", "");
    let settings = &b["settings"];
    let mut cfg = serde_json::to_value(settings).unwrap();
    cfg["operation_id"] = json!(id());
    cfg["expected_version"] = settings["version"].clone();
    cfg.as_object_mut().unwrap().remove("version");
    cfg.as_object_mut().unwrap().remove("audit");
    cfg["monthly_ordinal"] = json!(-1);
    call(&mut db, &u, "settings", cfg).unwrap();
    let b = read(&mut db, &u, "board", "");
    let s = b["schedules"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["object_id"] == kid)
        .unwrap();
    assert_eq!(s["mode"], "resume");
    assert_eq!(s["eligible"], false);
    assert_eq!(s["due_date"], "2099-01-01");
    assert_eq!(
        read(&mut db, &u, "knowledge", kid)["knowledge"]["details"]["state"],
        "paused"
    );
}

#[test]
fn embedded_summary_feedback_is_atomic_unique_and_independent_revisions_are_protected() {
    let (mut db, u, g) = fixture();
    let k = knowledge(&mut db, &u, &g, "experience");
    let kid = k["input"]["id"].as_str().unwrap();
    let sid = id();
    let eid = id();
    let input = json!({"business_date":"2020-01-01","time_slot":"晚上","targets":[{"goal_id":g,"role":"main"}],"summary":{"targets":[{"goal_id":g,"performed":true,"observation":"测试目标表现"}],"confirmation":"none","feedback":[{"id":eid,"knowledge_id":kid,"result":"helpful"}]}});
    let request:skarma_core::foundation::session_model::Write=serde_json::from_value(json!({"id":sid,"operation_id":id(),"expected_version":null,"action":"complete","input":input})).unwrap();
    let first = store::write_foundation_session(&mut db, &u, request.clone()).unwrap();
    store::write_foundation_session(&mut db, &u, request).unwrap();
    assert_eq!(read(&mut db, &u, "knowledge", kid)["summary"]["total"], 1);
    assert_eq!(
        first.input.summary.as_ref().unwrap().feedback[0].expected_version,
        Some(1)
    );
    let mut revised = first.input;
    revised.summary.as_mut().unwrap().feedback[0].result = "uncertain".into();
    let second=store::write_foundation_session(&mut db,&u,serde_json::from_value(json!({"id":sid,"operation_id":id(),"expected_version":1,"action":"complete","input":revised,"change_note":"测试明确更正结果"})).unwrap()).unwrap();
    assert_eq!(
        read(&mut db, &u, "knowledge", kid)["summary"]["counts"]["uncertain"],
        1
    );
    let original = read(&mut db, &u, "event_history", &eid)["history"][0].clone();
    let mut child = original["input"].clone();
    child["result"] = json!("unhelpful");
    call(&mut db,&u,"feedback",json!({"id":eid,"operation_id":id(),"expected_version":2,"action":"revise","input":child,"note":"测试单独编辑的不同观察"})).unwrap();
    let stale:skarma_core::foundation::session_model::Write=serde_json::from_value(json!({"id":sid,"operation_id":id(),"expected_version":2,"action":"complete","input":second.input,"change_note":"测试旧表单"})).unwrap();
    assert!(store::write_foundation_session(&mut db, &u, stale).is_err());
    assert_eq!(
        store::foundation_snapshot(&mut db).unwrap().sessions[0].version,
        2
    );
    assert_eq!(read(&mut db, &u, "knowledge", kid)["summary"]["total"], 1);
}
#[test]
fn review_context_only_counts_explicit_signal_scope() {
    let (mut db, u, g) = fixture();
    let mid = id();
    store::write_foundation(&mut db,&u,serde_json::from_value(json!({"id":mid,"operation_id":id(),"expected_version":null,"change_note":"","mutation":{"kind":"method","value":{"title":"测试方法","goal_ids":[g]}}})).unwrap()).unwrap();
    let sid = id();
    store::write_foundation_session(&mut db,&u,serde_json::from_value(json!({"id":sid,"operation_id":id(),"expected_version":null,"action":"complete","input":{"business_date":skarma_core::foundation::sessions::today(),"time_slot":"晚上","summary":{"targets":[{"goal_id":g,"performed":true,"observation":"测试真实观察"}],"confirmation":"used","methods":[{"id":mid,"version":1}],"signals":[{"kind":"主动开始","method_ids":[mid]},{"kind":"求助","goal_ids":[g]},{"kind":"投入愉快"}]}}})).unwrap()).unwrap();
    let goal = call(
        &mut db,
        &u,
        "read",
        json!({"view":"review_context","id":g,"object_kind":"goal"}),
    )
    .unwrap();
    assert_eq!(goal["evidence"][0]["signals"].as_array().unwrap().len(), 1);
    assert_eq!(goal["evidence"][0]["signals"][0]["kind"], "求助");
    let method = call(
        &mut db,
        &u,
        "read",
        json!({"view":"review_context","id":mid,"object_kind":"method"}),
    )
    .unwrap();
    assert_eq!(
        method["evidence"][0]["signals"].as_array().unwrap().len(),
        1
    );
    assert_eq!(method["evidence"][0]["signals"][0]["kind"], "主动开始");
}
#[test]
fn group_deferral_checks_all_tokens_and_rolls_back_the_whole_group() {
    let (mut db, u, g) = fixture();
    let second = id();
    store::write_foundation(&mut db,&u,serde_json::from_value(json!({"id":second,"operation_id":id(),"expected_version":null,"change_note":"","mutation":{"kind":"goal","value":{"title":"测试第二目标","level":"short"}}})).unwrap()).unwrap();
    let first = review_input(&mut db, &u, &g);
    let other = review_input(&mut db, &u, &second);
    let token = |r: &Value| json!({"object_kind":r["object_kind"],"object_id":r["object_id"],"object_version":r["object_version"],"schedule_version":r["schedule_version"],"generation":r["generation"],"due_date":r["due_date"]});
    let mut group = first.clone();
    group["action"] = json!("defer_group");
    group["next_date"] = json!("2099-01-01");
    group["group_tokens"] = json!([token(&first), token(&other)]);
    store::write_foundation(&mut db,&u,serde_json::from_value(json!({"id":second,"operation_id":id(),"expected_version":1,"change_note":"测试状态修订","mutation":{"kind":"goal","value":{"title":"测试第二目标","level":"short","state":"maintaining"}}})).unwrap()).unwrap();
    assert!(call(&mut db, &u, "review", group.clone()).is_err());
    let unchanged = review_input(&mut db, &u, &g);
    assert_eq!(unchanged["schedule_version"], first["schedule_version"]);
    let new_other = review_input(&mut db, &u, &second);
    group["group_tokens"] = json!([token(&unchanged), token(&new_other)]);
    let saved = call(&mut db, &u, "review", group.clone()).unwrap();
    assert_eq!(saved["deferred_group"], 2);
    call(&mut db, &u, "review", group).unwrap();
    assert_eq!(
        db.query_row::<i64, _, _>("SELECT COUNT(*) FROM prd_reviews", [], |r| r.get(0))
            .unwrap(),
        0
    );
}

#[test]
fn actions_have_member_edit_history_and_are_locked_when_reserved_by_a_plan() {
    let (mut db, u, g) = fixture();
    let mut r = review_input(&mut db, &u, &g);
    let aid = id();
    r["next_actions"] = json!([{"id":aid,"content":"测试待做","goal_ids":[g],"assignee_id":null}]);
    call(&mut db, &u, "review", r).unwrap();
    let a = store::foundation_snapshot(&mut db).unwrap().actions[0].clone();
    let edit = json!({"id":aid,"operation_id":id(),"expected_version":a.version,"input":a.input,"state":"cancelled","note":"测试：明确取消此下一步"});
    let other = user();
    let done = call(&mut db, &other, "action", edit.clone()).unwrap();
    assert_eq!(done["status"], "cancelled");
    assert_eq!(done["audit"]["actor_id"], other.id);
    assert_eq!(
        read(&mut db, &u, "action", &aid)["history"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let mut stale = edit;
    stale["operation_id"] = json!(id());
    assert!(call(&mut db, &u, "action", stale).is_err());
    let mut restore = json!({"id":aid,"operation_id":id(),"expected_version":2,"input":done["input"],"state":"pending","note":"测试恢复待做"});
    call(&mut db, &u, "action", restore.clone()).unwrap();
    let sid = id();
    store::write_foundation_session(&mut db,&u,serde_json::from_value(json!({"id":sid,"operation_id":id(),"expected_version":null,"action":"save_draft","input":{"business_date":"2020-01-01","time_slot":"晚上","action_ids":[aid]}})).unwrap()).unwrap();
    restore["expected_version"] = json!(4);
    restore["operation_id"] = json!(id());
    restore["state"] = json!("cancelled");
    assert!(call(&mut db, &u, "action", restore).is_err());
}
#[test]
fn ordinary_pause_edits_keep_a_recovery_date_without_automatically_restoring_the_object() {
    let (mut db, u, g) = fixture();
    store::write_foundation(&mut db,&u,serde_json::from_value(json!({"id":g,"operation_id":id(),"expected_version":1,"change_note":"测试暂停并确认恢复日期","mutation":{"kind":"goal","value":{"title":"测试近期本领","level":"short","state":"paused","resume_on":"2020-01-01"}}})).unwrap()).unwrap();
    let b = read(&mut db, &u, "board", "");
    let schedule = b["schedules"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["object_id"] == g)
        .unwrap();
    assert_eq!(schedule["mode"], "resume");
    assert_eq!(schedule["eligible"], false);
    assert_eq!(schedule["due_date"], "2020-01-01");
    let first = store::foundation_snapshot(&mut db).unwrap();
    assert!(matches!(
        first.goals[0].input.state,
        Some(skarma_core::foundation::model::GoalState::Paused)
    ));
}
#[test]
fn already_reviewed_versions_are_not_new_evidence_but_later_records_still_are() {
    let (mut db, u, g) = fixture();
    let make_session = |sid: &str| {
        serde_json::from_value(json!({"id":sid,"operation_id":id(),"expected_version":null,"action":"complete","input":{"business_date":skarma_core::foundation::sessions::today(),"time_slot":"晚上","summary":{"targets":[{"goal_id":g,"performed":true,"observation":"测试实际记录"}],"confirmation":"none"}}})).unwrap()
    };
    store::write_foundation_session(&mut db, &u, make_session(&id())).unwrap();
    let c = call(
        &mut db,
        &u,
        "read",
        json!({"view":"review_context","id":g,"object_kind":"goal"}),
    )
    .unwrap();
    let mut r = review_input(&mut db, &u, &g);
    r["seen_versions"] = c["seen_versions"].clone();
    call(&mut db, &u, "review", r).unwrap();
    let after = call(
        &mut db,
        &u,
        "read",
        json!({"view":"review_context","id":g,"object_kind":"goal"}),
    )
    .unwrap();
    assert_eq!(after["total"], 0);
    let mut duplicate = review_input(&mut db, &u, &g);
    duplicate["seen_versions"] = after["seen_versions"].clone();
    assert!(call(&mut db, &u, "review", duplicate).is_err());
    store::write_foundation_session(&mut db, &u, make_session(&id())).unwrap();
    assert_eq!(
        call(
            &mut db,
            &u,
            "read",
            json!({"view":"review_context","id":g,"object_kind":"goal"})
        )
        .unwrap()["total"],
        1
    );
}

#[test]
fn completed_summary_keeps_a_disabled_members_historical_action_assignment() {
    let (mut db, u, g) = fixture();
    store::initialize_prd_runtime(&db).unwrap();
    let owner = id();
    db.execute("INSERT INTO auth_users(id,username,display_name,password_hash,role,active,created_at) VALUES(?1,'synthetic-old','测试原负责家人','synthetic-unused-hash','member',1,'2020-01-01')",[&owner]).unwrap();
    let sid = id();
    let aid = id();
    let input = json!({"business_date":"2020-01-01","time_slot":"晚上","summary":{"targets":[{"goal_id":g,"performed":true,"observation":"测试实际表现"}],"confirmation":"none","new_actions":[{"id":aid,"content":"测试已有下一步","goal_ids":[g],"assignee_id":owner}]}});
    let first=store::write_foundation_session(&mut db,&u,serde_json::from_value(json!({"id":sid,"operation_id":id(),"expected_version":null,"action":"complete","input":input})).unwrap()).unwrap();
    db.execute("UPDATE auth_users SET active=0 WHERE id=?1", [&owner])
        .unwrap();
    let second=store::write_foundation_session(&mut db,&u,serde_json::from_value(json!({"id":sid,"operation_id":id(),"expected_version":1,"action":"complete","change_note":"测试补充原总结","input":first.input})).unwrap()).unwrap();
    let mut changed = second.input;
    changed.summary.as_mut().unwrap().new_actions[0].content = "测试更清楚的下一步表述".into();
    store::write_foundation_session(&mut db,&u,serde_json::from_value(json!({"id":sid,"operation_id":id(),"expected_version":2,"action":"complete","change_note":"测试修正表述","input":changed})).unwrap()).unwrap();
    let actions = store::foundation_snapshot(&mut db).unwrap().actions;
    assert_eq!(
        actions[0].input.assignee_id.as_deref(),
        Some(owner.as_str())
    );
    assert_eq!(actions[0].assignee_name.as_deref(), Some("测试原负责家人"));
    assert_eq!(actions[0].version, 2);
}

#[test]
fn method_adjustment_keeps_the_exact_knowledge_version_without_copying_recursive_sources() {
    let (mut db, u, g) = fixture();
    let k = knowledge(&mut db, &u, &g, "experience");
    let mid = id();
    let make = |version: Option<u64>, basis_version: u64, note: &str| {
        serde_json::from_value(json!({"id":mid,"operation_id":id(),"expected_version":version,"change_note":note,"mutation":{"kind":"method","value":{"title":"测试根据心得调整","goal_ids":[g],"basis":[{"id":k["input"]["id"],"version":basis_version}]}}})).unwrap()
    };
    let initial = store::write_foundation(&mut db, &u, make(None, 1, "")).unwrap();
    assert_eq!(
        serde_json::to_value(&initial).unwrap()["value"]["basis_snapshots"][0]["content"],
        "测试做法或困难"
    );
    let mut updated = k["input"].clone();
    updated["content"] = json!("测试新条件下的做法");
    call(&mut db,&u,"knowledge",json!({"id":k["input"]["id"],"operation_id":id(),"expected_version":1,"action":"edit","input":updated,"details":k["details"],"note":"补充新条件"})).unwrap();
    assert!(store::write_foundation(&mut db, &u, make(Some(1), 2, "")).is_err());
    let adjusted =
        store::write_foundation(&mut db, &u, make(Some(1), 2, "根据新条件调整步骤")).unwrap();
    assert_eq!(
        serde_json::to_value(adjusted).unwrap()["value"]["basis_snapshots"][0]["content"],
        "测试新条件下的做法"
    );
    let history = store::foundation_history(&mut db, &mid).unwrap();
    assert_eq!(
        serde_json::to_value(&history[0]).unwrap()["entity"]["value"]["basis_snapshots"][0]["content"],
        "测试做法或困难"
    );
}

#[test]
fn review_pagination_only_consumes_loaded_evidence_and_accumulates_prior_confirmations() {
    let (mut db, u, g) = fixture();
    let today = skarma_core::foundation::sessions::today();
    for n in 0..31 {
        let mid = id();
        call(&mut db,&u,"observe",json!({"id":mid,"operation_id":id(),"expected_version":null,"kind":"moment","action":"save","input":{"business_date":today,"text":format!("合成观察{n}"),"goal_ids":[g]}})).unwrap();
    }
    let context = |db: &mut Connection, page: u32| {
        call(
            db,
            &u,
            "read",
            json!({"view":"review_context","object_kind":"goal","id":g,"page":page}),
        )
        .unwrap()
    };
    let first = context(&mut db, 0);
    let second_page = context(&mut db, 1);
    assert_eq!(first["total"], 31);
    assert_eq!(first["seen_versions"].as_object().unwrap().len(), 30);
    assert_eq!(second_page["seen_versions"].as_object().unwrap().len(), 1);
    assert!(first.get("_all_versions").is_none());
    assert_eq!(
        first["evidence_signature"],
        second_page["evidence_signature"]
    );
    let mut r = review_input(&mut db, &u, &g);
    r["seen_versions"] = first["seen_versions"].clone();
    call(&mut db, &u, "review", r).unwrap();
    let remaining = context(&mut db, 0);
    assert_eq!(remaining["total"], 1);
    let mut next = review_input(&mut db, &u, &g);
    next["seen_versions"] = remaining["seen_versions"].clone();
    let completed = call(&mut db, &u, "review", next).unwrap();
    assert_eq!(
        completed["review"]["seen_versions"]
            .as_object()
            .unwrap()
            .len(),
        31
    );
    assert_eq!(context(&mut db, 0)["total"], 0);
}
