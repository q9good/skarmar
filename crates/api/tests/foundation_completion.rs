//! Synthetic acceptance of progress, lesson feedback and review context. No live data.
use axum::http::StatusCode;
use rusqlite::Connection;
use serde_json::{Value, json};
use skarma_api::store;
use skarma_core::auth::{AccountRole, User};
use uuid::Uuid;
fn id() -> String {
    Uuid::new_v4().to_string()
}
fn process(db: &mut Connection, u: &User, kind: &str, input: Value) -> store::Result<Value> {
    store::foundation_process(
        db,
        u,
        serde_json::from_value(json!({"kind":kind,"value":input})).unwrap(),
    )
}
fn read(db: &mut Connection, u: &User, view: &str, object: &str, kind: &str) -> Value {
    process(
        db,
        u,
        "read",
        json!({"view":view,"id":object,"object_kind":kind}),
    )
    .unwrap()
}
fn object(db: &mut Connection, u: &User, id: &str, kind: &str, input: Value) {
    let mut request = json!({"id":id,"operation_id":self::id(),"expected_version":null,"change_note":"","mutation":{"kind":kind,"value":input}});
    if kind == "method" {
        request["method_create_mode"] = json!("quick");
    }
    store::write_foundation(db, u, serde_json::from_value(request).unwrap()).unwrap();
}
fn lesson(
    db: &mut Connection,
    u: &User,
    id: &str,
    expected: Option<u64>,
    kind: &str,
    input: Value,
) -> store::Result<Value> {
    store::write_foundation_lesson(db,u,serde_json::from_value(json!({"id":id,"operation_id":self::id(),"expected_version":expected,"action":{"kind":kind,"value":input}})).unwrap()).map(|v|serde_json::to_value(v).unwrap())
}
struct Fixture {
    db: Connection,
    u: User,
    g: String,
    m: String,
    k: String,
    l: String,
    observation: Value,
}
fn fixture() -> Fixture {
    let mut db = Connection::open_in_memory().unwrap();
    store::initialize_foundation(&db).unwrap();
    let u = User {
        id: id(),
        username: "synthetic".into(),
        display_name: "测试家人".into(),
        role: AccountRole::Member,
        active: true,
    };
    let (g, m, k, l, c, t) = (id(), id(), id(), id(), id(), id());
    object(
        &mut db,
        &u,
        &g,
        "goal",
        json!({"title":"补充近期本领","level":"short"}),
    );
    object(
        &mut db,
        &u,
        &m,
        "method",
        json!({"title":"轮流滚球","description":"先跟随玩法","goal_ids":[g]}),
    );
    for (id, kind) in [(&c, "course"), (&t, "teacher")] {
        object(
            &mut db,
            &u,
            id,
            "catalog",
            json!({"kind":kind,"name":format!("合成{kind}"),"active":true}),
        );
    }
    process(&mut db,&u,"knowledge",json!({"id":k,"operation_id":id(),"expected_version":null,"action":"create","input":{"id":k,"kind":"experience","content":"先跟随再示范","goal_ids":[g],"method_ids":[m],"stage":"helpful"},"details":{"title":"既有经验","evidence":"家人已有观察"}})).unwrap();
    let observation =
        json!({"business_date":"2020-01-01","course_id":c,"teacher_id":t,"notes":"原来课堂观察"});
    lesson(&mut db, &u, &l, None, "observation", observation.clone()).unwrap();
    Fixture {
        db,
        u,
        g,
        m,
        k,
        l,
        observation,
    }
}
fn organization(f: &Fixture) -> Value {
    let fid = id();
    json!({"seen_observation_version":1,"targets":[{"goal_id":f.g,"note":"明确观察"}],"method_ids":[f.m],"feedback":[{"id":fid,"knowledge_id":f.k,"result":"helpful","observation":"先示范后愿意再试","expected_version":null}],"feedback_knowledge_versions":{f.k.clone():1}})
}
#[test]
fn lesson_feedback_is_atomic_idempotent_and_keeps_the_selected_observation_version() {
    let mut f = fixture();
    let mut later = f.observation.clone();
    later["notes"] = json!("之后补充的新观察");
    later["business_date"] = json!("2020-01-02");
    lesson(&mut f.db, &f.u, &f.l, Some(1), "observation", later).unwrap();
    let input = organization(&f);
    let command:skarma_core::foundation::lesson_model::LessonWrite=serde_json::from_value(json!({"id":f.l,"operation_id":id(),"expected_version":1,"action":{"kind":"organization","value":input}})).unwrap();
    f.db.execute_batch("CREATE TRIGGER synthetic_feedback_failure BEFORE INSERT ON prd_feedback BEGIN SELECT RAISE(ABORT,'synthetic late failure'); END;").unwrap();
    assert_eq!(
        store::write_foundation_lesson(&mut f.db, &f.u, command.clone())
            .unwrap_err()
            .0,
        StatusCode::INTERNAL_SERVER_ERROR
    );
    let current = store::foundation_lesson_history(&mut f.db, &f.l).unwrap();
    assert_eq!(current.len(), 2);
    assert_eq!(current.last().unwrap().lesson.organization_version, 1);
    assert_eq!(
        f.db.query_row("SELECT COUNT(*) FROM prd_feedback", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    f.db.execute_batch("DROP TRIGGER synthetic_feedback_failure;")
        .unwrap();
    let saved = store::write_foundation_lesson(&mut f.db, &f.u, command.clone()).unwrap();
    assert_eq!(saved.observation_version, 2);
    assert_eq!(saved.organization_version, 2);
    assert_eq!(saved.organization.feedback_ids.len(), 1);
    assert_eq!(
        serde_json::to_value(store::write_foundation_lesson(&mut f.db, &f.u, command).unwrap())
            .unwrap(),
        serde_json::to_value(&saved).unwrap()
    );
    let events = read(&mut f.db, &f.u, "lesson_feedback", &f.l, "");
    assert_eq!(events.as_array().unwrap().len(), 1);
    let e = &events[0];
    assert_eq!(e["input"]["observation_version"], 1);
    assert_eq!(e["input"]["business_date"], "2020-01-01");
    assert_eq!(
        e["source_snapshot"]["referenced_observation"]["value"]["notes"],
        "原来课堂观察"
    );
    assert_eq!(
        e["source_snapshot"]["observation"]["notes"],
        "之后补充的新观察"
    );
    assert_eq!(
        read(&mut f.db, &f.u, "knowledge", &f.k, "")["summary"]["total"],
        1
    );
    let mut duplicate = input;
    duplicate["feedback"][0]["id"] = json!(id());
    assert_eq!(
        lesson(&mut f.db, &f.u, &f.l, Some(2), "organization", duplicate)
            .unwrap_err()
            .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        read(&mut f.db, &f.u, "knowledge", &f.k, "")["summary"]["total"],
        1
    );
    assert_eq!(store::write_foundation_lesson(&mut f.db,&f.u,serde_json::from_value(json!({"id":f.l,"operation_id":id(),"expected_version":saved.version,"action":{"kind":"withdraw"}})).unwrap()).unwrap_err().0,StatusCode::CONFLICT);
}
#[test]
fn changed_knowledge_unrelated_feedback_and_invalid_results_leave_lesson_unchanged() {
    let mut f = fixture();
    let original = organization(&f);
    let mut bad = original.clone();
    bad["feedback"][0]["result"] = json!("improved");
    assert_eq!(
        lesson(&mut f.db, &f.u, &f.l, Some(1), "organization", bad)
            .unwrap_err()
            .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    process(&mut f.db,&f.u,"knowledge",json!({"id":f.k,"operation_id":id(),"expected_version":1,"action":"edit","input":{"id":f.k,"kind":"experience","content":"经过独立修订的做法","goal_ids":[f.g],"method_ids":[f.m],"stage":"helpful"},"details":{"title":"既有经验"},"note":"更正条件"})).unwrap();
    assert_eq!(
        lesson(
            &mut f.db,
            &f.u,
            &f.l,
            Some(1),
            "organization",
            original.clone()
        )
        .unwrap_err()
        .0,
        StatusCode::CONFLICT
    );
    let mut unrelated = original;
    unrelated["feedback_knowledge_versions"][&f.k] = json!(2);
    unrelated["targets"] = json!([]);
    unrelated["method_ids"] = json!([]);
    assert_eq!(
        lesson(&mut f.db, &f.u, &f.l, Some(1), "organization", unrelated)
            .unwrap_err()
            .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let h = store::foundation_lesson_history(&mut f.db, &f.l).unwrap();
    assert_eq!(h.len(), 1);
    assert_eq!(h[0].lesson.organization_version, 1);
    assert_eq!(
        read(&mut f.db, &f.u, "lesson_feedback", &f.l, "")
            .as_array()
            .unwrap()
            .len(),
        0
    );
}
fn review(context: &Value, g: &str) -> Value {
    json!({"id":id(),"operation_id":id(),"expected_version":null,"object_kind":"goal","object_id":g,"object_version":context["object_version"],"schedule_version":context["schedule"]["version"],"generation":context["schedule"]["generation"],"due_date":context["schedule"]["due_date"],"action":"complete","decision":"continue","note":"明确回看实际观察","evidence":[],"seen_versions":context["seen_versions"]})
}
#[test]
fn review_burden_is_optional_auditable_and_never_advances_schedule_on_correction() {
    let mut f = fixture();
    let context = read(&mut f.db, &f.u, "review_context", &f.g, "goal");
    let mut command = review(&context, &f.g);
    command["caregiver_state"] = json!("bad");
    assert_eq!(
        process(&mut f.db, &f.u, "review", command.clone())
            .unwrap_err()
            .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    command["caregiver_state"] = json!("stretched");
    let result = process(&mut f.db, &f.u, "review", command.clone()).unwrap();
    assert_eq!(result["review"]["caregiver_state"], "stretched");
    let schedule = result["schedule"].clone();
    let original = result["review"].clone();
    let mut revised = command.clone();
    revised["operation_id"] = json!(id());
    revised["expected_version"] = json!(1);
    revised["action"] = json!("revise");
    revised["note"] = json!("更正家人当时的感受");
    revised["caregiver_state"] = json!("easy");
    let saved = process(&mut f.db, &f.u, "review", revised).unwrap();
    assert_eq!(saved["caregiver_state"], "easy");
    assert_eq!(saved["version"], 2);
    assert_eq!(
        read(&mut f.db, &f.u, "review_context", &f.g, "goal")["schedule"],
        schedule
    );
    assert_eq!(
        process(&mut f.db, &f.u, "review", command).unwrap()["review"],
        original
    );
    assert_eq!(
        read(
            &mut f.db,
            &f.u,
            "review_history",
            saved["id"].as_str().unwrap(),
            ""
        )["history"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}
#[test]
fn concurrent_context_only_reports_explicit_starts_and_keeps_method_scope() {
    let mut f = fixture();
    let g2 = id();
    let m2 = id();
    object(
        &mut f.db,
        &f.u,
        &g2,
        "goal",
        json!({"title":"另一个目标","level":"short"}),
    );
    object(
        &mut f.db,
        &f.u,
        &m2,
        "method",
        json!({"title":"另一个方法","goal_ids":[g2]}),
    );
    let sid = id();
    store::write_foundation_session(&mut f.db,&f.u,serde_json::from_value(json!({"id":sid,"operation_id":id(),"expected_version":null,"action":"complete","input":{"business_date":"2020-01-01","time_slot":"晚上","targets":[{"goal_id":f.g,"role":"main"},{"goal_id":g2,"role":"secondary"}],"summary":{"targets":[{"goal_id":f.g,"performed":true,"observation":"实际表现一"},{"goal_id":g2,"performed":true,"observation":"实际表现二"}],"confirmation":"used","methods":[{"id":f.m,"version":1},{"id":m2,"version":1}],"first_try_ids":[f.m],"context":"家人原文：当天换了材料"}}})).unwrap()).unwrap();
    let mut org = organization(&f);
    org["feedback"] = json!([]);
    lesson(&mut f.db, &f.u, &f.l, Some(1), "organization", org).unwrap();
    let ctx = read(&mut f.db, &f.u, "review_context", &f.g, "goal");
    let lesson_e = ctx["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["source"]["kind"] == "lesson")
        .unwrap();
    assert!(
        lesson_e["changes"].as_array().unwrap().is_empty(),
        "first stored lesson must not mean new course"
    );
    let session_e = ctx["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["source"]["kind"] == "session")
        .unwrap();
    assert_eq!(session_e["changes"][0]["kind"], "method_first_try");
    assert_eq!(session_e["changes"][1]["text"], "家人原文：当天换了材料");
    let ctx2 = read(&mut f.db, &f.u, "review_context", &g2, "goal");
    assert!(
        !ctx2["evidence"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|e| e["changes"].as_array().unwrap())
            .any(|c| c["kind"] == "method_first_try")
    );
    let mut marked = f.observation.clone();
    marked["course_started"] = json!(true);
    lesson(&mut f.db, &f.u, &f.l, Some(1), "observation", marked).unwrap();
    let ctx = read(&mut f.db, &f.u, "review_context", &f.g, "goal");
    assert!(
        ctx["evidence"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|e| e["changes"].as_array().unwrap())
            .any(|c| c["kind"] == "course_started")
    );
    let preserved = lesson(
        &mut f.db,
        &f.u,
        &f.l,
        Some(2),
        "observation",
        f.observation.clone(),
    )
    .unwrap();
    assert_eq!(preserved["observation"]["course_started"], true);
    let mut unmarked = f.observation;
    unmarked["course_started"] = json!(false);
    let result = lesson(&mut f.db, &f.u, &f.l, Some(3), "observation", unmarked).unwrap();
    assert_eq!(result["observation"]["course_started"], false);
}
#[test]
fn goal_progress_uses_actual_measurements_and_retains_zero_opportunity_unknowns() {
    let mut f = fixture();
    let metric = id();
    process(&mut f.db,&f.u,"observe",json!({"id":metric,"operation_id":id(),"expected_version":null,"kind":"metric","action":"save","input":{"goal_id":f.g,"goal_version":1,"name":"轮替机会","template":"count","criterion":"五次机会","support":"一次提醒"}})).unwrap();
    let mut mids = vec![];
    for (day, n, k, support) in [
        ("2020-01-01", 5, 3, "一次提醒"),
        ("2020-01-02", 0, 0, "一次提醒"),
        ("2020-01-03", 5, 4, "不提醒"),
    ] {
        let m = id();
        process(&mut f.db,&f.u,"observe",json!({"id":m,"operation_id":id(),"expected_version":null,"kind":"measurement","action":"save","input":{"metric_id":metric,"metric_version":1,"business_date":day,"opportunities":n,"achieved":k,"support":support}})).unwrap();
        mids.push(m);
    }
    let p = read(&mut f.db, &f.u, "goal_progress", "", "");
    assert_eq!(p["goals"][0]["measurement_count"], 3);
    assert_eq!(p["goals"][0]["segment_count"], 2);
    assert!(
        p["goals"][0]["latest"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["input"]["opportunities"] == 0)
    );
    assert_eq!(
        f.db.query_row("SELECT COUNT(*) FROM prd_sessions", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    let before: String =
        f.db.query_row(
            "SELECT data FROM prd_observations WHERE id=?1",
            [&mids[1]],
            |r| r.get(0),
        )
        .unwrap();
    read(&mut f.db, &f.u, "goal_progress", "", "");
    assert_eq!(
        f.db.query_row(
            "SELECT data FROM prd_observations WHERE id=?1",
            [&mids[1]],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        before
    );
}

#[test]
fn shared_review_queue_returns_current_relations_names_and_schedules_together() {
    let mut f = fixture();
    let first = read(&mut f.db, &f.u, "review_queue", "", "");
    assert_eq!(first["state"]["schema"], "skarmar.prd.v1");
    let new_goal = id();
    object(
        &mut f.db,
        &f.u,
        &new_goal,
        "goal",
        json!({"title":"最新关联目标","level":"medium"}),
    );
    store::write_foundation(&mut f.db,&f.u,serde_json::from_value(json!({"id":f.m,"operation_id":id(),"expected_version":1,"change_note":"核对最新适用范围","mutation":{"kind":"method","value":{"title":"最新方法名称","goal_ids":[f.g,new_goal]}}})).unwrap()).unwrap();
    let second = read(&mut f.db, &f.u, "review_queue", "", "");
    let method = second["state"]["methods"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["id"] == f.m)
        .unwrap();
    assert_eq!(method["version"], 2);
    assert_eq!(method["input"]["title"], "最新方法名称");
    assert_eq!(method["input"]["goal_ids"].as_array().unwrap().len(), 2);
    assert_eq!(
        second["board"]["schedules"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|s| s["object_kind"] == "method" && s["object_id"] == f.m)
            .count(),
        1
    );
    assert!(
        second["board"]["data_version"].as_u64().unwrap()
            > first["board"]["data_version"].as_u64().unwrap()
    );
    assert!(
        second["state"].get("feedback").is_none(),
        "queue snapshot must not ship growing event histories"
    );
    assert_eq!(
        f.db.query_row("SELECT COUNT(*) FROM prd_feedback", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
}

fn practice(f: &mut Fixture, k: &str, result: &str, method_version: u64) -> String {
    let eid = id();
    let today: String =
        f.db.query_row("SELECT date('now','+8 hours')", [], |r| r.get(0))
            .unwrap();
    process(&mut f.db,&f.u,"feedback",json!({"id":eid,"operation_id":id(),"action":"record","input":{"knowledge_id":k,"source":{"kind":"independent","id":eid,"version":1},"business_date":today,"result":result,"observation":format!("观察{result}"),"comparable":true,"basis":"同样任务","support":"一次提示","method_refs":[{"id":f.m,"version":method_version}]}})).unwrap();
    eid
}
fn edit_method(f: &mut Fixture, version: u64, mode: Option<&str>, text: &str) {
    let mut input = json!({"id":f.m,"operation_id":id(),"expected_version":version,"change_note":"核实实际做法","mutation":{"kind":"method","value":{"title":"轮流滚球","description":text,"goal_ids":[f.g]}}});
    if let Some(mode) = mode {
        input["method_change"] = json!(mode);
    }
    let cmd = serde_json::from_value(input).unwrap();
    let saved = store::write_foundation(&mut f.db, &f.u, cmd).unwrap();
    assert_eq!(saved.version(), version + 1);
}
#[test]
fn practice_adjustment_splits_comparison_without_losing_history_or_resetting_on_correction() {
    let mut f = fixture();
    let k = id();
    process(&mut f.db,&f.u,"knowledge",json!({"id":k,"operation_id":id(),"action":"create","input":{"id":k,"kind":"difficulty","content":"轮流时离开","goal_ids":[f.g],"method_ids":[f.m]},"details":{"title":"合成轮替困难","state":"following","basis":"同样任务","support":"一次提示"}})).unwrap();
    for _ in 0..2 {
        practice(&mut f, &k, "same", 1);
    }
    edit_method(&mut f, 1, Some("correction"), "更正文案，实际做法相同");
    assert_eq!(
        read(&mut f.db, &f.u, "knowledge", &k, "")["summary"]["comparable_count"],
        2
    );
    edit_method(&mut f, 2, Some("adjustment"), "先跟随再增加小变化");
    let adjusted = read(&mut f.db, &f.u, "knowledge", &k, "");
    assert_eq!(
        adjusted["summary"]["comparison_boundary"]["method_version"],
        3
    );
    assert_eq!(adjusted["summary"]["comparable_count"], 0);
    practice(&mut f, &k, "same", 3);
    practice(&mut f, &k, "same", 1); // A late old-method reference is history, not new-phase evidence.
    let current = read(&mut f.db, &f.u, "knowledge", &k, "");
    assert_eq!(current["summary"]["total"], 4);
    assert_eq!(current["summary"]["comparable_count"], 1);
    assert_eq!(current["summary"]["offer_weekly"], false);
    for _ in 0..2 {
        practice(&mut f, &k, "same", 3);
    }
    assert_eq!(
        read(&mut f.db, &f.u, "knowledge", &k, "")["summary"]["offer_weekly"],
        true
    );
    edit_method(&mut f, 3, None, "实际步骤再次调整");
    assert_eq!(
        read(&mut f.db, &f.u, "knowledge", &k, "")["summary"]["comparable_count"],
        0
    );
}
#[test]
fn latest_counterexample_survives_paging_and_tracks_result_correction() {
    let mut f = fixture();
    let k = f.k.clone();
    let contrary = practice(&mut f, &k, "unhelpful", 1);
    for _ in 0..32 {
        practice(&mut f, &k, "helpful", 1);
    }
    let s = read(&mut f.db, &f.u, "knowledge", &k, "");
    assert_eq!(s["summary"]["latest_results"]["unhelpful"]["id"], contrary);
    assert!(
        !s["summary"]["latest"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["id"] == contrary)
    );
    let e = read(&mut f.db, &f.u, "events", &k, "");
    assert_eq!(e["events"].as_array().unwrap().len(), 30);
    let old = read(&mut f.db, &f.u, "event_history", &contrary, "")["history"][0]["input"].clone();
    let mut input = old;
    input["result"] = json!("uncertain");
    process(&mut f.db,&f.u,"feedback",json!({"id":contrary,"operation_id":id(),"expected_version":1,"action":"revise","input":input,"note":"核对后更正结果"})).unwrap();
    let s = read(&mut f.db, &f.u, "knowledge", &k, "");
    assert!(s["summary"]["latest_results"]["unhelpful"].is_null());
    assert_eq!(s["summary"]["latest_results"]["uncertain"]["version"], 2);
}
#[test]
fn partial_review_keeps_older_unseen_evidence_for_the_next_review() {
    let mut f = fixture();
    for n in 0..5 {
        process(&mut f.db,&f.u,"observe",json!({"id":id(),"operation_id":id(),"kind":"moment","action":"save","input":{"business_date":"2020-01-01","text":format!("旧观察{n}"),"goal_ids":[f.g]}})).unwrap();
    }
    let c = read(&mut f.db, &f.u, "review_context", &f.g, "goal");
    assert_eq!(c["total"], 5);
    let mut seen = serde_json::Map::new();
    for e in c["evidence"].as_array().unwrap().iter().take(3) {
        for (k, v) in e["seen_versions"].as_object().unwrap() {
            seen.insert(k.clone(), v.clone());
        }
    }
    let mut r = review(&c, &f.g);
    r["seen_versions"] = Value::Object(seen);
    process(&mut f.db, &f.u, "review", r.clone()).unwrap();
    // Retrying the completed operation does not consume more evidence or create another review.
    process(&mut f.db, &f.u, "review", r).unwrap();
    let next = read(&mut f.db, &f.u, "review_context", &f.g, "goal");
    assert_eq!(next["total"], 2);
    assert!(
        next["evidence"]
            .as_array()
            .unwrap()
            .iter()
            .all(|e| e["carried_over"] == true)
    );
    assert!(next["next_defaults"]["fortnightly"].as_str().unwrap() > next["to"].as_str().unwrap());
}
#[test]
fn entity_lesson_and_session_revision_pages_are_bounded_and_lossless() {
    let mut f = fixture();
    for version in 1..32 {
        edit_method(
            &mut f,
            version,
            Some("correction"),
            &format!("核对文字{version}"),
        );
        let mut obs = f.observation.clone();
        obs["notes"] = json!(format!("课记文字{version}"));
        lesson(&mut f.db, &f.u, &f.l, Some(version), "observation", obs).unwrap();
    }
    let sid = id();
    let input = json!({"business_date":"2020-01-01","time_slot":"晚上","targets":[{"goal_id":f.g,"role":"main"}],"plan":"合成计划"});
    for version in 0..32 {
        store::write_foundation_session(&mut f.db,&f.u,serde_json::from_value(json!({"id":sid,"operation_id":id(),"expected_version":if version==0{None}else{Some(version)},"action":"save_draft","input":input,"change_note":"补充计划"})).unwrap()).unwrap();
    }
    for (view, id) in [
        ("entity_history", f.m.clone()),
        ("lesson_history", f.l.clone()),
        ("session_history", sid),
    ] {
        let first = process(
            &mut f.db,
            &f.u,
            "read",
            json!({"view":view,"id":id,"page":0}),
        )
        .unwrap();
        let second = process(
            &mut f.db,
            &f.u,
            "read",
            json!({"view":view,"id":id,"page":1}),
        )
        .unwrap();
        assert_eq!(first["items"].as_array().unwrap().len(), 30, "{view}");
        assert_eq!(first["has_more"], true);
        assert_eq!(second["items"].as_array().unwrap().len(), 2, "{view}");
        assert_eq!(second["has_more"], false);
        let versions: Vec<u64> = first["items"]
            .as_array()
            .unwrap()
            .iter()
            .chain(second["items"].as_array().unwrap())
            .map(|r| match view {
                "entity_history" => r["entity"]["value"]["version"].as_u64().unwrap(),
                "lesson_history" => r["lesson"]["version"].as_u64().unwrap(),
                _ => r["session"]["version"].as_u64().unwrap(),
            })
            .collect();
        assert_eq!(
            versions,
            (1..=32).rev().collect::<Vec<_>>(),
            "{view} lost or repeated a version"
        );
    }
}

#[test]
fn method_changes_before_knowledge_creation_or_binding_do_not_create_retroactive_phases() {
    let mut f = fixture();
    edit_method(&mut f, 1, Some("adjustment"), "合成先前做法调整");
    let k = id();
    process(&mut f.db,&f.u,"knowledge",json!({"id":k,"operation_id":id(),"action":"create","input":{"id":k,"kind":"difficulty","content":"新记录的困难","goal_ids":[f.g],"method_ids":[f.m]},"details":{"title":"较晚建立的困难","basis":"同样任务","support":"一次提示"}})).unwrap();
    assert!(read(&mut f.db, &f.u, "knowledge", &k, "")["summary"]["comparison_boundary"].is_null());
    edit_method(&mut f, 2, Some("adjustment"), "困难已关联后的新做法");
    assert_eq!(
        read(&mut f.db, &f.u, "knowledge", &k, "")["summary"]["comparison_boundary"]["method_version"],
        3
    );
}

#[test]
fn difficulty_adjustments_within_one_day_start_distinct_phases_and_note_corrections_do_not() {
    let mut f = fixture();
    let k = id();
    process(&mut f.db,&f.u,"knowledge",json!({"id":k,"operation_id":id(),"action":"create","input":{"id":k,"kind":"difficulty","content":"需要调整观察的困难","goal_ids":[f.g],"method_ids":[f.m]},"details":{"title":"同日比较期","state":"following","basis":"同样任务","support":"一次提示"}})).unwrap();
    for _ in 0..2 {
        practice(&mut f, &k, "same", 1);
    }
    let mut saved = Value::Null;
    let mut last_command = Value::Null;
    for round in 0..2 {
        let c = read(&mut f.db, &f.u, "review_context", &k, "knowledge");
        let mut r = review(&c, &k);
        r["object_kind"] = json!("knowledge");
        r["decision"] = json!("adjust");
        r["note"] = json!(format!("第{round}次调整观察条件"));
        last_command = r.clone();
        saved = process(&mut f.db, &f.u, "review", r).unwrap();
        practice(&mut f, &k, "same", 1);
        let s = read(&mut f.db, &f.u, "knowledge", &k, "");
        assert_eq!(
            s["summary"]["comparable_count"], 1,
            "same-day round {round} reused old evidence"
        );
        assert_eq!(s["summary"]["offer_weekly"], false);
    }
    // Correcting a review explanation is not another practice adjustment.
    last_command["operation_id"] = json!(id());
    last_command["expected_version"] = json!(1);
    last_command["action"] = json!("revise");
    last_command["note"] = json!("更正说明文字，不改变做法");
    assert_eq!(last_command["id"], saved["review"]["id"]);
    process(&mut f.db, &f.u, "review", last_command).unwrap();
    assert_eq!(
        read(&mut f.db, &f.u, "knowledge", &k, "")["summary"]["comparable_count"],
        1
    );
    assert_eq!(
        read(&mut f.db, &f.u, "knowledge", &k, "")["summary"]["total"],
        4
    );
    for decision in ["resolve", "reopen"] {
        let c = read(&mut f.db, &f.u, "review_context", &k, "knowledge");
        let mut r = review(&c, &k);
        r["object_kind"] = json!("knowledge");
        r["decision"] = json!(decision);
        r["note"] = json!("合成状态复盘依据");
        process(&mut f.db, &f.u, "review", r).unwrap();
    }
    practice(&mut f, &k, "same", 1);
    assert_eq!(
        read(&mut f.db, &f.u, "knowledge", &k, "")["summary"]["comparable_count"],
        1
    );
    assert_eq!(
        read(&mut f.db, &f.u, "knowledge", &k, "")["summary"]["total"],
        5
    );
}
