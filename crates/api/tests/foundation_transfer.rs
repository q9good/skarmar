//! Synthetic conversion/import safety tests; no real workbook or production DB.
use axum::http::StatusCode;
use rusqlite::Connection;
use serde_json::{Value, json};
use skarma_api::store;
use skarma_core::{
    auth::{AccountRole, User},
    foundation::transfer::Command,
};
use uuid::Uuid;
fn id() -> String {
    Uuid::new_v4().to_string()
}
fn actor() -> User {
    User {
        id: id(),
        username: "synthetic".into(),
        display_name: "合成管理员".into(),
        role: AccountRole::Admin,
        active: true,
    }
}
fn call(
    db: &mut Connection,
    u: &User,
    kind: &str,
    value: Value,
) -> skarma_core::store::Result<Value> {
    store::data_transfer(
        db,
        u,
        serde_json::from_value(json!({"kind":kind,"value":value})).unwrap(),
    )
}
fn new_db() -> Connection {
    let db = Connection::open_in_memory().unwrap();
    store::initialize_prd_runtime(&db).unwrap();
    db
}
fn upload(db: &mut Connection, u: &User, sheets: Value) -> Value {
    call(
        db,
        u,
        "workbook",
        json!({"filename":"合成.xlsx","sha256":"synthetic-only","sheets":sheets}),
    )
    .unwrap()
}
fn sheet(name: &str, headers: Value, cells: Value) -> Value {
    json!({"name":name,"headers":headers,"rows":cells.as_array().unwrap().iter().enumerate().map(|(i,c)|json!({"row":i+2,"cells":c})).collect::<Vec<_>>()})
}
fn map(
    db: &mut Connection,
    u: &User,
    p: &Value,
    rules: Value,
) -> skarma_core::store::Result<Value> {
    call(
        db,
        u,
        "map",
        json!({"preview_id":p["preview_id"],"namespace":"synthetic-stable-document","rules":rules}),
    )
}
fn confirm(db: &mut Connection, u: &User, p: &Value) -> skarma_core::store::Result<Value> {
    call(
        db,
        u,
        "commit",
        json!({"preview_id":p["preview_id"],"digest":p["digest"]}),
    )
}
fn goal_rule() -> Value {
    json!({"sheet":"目标","kind":"goal","level":"short","fields":{"key":"编号","title":"名称"}})
}
fn state(db: &mut Connection) -> Value {
    // no auth bypass; direct state helper is only in tests
    let raw: String = db
        .query_row(
            "SELECT COALESCE(json_group_array(json(data)), '[]') FROM prd_entities",
            [],
            |r| r.get(0),
        )
        .unwrap();
    serde_json::from_str(&raw).unwrap()
}
#[test]
fn workbook_mapping_repeat_archive_relations_and_literal_source() {
    let mut db = new_db();
    let u = actor();
    let sheets = json!([
        sheet(
            "目标",
            json!(["编号", "名称", "额外字段"]),
            json!([["G1", "轮替", "=SUM(A1:A3)"]])
        ),
        sheet(
            "方法",
            json!(["编号", "名称", "关联"]),
            json!([["M1", "轮流滚球", "目标:G1"]])
        ),
        sheet(
            "经验",
            json!(["编号", "内容", "目标", "方法"]),
            json!([["K1", "合成待验证想法", "目标:G1", "方法:M1"]])
        ),
        sheet(
            "原说明",
            json!(["说明"]),
            json!([["保留🙂原文"], ["保留🙂原文"]])
        )
    ]);
    let rules = json!([goal_rule(),{"sheet":"方法","kind":"method","fields":{"key":"编号","title":"名称","goals":"关联"}},{"sheet":"经验","kind":"experience","fields":{"key":"编号","title":"内容","goals":"目标","methods":"方法"}},{"sheet":"原说明","kind":"archive","fields":{}}]);
    let p = upload(&mut db, &u, sheets.clone());
    let plan = map(&mut db, &u, &p, rules.clone()).unwrap();
    assert_eq!(state(&mut db).as_array().unwrap().len(), 0);
    let receipt = confirm(&mut db, &u, &plan).unwrap();
    assert_eq!(receipt["saved"], 3);
    assert_eq!(receipt["archived"], 2);
    assert_eq!(confirm(&mut db, &u, &plan).unwrap(), receipt);
    let raw: String = db
        .query_row(
            "SELECT data FROM prd_import_rows WHERE data LIKE '%=SUM%'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(raw.contains("=SUM(A1:A3)"));
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM prd_method_goals", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    let knowledge: String = db
        .query_row("SELECT data FROM prd_knowledge", [], |r| r.get(0))
        .unwrap();
    let k: Value = serde_json::from_str(&knowledge).unwrap();
    assert_eq!(k["input"]["stage"], "idea");
    assert_eq!(k["input"]["method_ids"].as_array().unwrap().len(), 1);
    let p2 = upload(&mut db, &u, sheets);
    let again = map(&mut db, &u, &p2, rules).unwrap();
    let repeat = confirm(&mut db, &u, &again).unwrap();
    assert_eq!(repeat["skipped"], 5);
    assert_eq!(repeat["saved"], 0);
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM prd_feedback", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
}
#[test]
fn import_detects_conflicts_permissions_expiry_and_rolls_back_partial_writes() {
    let mut db = new_db();
    let u = actor();
    let mut member = u.clone();
    member.role = AccountRole::Member;
    assert_eq!(
        store::data_transfer(&mut db, &member, Command::AuthorizeUpload)
            .unwrap_err()
            .0,
        StatusCode::FORBIDDEN
    );
    let p = upload(
        &mut db,
        &u,
        json!([sheet(
            "目标",
            json!(["编号", "名称"]),
            json!([["G1", "有效"], ["G2", "x".repeat(121)]])
        )]),
    );
    let plan = map(&mut db, &u, &p, json!([goal_rule()])).unwrap();
    assert_eq!(
        confirm(&mut db, &u, &plan).unwrap_err().0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert!(state(&mut db).as_array().unwrap().is_empty());
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM prd_import_rows", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    let p = upload(
        &mut db,
        &u,
        json!([sheet(
            "目标",
            json!(["编号", "名称"]),
            json!([["G1", "目标一"]])
        )]),
    );
    let plan = map(&mut db, &u, &p, json!([goal_rule()])).unwrap();
    db.execute(
        "INSERT INTO prd_import_batches(id,data) VALUES('synthetic-change','{}')",
        [],
    )
    .unwrap();
    assert_eq!(
        confirm(&mut db, &u, &plan).unwrap_err().0,
        StatusCode::CONFLICT
    );
    let plan = map(&mut db, &u, &p, json!([goal_rule()])).unwrap();
    confirm(&mut db, &u, &plan).unwrap();
    let p2 = upload(
        &mut db,
        &u,
        json!([sheet(
            "目标",
            json!(["编号", "名称"]),
            json!([["G1", "换了名字"]])
        )]),
    );
    assert_eq!(
        map(&mut db, &u, &p2, json!([goal_rule()])).unwrap_err().0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        call(
            &mut db,
            &member,
            "source",
            json!({"key":plan["rows"][0]["key"]})
        )
        .unwrap_err()
        .0,
        StatusCode::FORBIDDEN
    );
    db.execute("UPDATE prd_import_previews SET data=json_set(data,'$.expires_at','2000-01-01T00:00:00+00:00') WHERE id=?1",[p["preview_id"].as_str().unwrap()]).unwrap();
    assert_eq!(
        confirm(&mut db, &u, &plan).unwrap()["saved"],
        1,
        "committed receipt survives expiry"
    );
}
#[test]
fn duplicate_missing_keys_wrong_types_and_ambiguous_rules_do_not_create_business() {
    let mut db = new_db();
    let u = actor();
    let p = upload(
        &mut db,
        &u,
        json!([sheet(
            "目标",
            json!(["编号", "名称"]),
            json!([["G1", "甲"], ["G1", "乙"]])
        )]),
    );
    assert_eq!(
        map(&mut db, &u, &p, json!([goal_rule()])).unwrap_err().0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let p = upload(
        &mut db,
        &u,
        json!([sheet("目标", json!(["编号", "名称"]), json!([["", "甲"]]))]),
    );
    assert!(map(&mut db, &u, &p, json!([goal_rule()])).is_err());
    let p = upload(
        &mut db,
        &u,
        json!([sheet(
            "目标",
            json!(["编号", "名称"]),
            json!([["G1", "甲"]])
        )]),
    );
    assert!(map(&mut db, &u, &p, json!([])).is_err());
    let p = upload(
        &mut db,
        &u,
        json!([sheet(
            "目标",
            json!(["编号", "名称"]),
            json!([["G1", "甲"]])
        )]),
    );
    let mut r = goal_rule();
    r["kind"] = json!("session");
    assert!(map(&mut db, &u, &p, json!([r])).is_err());
    assert!(state(&mut db).as_array().unwrap().is_empty());
}
fn legacy() -> (Connection, User) {
    let mut db = Connection::open_in_memory().unwrap();
    store::initialize(&db).unwrap();
    store::seed_demo(&mut db).unwrap();
    let u = actor();
    db.execute("INSERT INTO auth_users(id,username,display_name,password_hash,role,active,created_at) VALUES(?1,'synthetic','合成管理员','synthetic-hash-only','admin',1,'2020-01-01')",[&u.id]).unwrap();
    (db, u)
}
fn upgrade_request(preview: &Value, disposition: &str) -> Value {
    json!({"source_digest":preview["source_digest"],"note":"合成转换；已检查逐条决策与备份方案","sessions":preview["sessions"].as_array().unwrap().iter().map(|s|json!({"id":s["id"],"disposition":disposition,"actual_goal_ids":[],"observations":{}})).collect::<Vec<_>>()})
}
#[test]
fn explicit_upgrade_retains_old_tables_accounts_archives_and_rolls_back_cleanly() {
    let (mut db, u) = legacy();
    let before = store::state(&db).unwrap();
    let preview = store::data_transfer(&mut db, &u, Command::UpgradePreview).unwrap();
    let request = upgrade_request(&preview, "archive");
    let result = call(&mut db, &u, "upgrade_commit", request.clone()).unwrap();
    store::initialize_prd_runtime(&db).unwrap();
    assert_eq!(store::initialize(&db).unwrap_err().0, StatusCode::CONFLICT);
    assert_eq!(
        db.query_row(
            "SELECT COUNT(*) FROM prd_entities WHERE kind='goal'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        before.goals.len() as i64
    );
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM prd_sessions", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        db.query_row("SELECT password_hash FROM auth_users", [], |r| r
            .get::<_, String>(0))
            .unwrap(),
        "synthetic-hash-only"
    );
    assert_eq!(
        call(&mut db, &u, "upgrade_commit", request).unwrap()["baseline"],
        result["baseline"]
    );
    assert_eq!(
        call(
            &mut db,
            &u,
            "rollback",
            json!({"baseline":result["baseline"],"note":"合成回滚"})
        )
        .unwrap()["rolled_back"],
        true
    );
    assert_eq!(
        serde_json::to_value(store::state(&db).unwrap()).unwrap(),
        serde_json::to_value(before).unwrap()
    );
    assert_eq!(
        db.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE name LIKE 'prd_%'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
}
#[test]
fn upgrade_stale_source_incomplete_decisions_and_post_upgrade_writes_block() {
    let (mut db, u) = legacy();
    let p = store::data_transfer(&mut db, &u, Command::UpgradePreview).unwrap();
    let old = upgrade_request(&p, "archive");
    db.execute(
        "INSERT INTO operations(id,request,response) VALUES('synthetic-change','{}','{}')",
        [],
    )
    .unwrap();
    assert_eq!(
        call(&mut db, &u, "upgrade_commit", old).unwrap_err().0,
        StatusCode::CONFLICT
    );
    let p = store::data_transfer(&mut db, &u, Command::UpgradePreview).unwrap();
    let result = call(
        &mut db,
        &u,
        "upgrade_commit",
        upgrade_request(&p, "archive"),
    )
    .unwrap();
    // Even an apparently harmless new operation must prohibit destructive rollback.
    db.execute("INSERT INTO prd_operations(id,actor_id,request,response) VALUES(?1,?2,'synthetic-change','{}')",rusqlite::params![id(),u.id]).unwrap();
    assert_eq!(
        call(
            &mut db,
            &u,
            "rollback",
            json!({"baseline":result["baseline"],"note":"不得丢数据"})
        )
        .unwrap_err()
        .0,
        StatusCode::CONFLICT
    );
    store::initialize_prd_runtime(&db).unwrap();
}
#[test]
fn failing_upgrade_is_atomic_including_schema_and_original_unchanged() {
    let (mut db, u) = legacy();
    let g = store::state(&db).unwrap().goals[0].clone();
    let mut raw = serde_json::to_value(g).unwrap();
    raw["title"] = json!("x".repeat(121));
    db.execute(
        "UPDATE goals SET data=?1 WHERE id=?2",
        rusqlite::params![raw.to_string(), raw["id"].as_str().unwrap()],
    )
    .unwrap();
    let p = store::data_transfer(&mut db, &u, Command::UpgradePreview).unwrap();
    assert!(
        call(
            &mut db,
            &u,
            "upgrade_commit",
            upgrade_request(&p, "archive")
        )
        .is_err()
    );
    assert_eq!(
        db.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE name LIKE 'prd_%'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    store::initialize(&db).unwrap();
}

#[test]
fn explicit_completed_workbook_family_is_counted_once_and_methods_are_not_inferred() {
    let mut db = new_db();
    let u = actor();
    let sheets = json!([
        sheet("目标", json!(["编号", "名称"]), json!([["G1", "轮替"]])),
        sheet(
            "家庭",
            json!([
                "编号",
                "日期",
                "时段",
                "状态",
                "主目标",
                "实际目标",
                "表现",
                "方法情况",
                "计划"
            ]),
            json!([[
                "S1",
                "2020/1/2",
                "晚上",
                "completed",
                "目标:G1",
                "目标:G1",
                "轮替三次后休息",
                "unknown",
                "一起滚球"
            ]])
        )
    ]);
    let rules = json!([goal_rule(),{"sheet":"家庭","kind":"session","fields":{"key":"编号","date":"日期","time_slot":"时段","status":"状态","main_goal":"主目标","actual_goals":"实际目标","observation":"表现","confirmation":"方法情况","plan":"计划"}}]);
    let p = upload(&mut db, &u, sheets.clone());
    let plan = map(&mut db, &u, &p, rules.clone()).unwrap();
    let r = confirm(&mut db, &u, &plan).unwrap();
    assert_eq!(r["saved"], 2);
    let saved: String = db
        .query_row("SELECT data FROM prd_sessions", [], |r| r.get(0))
        .unwrap();
    let s: Value = serde_json::from_str(&saved).unwrap();
    assert_eq!(s["status"], "completed");
    assert_eq!(s["input"]["business_date"], "2020-01-02");
    assert_eq!(s["input"]["summary"]["confirmation"], "unknown");
    assert!(s["actual_methods"].as_array().unwrap().is_empty());
    assert_eq!(s["actual_targets"].as_array().unwrap().len(), 1);
    let p = upload(&mut db, &u, sheets);
    let plan = map(&mut db, &u, &p, rules).unwrap();
    assert_eq!(confirm(&mut db, &u, &plan).unwrap()["skipped"], 2);
}
#[test]
fn upgrade_confirmed_family_and_course_preserve_historical_names_and_original_references() {
    let (mut db, u) = legacy();
    let g = store::state(&db)
        .unwrap()
        .goals
        .into_iter()
        .find(|g| g.is_trainable())
        .unwrap();
    let sid = id();
    let session = json!({"business_date":"2020-01-01","time_slot":"晚上","activities":["游戏"],"organization":"家庭","plan":"合成安排","observation":"合成表现","targets":[{"goal_id":g.id,"role":"primary","plan":"轮流","progress":"合成两轮","outcome":"continue","next_step":""}],"has_difficulty":true,"difficulty":"等待时需要帮助","has_experience":false,"experience":"","experience_kind":"task"});
    store::save_session(&mut db,&sid,serde_json::from_value(json!({"operation_id":id(),"expected_version":null,"action":"complete","session":session})).unwrap()).unwrap();
    let lesson = id();
    store::save_lesson(&mut db,&lesson,serde_json::from_value(json!({"expected_version":null,"lesson":{"business_date":"2020-01-01","course":"旧课程","teacher":"旧老师","content":"课堂活动","goals":"不用于自动关联的原文","problems":"","home_cooperation":"","links":{"targets":[{"goal_id":g.id,"role":"primary","plan":"","progress":"观察"}],"pathway_ids":[]}}})).unwrap()).unwrap();
    let p = store::data_transfer(&mut db, &u, Command::UpgradePreview).unwrap();
    let mut req = upgrade_request(&p, "completed");
    req["sessions"][0]["actual_goal_ids"] = json!([g.id]);
    let result = call(&mut db, &u, "upgrade_commit", req).unwrap();
    assert_eq!(result["receipt"]["sessions"], 1);
    assert_eq!(result["receipt"]["lessons"], 1);
    let saved: String = db
        .query_row("SELECT data FROM prd_sessions WHERE id=?1", [&sid], |r| {
            r.get(0)
        })
        .unwrap();
    let s: Value = serde_json::from_str(&saved).unwrap();
    assert_eq!(s["actual_targets"][0]["goal"]["input"]["title"], g.title);
    assert_eq!(s["input"]["summary"]["confirmation"], "unknown");
    let k: String = db
        .query_row(
            "SELECT data FROM prd_knowledge WHERE source_session_id=?1",
            [&sid],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&k).unwrap()["source_session_id"],
        sid
    );
    let l: String = db
        .query_row("SELECT data FROM prd_lessons WHERE id=?1", [&lesson], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&l).unwrap()["teacher"]["name"],
        "旧老师"
    );
    let empty = skarma_core::foundation::process_model::Read {
        view: "board".into(),
        id: String::new(),
        object_kind: String::new(),
        page: 0,
    };
    store::foundation_process(
        &mut db,
        &u,
        skarma_core::foundation::process_model::Command::Read(empty),
    )
    .unwrap();
    // Read-only views and restart must not silently invalidate the baseline.
    store::initialize_prd_runtime(&db).unwrap();
    call(
        &mut db,
        &u,
        "rollback",
        json!({"baseline":result["baseline"],"note":"合成回滚"}),
    )
    .unwrap();
    assert_eq!(store::state(&db).unwrap().sessions.len(), 1);
}

#[test]
fn late_storage_failure_rolls_back_rows_relations_receipts_and_can_retry_original_preview() {
    let mut db = new_db();
    let u = actor();
    let p = upload(
        &mut db,
        &u,
        json!([sheet(
            "目标",
            json!(["编号", "名称"]),
            json!([["G1", "合成目标"]])
        )]),
    );
    let plan = map(&mut db, &u, &p, json!([goal_rule()])).unwrap();
    db.execute_batch("CREATE TRIGGER synthetic_import_failure BEFORE INSERT ON prd_import_rows BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;").unwrap();
    assert_eq!(
        confirm(&mut db, &u, &plan).unwrap_err().0,
        StatusCode::INTERNAL_SERVER_ERROR
    );
    for table in [
        "prd_entities",
        "prd_operations",
        "prd_revisions",
        "prd_import_rows",
        "prd_import_batches",
    ] {
        assert_eq!(
            db.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    db.execute_batch("DROP TRIGGER synthetic_import_failure;")
        .unwrap();
    assert_eq!(confirm(&mut db, &u, &plan).unwrap()["saved"], 1);
}
#[test]
fn preview_owner_and_uncommitted_expiry_are_checked_without_side_effects() {
    let mut db = new_db();
    let u = actor();
    let other = actor();
    let p = upload(
        &mut db,
        &u,
        json!([sheet(
            "目标",
            json!(["编号", "名称"]),
            json!([["G1", "合成目标"]])
        )]),
    );
    assert_eq!(
        map(&mut db, &other, &p, json!([goal_rule()]))
            .unwrap_err()
            .0,
        StatusCode::UNAUTHORIZED
    );
    let plan = map(&mut db, &u, &p, json!([goal_rule()])).unwrap();
    db.execute("UPDATE prd_import_previews SET data=json_set(data,'$.expires_at','2000-01-01T00:00:00+00:00') WHERE id=?1",[p["preview_id"].as_str().unwrap()]).unwrap();
    assert_eq!(
        confirm(&mut db, &u, &plan).unwrap_err().0,
        StatusCode::CONFLICT
    );
    assert!(state(&mut db).as_array().unwrap().is_empty());
}

#[test]
fn migration_keeps_preexisting_helpful_practice_stopped_state_and_active_difficulties_without_votes()
 {
    let (mut db, u) = legacy();
    let goal = store::state(&db).unwrap().goals[0].id.clone();
    for (table, status) in [
        ("experiences", "verified"),
        ("experiences", "stopped"),
        ("difficulties", "active"),
    ] {
        let id = id();
        let k = json!({"id":id,"version":2,"source_session_id":null,"source_lesson_id":null,"content":format!("合成原有{status}"),"kind":null,"status":status,"next_review_date":null,"conclusion":"原家人观察依据","goal_ids":[goal],"pathway_ids":[]});
        db.execute(
            &format!("INSERT INTO {table}(id,source_session_id,data) VALUES(?1,NULL,?2)"),
            rusqlite::params![id, k.to_string()],
        )
        .unwrap();
    }
    let p = store::data_transfer(&mut db, &u, Command::UpgradePreview).unwrap();
    call(
        &mut db,
        &u,
        "upgrade_commit",
        upgrade_request(&p, "archive"),
    )
    .unwrap();
    let rows = db
        .prepare("SELECT data FROM prd_knowledge")
        .unwrap()
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    for raw in rows {
        let k: Value = serde_json::from_str(&raw).unwrap();
        match k["input"]["content"].as_str().unwrap() {
            "合成原有verified" => {
                assert_eq!(k["input"]["stage"], "helpful");
                assert_eq!(k["details"]["evidence"], "原家人观察依据");
            }
            "合成原有stopped" => assert_eq!(k["details"]["state"], "retired"),
            "合成原有active" => assert_eq!(k["details"]["state"], "following"),
            _ => panic!("unexpected"),
        }
    }
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM prd_feedback", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
}
#[test]
fn explicit_workbook_practice_stage_is_preserved_instead_of_forced_back_to_idea() {
    let mut db = new_db();
    let u = actor();
    let p = upload(
        &mut db,
        &u,
        json!([
            sheet("目标", json!(["编号", "名称"]), json!([["G1", "合成目标"]])),
            sheet(
                "经验",
                json!(["编号", "内容", "阶段", "状态", "依据", "关联"]),
                json!([[
                    "K1",
                    "合成既有做法",
                    "已验证",
                    "停用",
                    "家人原有观察",
                    "目标:G1"
                ]])
            )
        ]),
    );
    let plan=map(&mut db,&u,&p,json!([goal_rule(),{"sheet":"经验","kind":"experience","fields":{"key":"编号","title":"内容","stage":"阶段","state":"状态","evidence":"依据","goals":"关联"}}])).unwrap();
    confirm(&mut db, &u, &plan).unwrap();
    let raw: String = db
        .query_row("SELECT data FROM prd_knowledge", [], |r| r.get(0))
        .unwrap();
    let k: Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(k["input"]["stage"], "helpful");
    assert_eq!(k["details"]["state"], "retired");
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM prd_feedback", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[test]
fn reusing_edited_source_goal_for_new_history_requires_reconciliation_but_repeat_is_safe() {
    let mut db = new_db();
    let u = actor();
    let goals = sheet(
        "目标",
        json!(["编号", "名称"]),
        json!([["G1", "原目标定义"]]),
    );
    let p = upload(&mut db, &u, json!([goals.clone()]));
    let plan = map(&mut db, &u, &p, json!([goal_rule()])).unwrap();
    let receipt = confirm(&mut db, &u, &plan).unwrap();
    let gid = receipt["sources"][0]["target_id"]
        .as_str()
        .unwrap()
        .to_string();
    store::write_foundation(&mut db,&u,serde_json::from_value(json!({"id":gid,"operation_id":id(),"expected_version":1,"change_note":"明确修改定义","mutation":{"kind":"goal","value":{"title":"后来修改的目标","level":"short"}}})).unwrap()).unwrap();
    let p = upload(&mut db, &u, json!([goals.clone()]));
    let plan = map(&mut db, &u, &p, json!([goal_rule()])).unwrap();
    assert_eq!(confirm(&mut db, &u, &plan).unwrap()["skipped"], 1);
    let p = upload(
        &mut db,
        &u,
        json!([
            goals,
            sheet(
                "方法",
                json!(["编号", "名称", "关联"]),
                json!([["M1", "新方法", "目标:G1"]])
            )
        ]),
    );
    let r=map(&mut db,&u,&p,json!([goal_rule(),{"sheet":"方法","kind":"method","fields":{"key":"编号","title":"名称","goals":"关联"}}])).unwrap_err();
    assert_eq!(r.0, StatusCode::CONFLICT);
    assert!(r.1.contains("已修订"));
    assert_eq!(
        db.query_row(
            "SELECT COUNT(*) FROM prd_entities WHERE kind='method'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
}

fn starter_keys() -> Vec<String> {
    let pack: Value =
        serde_json::from_str(include_str!("../../../data/seeds/family-starter-v1.json")).unwrap();
    let catalog: Value =
        serde_json::from_str(include_str!("../../../data/seeds/initial-catalog.json")).unwrap();
    pack["goals"]
        .as_array()
        .unwrap()
        .iter()
        .chain(pack["methods"].as_array().unwrap())
        .map(|r| r["key"].as_str().unwrap().to_string())
        .chain(
            catalog["catalog"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| format!("catalog.{}", r["key"].as_str().unwrap())),
        )
        .collect()
}
fn starter_preview(db: &mut Connection, u: &User, keys: &[String]) -> Value {
    call(db, u, "starter_preview", json!({"keys":keys})).unwrap()
}
fn starter_confirm(db: &mut Connection, u: &User, p: &Value) -> skarma_core::store::Result<Value> {
    call(
        db,
        u,
        "commit",
        json!({"preview_id":p["preview_id"],"digest":p["digest"]}),
    )
}
#[test]
fn starter_preview_is_read_only_and_retries_preserve_edits_renames_and_inactive_options() {
    let mut db = new_db();
    let u = actor();
    let keys = starter_keys();
    assert_eq!(keys.len(), 26);
    let p = starter_preview(&mut db, &u, &keys);
    assert!(
        store::foundation_snapshot(&mut db)
            .unwrap()
            .goals
            .is_empty()
    );
    let receipt = starter_confirm(&mut db, &u, &p).unwrap();
    assert_eq!(receipt["saved"], 26);
    assert_eq!(starter_confirm(&mut db, &u, &p).unwrap(), receipt);
    let s = store::foundation_snapshot(&mut db).unwrap();
    assert_eq!(
        (s.goals.len(), s.methods.len(), s.catalog.len()),
        (6, 10, 10)
    );
    assert!(
        s.lessons.is_empty()
            && s.sessions.is_empty()
            && s.knowledge.is_empty()
            && s.actions.is_empty()
    );
    assert!(
        s.goals
            .iter()
            .all(|g| g.input.level == skarma_core::model::Level::Long)
    );
    let g = s
        .goals
        .iter()
        .find(|g| g.input.title == "内在动机与主动探索")
        .unwrap();
    let mut v = serde_json::to_value(&g.input).unwrap();
    v["title"] = json!("家人调整后的长期方向");
    store::write_foundation(&mut db,&u,serde_json::from_value(json!({"id":g.id,"operation_id":id(),"expected_version":1,"change_note":"家人调整","mutation":{"kind":"goal","value":v}})).unwrap()).unwrap();
    let c = &s.catalog[0];
    let mut v = serde_json::to_value(&c.input).unwrap();
    v["active"] = json!(false);
    v["name"] = json!("家人调整后的选项");
    store::write_foundation(&mut db,&u,serde_json::from_value(json!({"id":c.id,"operation_id":id(),"expected_version":1,"change_note":"停用","mutation":{"kind":"catalog","value":v}})).unwrap()).unwrap();
    let before = serde_json::to_value(store::foundation_snapshot(&mut db).unwrap()).unwrap();
    let p = starter_preview(&mut db, &u, &keys);
    assert!(
        p["rows"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["skipped"] == true)
    );
    let r = starter_confirm(&mut db, &u, &p).unwrap();
    assert_eq!(r["saved"], 0);
    assert_eq!(r["skipped"], 26);
    assert_eq!(
        serde_json::to_value(store::foundation_snapshot(&mut db).unwrap()).unwrap(),
        before
    );
}
#[test]
fn starter_reuses_only_previewed_exact_matches_and_keeps_old_content_and_state() {
    let mut db = new_db();
    let u = actor();
    for (kind, value) in [
        (
            "goal",
            json!({"title":"内在动机与主动探索","level":"long","meaning":"原有个体化说明","state":"paused"}),
        ),
        (
            "catalog",
            json!({"kind":"teacher","name":"唐唐","active":false}),
        ),
    ] {
        store::write_foundation(&mut db,&u,serde_json::from_value(json!({"id":id(),"operation_id":id(),"expected_version":null,"mutation":{"kind":kind,"value":value}})).unwrap()).unwrap();
    }
    let before = store::foundation_snapshot(&mut db).unwrap();
    let g = before.goals[0].clone();
    let c = before.catalog[0].clone();
    let p = starter_preview(&mut db, &u, &starter_keys());
    assert_eq!(
        p["rows"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["starter_status"] == "reuse")
            .count(),
        2
    );
    let r = starter_confirm(&mut db, &u, &p).unwrap();
    assert_eq!(r["reused"], 2);
    assert_eq!(r["saved"], 24);
    let after = store::foundation_snapshot(&mut db).unwrap();
    assert_eq!(
        serde_json::to_value(after.goals.iter().find(|v| v.id == g.id)).unwrap(),
        serde_json::to_value(Some(g)).unwrap()
    );
    assert_eq!(
        serde_json::to_value(after.catalog.iter().find(|v| v.id == c.id)).unwrap(),
        serde_json::to_value(Some(c)).unwrap()
    );
}
#[test]
fn starter_is_admin_only_requires_dependencies_and_detects_stale_preview() {
    let mut db = new_db();
    let u = actor();
    let mut member = u.clone();
    member.role = AccountRole::Member;
    assert_eq!(
        call(&mut db, &member, "starter_preview", json!({"keys":["G01"]}))
            .unwrap_err()
            .0,
        StatusCode::FORBIDDEN
    );
    for keys in [
        json!([]),
        json!(["unknown"]),
        json!(["G01", "G01"]),
        json!(["M03"]),
    ] {
        assert_eq!(
            call(&mut db, &u, "starter_preview", json!({"keys":keys}))
                .unwrap_err()
                .0,
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    let p = starter_preview(&mut db, &u, &starter_keys());
    store::write_foundation(&mut db,&u,serde_json::from_value(json!({"id":id(),"operation_id":id(),"expected_version":null,"mutation":{"kind":"catalog","value":{"kind":"course","name":"合成并发新增","active":true}}})).unwrap()).unwrap();
    assert_eq!(
        starter_confirm(&mut db, &u, &p).unwrap_err().0,
        StatusCode::CONFLICT
    );
    assert!(
        store::foundation_snapshot(&mut db)
            .unwrap()
            .goals
            .is_empty()
    );
    assert_eq!(
        call(
            &mut db,
            &u,
            "map",
            json!({"preview_id":p["preview_id"],"namespace":"ordinary","rules":[]})
        )
        .unwrap_err()
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
}
#[test]
fn starter_failure_rolls_back_every_entity_and_source_marker() {
    let mut db = new_db();
    let u = actor();
    db.execute_batch("CREATE TRIGGER reject_starter BEFORE INSERT ON prd_entities WHEN NEW.kind='teacher' BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;").unwrap();
    let p = starter_preview(&mut db, &u, &starter_keys());
    assert!(starter_confirm(&mut db, &u, &p).is_err());
    let s = store::foundation_snapshot(&mut db).unwrap();
    assert!(s.goals.is_empty() && s.methods.is_empty() && s.catalog.is_empty());
    let n: i64 = db
        .query_row("SELECT count(*) FROM prd_import_rows", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 0);
    db.execute_batch("DROP TRIGGER reject_starter;").unwrap();
    assert_eq!(starter_confirm(&mut db, &u, &p).unwrap()["saved"], 26);
}

#[test]
fn starter_does_not_recreate_removed_items_or_guess_between_same_named_goals() {
    let mut db = new_db();
    let u = actor();
    let keys = starter_keys();
    let p = starter_preview(&mut db, &u, &keys);
    starter_confirm(&mut db, &u, &p).unwrap();
    let s = store::foundation_snapshot(&mut db).unwrap();
    let m = &s.methods[0];
    db.execute("DELETE FROM prd_method_goals WHERE method_id=?1", [&m.id])
        .unwrap();
    // A source receipt may outlive its entity; model this without breaking FKs.
    db.execute("DELETE FROM prd_revisions WHERE entity_id=?1", [&m.id])
        .unwrap();
    db.execute("DELETE FROM prd_entities WHERE id=?1", [&m.id])
        .unwrap();
    let p = starter_preview(&mut db, &u, &keys);
    assert!(
        p["rows"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["target_id"] == m.id
                && r["starter_status"] == "removed"
                && r["skipped"] == true)
    );
    assert_eq!(starter_confirm(&mut db, &u, &p).unwrap()["saved"], 0);
    assert_eq!(
        store::foundation_snapshot(&mut db).unwrap().methods.len(),
        9
    );
    let mut db = new_db();
    for _ in 0..2 {
        store::write_foundation(&mut db,&u,serde_json::from_value(json!({"id":id(),"operation_id":id(),"expected_version":null,"mutation":{"kind":"goal","value":{"title":"内在动机与主动探索","level":"long"}}})).unwrap()).unwrap();
    }
    assert_eq!(
        call(&mut db, &u, "starter_preview", json!({"keys":["G05"]}))
            .unwrap_err()
            .0,
        StatusCode::CONFLICT
    );

    let mut db = new_db();
    store::write_foundation(&mut db,&u,serde_json::from_value(json!({"id":id(),"operation_id":id(),"expected_version":null,"mutation":{"kind":"goal","value":{"title":"内在动机与主动探索","level":"short"}}})).unwrap()).unwrap();
    assert_eq!(
        call(&mut db, &u, "starter_preview", json!({"keys":["G05"]}))
            .unwrap_err()
            .0,
        StatusCode::CONFLICT
    );
}
