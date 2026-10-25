//! Disposable synthetic data only: intervention transactions and PRD statistical contract.
use rusqlite::Connection;
use serde_json::{Value, json};
use skarma_api::store;
use skarma_core::{
    auth::{AccountRole, User},
    foundation::{coverage, model::*, process_model::Details, session_model::*},
    model::Level,
};
use uuid::Uuid;
fn id() -> String {
    Uuid::new_v4().to_string()
}
fn actor() -> User {
    User {
        id: id(),
        username: "test".into(),
        display_name: "测试家人".into(),
        role: AccountRole::Member,
        active: true,
    }
}
fn put(db: &mut Connection, u: &User, mutation: Mutation) -> Saved {
    store::write_foundation(
        db,
        u,
        skarma_core::foundation::model::Write {
            provided_fields: None,
            method_change: None,
            method_create_mode: None,
            id: id(),
            operation_id: id(),
            expected_version: None,
            change_note: "".into(),
            mutation,
        },
    )
    .unwrap()
}
fn fixture() -> (Connection, User, Input, String, String) {
    let mut db = Connection::open_in_memory().unwrap();
    store::initialize_foundation(&db).unwrap();
    let u = actor();
    let goal = put(
        &mut db,
        &u,
        Mutation::Goal(
            serde_json::from_value(json!({"title":"测试本领","level":"short"})).unwrap(),
        ),
    );
    let method = put(
        &mut db,
        &u,
        Mutation::Method(
            serde_json::from_value(json!({"title":"测试方法","goal_ids":[goal.id()]})).unwrap(),
        ),
    );
    let input=serde_json::from_value(json!({"business_date":"2020-01-01","time_slot":"晚上","plan":"测试安排","targets":[{"goal_id":goal.id(),"role":"main"}],"methods":[{"id":method.id(),"version":1}]})).unwrap();
    (db, u, input, goal.id().into(), method.id().into())
}
fn cmd(
    id: &str,
    v: Option<u64>,
    action: Action,
    input: Option<Input>,
) -> skarma_core::foundation::session_model::Write {
    skarma_core::foundation::session_model::Write {
        id: id.into(),
        operation_id: Uuid::new_v4().to_string(),
        expected_version: v,
        action,
        input,
        change_note: "测试修订".into(),
    }
}
fn summarized(mut input: Input, g: &str, confirmation: Confirmation) -> Input {
    input.summary = Some(Summary {
        targets: vec![ActualTarget {
            goal_id: g.into(),
            performed: true,
            observation: "测试实际表现".into(),
        }],
        confirmation,
        ..Default::default()
    });
    input
}
#[test]
fn drafts_do_not_count_and_completion_requires_actual_targets_and_method_confirmation() {
    let (mut db, u, input, g, _) = fixture();
    let identifier = id();
    let draft = store::write_foundation_session(
        &mut db,
        &u,
        cmd(&identifier, None, Action::SaveDraft, Some(input.clone())),
    )
    .unwrap();
    assert_eq!(draft.status, "planned");
    assert!(draft.actual_methods.is_empty());
    let incomplete = summarized(input.clone(), &g, Confirmation::Unconfirmed);
    assert!(
        store::write_foundation_session(
            &mut db,
            &u,
            cmd(&identifier, Some(1), Action::Complete, Some(incomplete))
        )
        .is_err()
    );
    let data = summarized(input, &g, Confirmation::Unknown);
    let operation = cmd(&identifier, Some(1), Action::Complete, Some(data));
    let first = store::write_foundation_session(&mut db, &u, operation.clone()).unwrap();
    let retry = store::write_foundation_session(&mut db, &u, operation).unwrap();
    assert_eq!(retry.version, first.version);
    assert!(first.actual_methods.is_empty());
    assert_eq!(first.planned_methods.len(), 1);
    assert!(
        store::write_foundation_session(
            &mut db,
            &u,
            cmd(&identifier, Some(1), Action::Complete, Some(first.input))
        )
        .is_err()
    );
    let report = store::foundation_coverage_at(
        &mut db,
        coverage::Request {
            days: 30,
            source: "family".into(),
        },
        "2020-01-01",
    )
    .unwrap();
    assert_eq!(report.records.family, 1);
    assert_eq!(report.method_unknown.family, 1);
}
#[test]
fn outputs_retry_edit_and_failure_are_atomic_and_admin_delete_preserves_knowledge() {
    let (mut db, u, input, g, _) = fixture();
    let identifier = id();
    let kid = id();
    let mut input = summarized(input, &g, Confirmation::None);
    input.summary.as_mut().unwrap().new_knowledge = vec![
        serde_json::from_value(
            json!({"id":kid,"kind":"experience","content":"测试经验","goal_ids":[g],"stage":"try"}),
        )
        .unwrap(),
    ];
    db.execute_batch("CREATE TRIGGER fail_output BEFORE INSERT ON prd_knowledge BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;").unwrap();
    let request = cmd(&identifier, None, Action::Complete, Some(input));
    assert!(store::write_foundation_session(&mut db, &u, request.clone()).is_err());
    assert!(
        store::foundation_snapshot(&mut db)
            .unwrap()
            .sessions
            .is_empty()
    );
    db.execute_batch("DROP TRIGGER fail_output").unwrap();
    let saved = store::write_foundation_session(&mut db, &u, request.clone()).unwrap();
    store::write_foundation_session(&mut db, &u, request).unwrap();
    let mut changed = saved.input;
    changed
        .summary
        .as_mut()
        .unwrap()
        .knowledge_versions
        .insert(kid.clone(), 1);
    changed.summary.as_mut().unwrap().new_knowledge[0].content = "测试经验修订".into();
    let other = actor();
    let revised = store::write_foundation_session(
        &mut db,
        &other,
        cmd(&identifier, Some(1), Action::Complete, Some(changed)),
    )
    .unwrap();
    let data = store::foundation_snapshot(&mut db).unwrap();
    assert_eq!(data.knowledge.len(), 1);
    assert_eq!(data.knowledge[0].version, 2);
    assert_eq!(data.knowledge[0].author_id, u.id);
    assert!(
        store::write_foundation_session(
            &mut db,
            &u,
            cmd(&identifier, Some(2), Action::Delete, None)
        )
        .is_err()
    );
    let admin = User {
        role: AccountRole::Admin,
        ..actor()
    };
    store::write_foundation_session(
        &mut db,
        &admin,
        cmd(&identifier, Some(revised.version), Action::Delete, None),
    )
    .unwrap();
    assert_eq!(
        store::foundation_snapshot(&mut db).unwrap().knowledge.len(),
        1
    );
}
#[test]
fn summary_difficulties_save_structured_details_atomically_and_preserve_omitted_fields() {
    let (mut db, u, input, goal, _) = fixture();
    let session_id = id();
    let first_id = id();
    let second_id = id();
    let mut input = summarized(input, &goal, Confirmation::None);
    let summary = input.summary.as_mut().unwrap();
    summary.new_knowledge = vec![
        serde_json::from_value(json!({
            "id":first_id,
            "kind":"difficulty",
            "content":"等待时离开座位",
            "goal_ids":[goal]
        }))
        .unwrap(),
        serde_json::from_value(json!({
            "id":second_id,
            "kind":"difficulty",
            "content":"换材料后把玩具推开",
            "goal_ids":[goal]
        }))
        .unwrap(),
    ];
    let first_details = Details {
        adult_response: "先暂停并陪他休息".into(),
        guess: "可能有些累了".into(),
        conditions: "换了新材料".into(),
        ..Default::default()
    };
    let second_details = Details {
        adult_response: "把材料收近并等待".into(),
        guess: "还不确定是否不熟悉".into(),
        conditions: "第一次用这套材料".into(),
        ..Default::default()
    };
    summary.knowledge_details = Some(std::collections::BTreeMap::from([
        (first_id.clone(), first_details.clone()),
        (second_id.clone(), second_details.clone()),
    ]));
    db.execute_batch(&format!(
        "CREATE TRIGGER fail_second_difficulty BEFORE INSERT ON prd_knowledge WHEN NEW.id='{}' BEGIN SELECT RAISE(ABORT,'synthetic late output failure'); END;",
        second_id
    ))
    .unwrap();
    let request = cmd(&session_id, None, Action::Complete, Some(input));
    assert!(store::write_foundation_session(&mut db, &u, request.clone()).is_err());
    let failed = store::foundation_snapshot(&mut db).unwrap();
    assert!(failed.sessions.is_empty());
    assert!(failed.knowledge.is_empty());
    db.execute_batch("DROP TRIGGER fail_second_difficulty")
        .unwrap();

    let saved = store::write_foundation_session(&mut db, &u, request.clone()).unwrap();
    let retry = store::write_foundation_session(&mut db, &u, request).unwrap();
    assert_eq!(saved.version, retry.version);
    let state = store::foundation_snapshot(&mut db).unwrap();
    assert_eq!(state.knowledge.len(), 2);
    let first = state
        .knowledge
        .iter()
        .find(|k| k.input.id == first_id)
        .unwrap();
    assert_eq!(first.input.content, "等待时离开座位");
    assert_eq!(first.details.adult_response, first_details.adult_response);
    assert_eq!(first.details.guess, first_details.guess);
    assert_eq!(first.details.conditions, first_details.conditions);
    assert_eq!(
        first.source_session_id.as_deref(),
        Some(session_id.as_str())
    );
    assert_eq!(first.input.goal_ids, vec![goal.clone()]);

    // An older client omitting structured details may edit the parent without clearing them.
    let mut omitted = saved.input;
    let summary = omitted.summary.as_mut().unwrap();
    summary.knowledge_details = None;
    summary.targets[0].observation = "只修改本次实际表现".into();
    let preserved = store::write_foundation_session(
        &mut db,
        &u,
        cmd(&session_id, Some(1), Action::Complete, Some(omitted)),
    )
    .unwrap();
    let state = store::foundation_snapshot(&mut db).unwrap();
    assert!(
        state
            .knowledge
            .iter()
            .all(|knowledge| knowledge.version == 1)
    );
    assert_eq!(
        state
            .knowledge
            .iter()
            .find(|k| k.input.id == first_id)
            .unwrap()
            .details,
        first_details
    );

    // A present entry explicitly versions only the changed structured fields.
    let mut changed = preserved.input;
    let summary = changed.summary.as_mut().unwrap();
    summary
        .knowledge_versions
        .extend([(first_id.clone(), 1), (second_id.clone(), 1)]);
    let mut revised_details = first_details.clone();
    revised_details.adult_response = "后来先回应求助，再陪他休息".into();
    summary.knowledge_details = Some(std::collections::BTreeMap::from([
        (first_id.clone(), revised_details.clone()),
        (second_id.clone(), second_details),
    ]));
    store::write_foundation_session(
        &mut db,
        &u,
        cmd(
            &session_id,
            Some(preserved.version),
            Action::Complete,
            Some(changed),
        ),
    )
    .unwrap();
    let state = store::foundation_snapshot(&mut db).unwrap();
    let first = state
        .knowledge
        .iter()
        .find(|k| k.input.id == first_id)
        .unwrap();
    let second = state
        .knowledge
        .iter()
        .find(|k| k.input.id == second_id)
        .unwrap();
    assert_eq!(first.version, 2);
    assert_eq!(first.details.adult_response, revised_details.adult_response);
    assert_eq!(second.version, 1);

    let legacy: Summary = serde_json::from_value(json!({
        "targets":[],
        "confirmation":"none",
        "new_knowledge":[]
    }))
    .unwrap();
    assert!(legacy.knowledge_details.is_none());
}

#[test]
fn action_reservation_is_exclusive_cancellation_releases_and_future_completion_is_rejected() {
    let (mut db, u, input, g, _) = fixture();
    let source = id();
    let action = id();
    let mut seed = summarized(input.clone(), &g, Confirmation::None);
    seed.summary.as_mut().unwrap().new_actions.push(
        serde_json::from_value(
            json!({"id":action,"content":"测试下一步","goal_ids":[g],"assignee_id":null}),
        )
        .unwrap(),
    );
    store::write_foundation_session(
        &mut db,
        &u,
        cmd(&source, None, Action::Complete, Some(seed)),
    )
    .unwrap();
    let mut plan = input.clone();
    plan.action_ids.push(action.clone());
    let p1 = id();
    store::write_foundation_session(
        &mut db,
        &u,
        cmd(&p1, None, Action::SaveDraft, Some(plan.clone())),
    )
    .unwrap();
    let p2 = id();
    assert!(
        store::write_foundation_session(
            &mut db,
            &u,
            cmd(&p2, None, Action::SaveDraft, Some(plan.clone()))
        )
        .is_err()
    );
    store::write_foundation_session(&mut db, &u, cmd(&p1, Some(1), Action::Cancel, None)).unwrap();
    store::write_foundation_session(
        &mut db,
        &u,
        cmd(&p2, None, Action::SaveDraft, Some(plan.clone())),
    )
    .unwrap();
    let mut done = summarized(plan, &g, Confirmation::None);
    done.summary
        .as_mut()
        .unwrap()
        .action_outcomes
        .push(ActionOutcome {
            action_id: action.clone(),
            decision: "done".into(),
        });
    let saved = store::write_foundation_session(
        &mut db,
        &u,
        cmd(&p2, Some(1), Action::Complete, Some(done)),
    )
    .unwrap();
    assert_eq!(
        store::foundation_snapshot(&mut db).unwrap().actions[0].status,
        "completed"
    );
    let admin = User {
        role: AccountRole::Admin,
        ..actor()
    };
    store::write_foundation_session(
        &mut db,
        &admin,
        cmd(&p2, Some(saved.version), Action::Delete, None),
    )
    .unwrap();
    assert_eq!(
        store::foundation_snapshot(&mut db).unwrap().actions[0].status,
        "completed"
    );
    let mut future = summarized(input, &g, Confirmation::None);
    future.business_date = "2099-01-01".into();
    assert!(
        store::write_foundation_session(
            &mut db,
            &u,
            cmd(&id(), None, Action::Complete, Some(future))
        )
        .is_err()
    );
}
#[test]
fn historical_goal_lineage_and_method_version_survive_library_changes() {
    let (mut db, u, input, g, m) = fixture();
    let parent = put(
        &mut db,
        &u,
        Mutation::Goal(
            serde_json::from_value(json!({"title":"测试中期","level":"medium"})).unwrap(),
        ),
    );
    let mut goal = match store::foundation_snapshot(&mut db)
        .unwrap()
        .goals
        .into_iter()
        .find(|v| v.id == g)
    {
        Some(g) => g,
        _ => panic!(),
    };
    goal.input.parent_id = Some(parent.id().into());
    store::write_foundation(
        &mut db,
        &u,
        skarma_core::foundation::model::Write {
            provided_fields: None,
            method_change: None,
            method_create_mode: None,
            id: g.clone(),
            operation_id: id(),
            expected_version: Some(1),
            change_note: "测试层级".into(),
            mutation: Mutation::Goal(goal.input.clone()),
        },
    )
    .unwrap();
    let mut actual = summarized(input, &g, Confirmation::Used);
    actual.summary.as_mut().unwrap().methods = vec![MethodRef {
        id: m.clone(),
        version: 1,
    }];
    let identifier = id();
    store::write_foundation_session(
        &mut db,
        &u,
        cmd(&identifier, None, Action::Complete, Some(actual.clone())),
    )
    .unwrap();
    let mut method = store::foundation_snapshot(&mut db)
        .unwrap()
        .methods
        .into_iter()
        .find(|v| v.id == m)
        .unwrap();
    method.input.description = "测试：库中的新做法".into();
    store::write_foundation(
        &mut db,
        &u,
        skarma_core::foundation::model::Write {
            provided_fields: None,
            method_change: None,
            method_create_mode: None,
            id: m.clone(),
            operation_id: id(),
            expected_version: Some(1),
            change_note: "测试方法新版".into(),
            mutation: Mutation::Method(method.input),
        },
    )
    .unwrap();
    goal.input.parent_id = None;
    store::write_foundation(
        &mut db,
        &u,
        skarma_core::foundation::model::Write {
            provided_fields: None,
            method_change: None,
            method_create_mode: None,
            id: g.clone(),
            operation_id: id(),
            expected_version: Some(2),
            change_note: "测试换上级".into(),
            mutation: Mutation::Goal(goal.input),
        },
    )
    .unwrap();
    let changed = store::write_foundation_session(
        &mut db,
        &u,
        cmd(&identifier, Some(1), Action::Complete, Some(actual)),
    )
    .unwrap();
    assert_eq!(changed.actual_targets[0].ancestors[0].id, parent.id());
    assert_eq!(changed.actual_methods[0].id, m);
    assert_eq!(changed.actual_methods[0].version, 1);
}

#[test]
fn prd_appendix_d_golden_coverage_preserves_boundaries_lineage_and_unknowns() {
    let mut db = Connection::open_in_memory().unwrap();
    store::initialize_foundation(&db).unwrap();
    let fixture: Value =
        serde_json::from_str(include_str!("../../../tools/fixtures/prd-coverage.json")).unwrap();
    let make_goal = |id: &str, parent: Option<&str>| -> Value {
        let level = if id == "LG" {
            Level::Long
        } else if id.starts_with('M') {
            Level::Medium
        } else {
            Level::Short
        };
        let input: GoalInput = serde_json::from_value(
            json!({"title":id,"level":level,"parent_id":parent,"attention_start":"2026-07-01"}),
        )
        .unwrap();
        json!({"id":id,"version":1,"input":input})
    };
    for (id, parent) in [
        ("LG", None),
        ("M1", Some("LG")),
        ("M2", Some("LG")),
        ("S1", Some("M1")),
        ("S2", Some("M2")),
        ("S3", Some("M2")),
        ("S4", None),
    ] {
        let g = make_goal(id, parent);
        db.execute(
            "INSERT INTO prd_entities(id,kind,version,name,data) VALUES(?1,'goal',1,?1,?2)",
            rusqlite::params![id, json!({"kind":"goal","value":g}).to_string()],
        )
        .unwrap();
    }
    let make_method = |id: &str, v: u64| -> Value {
        let input: MethodInput =
            serde_json::from_value(json!({"title":id,"goal_ids":["S1"]})).unwrap();
        json!({"id":id,"version":v,"input":input})
    };
    for id in ["A", "B"] {
        db.execute(
            "INSERT INTO prd_entities(id,kind,version,name,data) VALUES(?1,'method',1,?1,?2)",
            rusqlite::params![
                id,
                json!({"kind":"method","value":make_method(id,1)}).to_string()
            ],
        )
        .unwrap();
    }
    for r in fixture["records"].as_array().unwrap() {
        let source = r["source"].as_str().unwrap();
        if source == "moment" {
            continue;
        }
        let mut ts = vec![];
        let mut ls = vec![];
        for pair in r["targets"].as_array().unwrap() {
            let id = pair[0].as_str().unwrap();
            let parent = pair[1].as_str();
            let parents = if id.starts_with('M') {
                vec![make_goal("LG", None)]
            } else if let Some(p) = parent {
                vec![make_goal(p, Some("LG")), make_goal("LG", None)]
            } else {
                vec![]
            };
            let g = make_goal(
                id,
                if id.starts_with('M') {
                    Some("LG")
                } else {
                    parent
                },
            );
            ts.push(json!({"input":{"goal_id":id,"role":"secondary","note":"测试表现"},"goal":g,"ancestors":parents}));
            ls.push(json!({"target":{"goal_id":id,"note":""},"goal":g,"ancestors":parents}));
        }
        let ms: Vec<_> = r["methods"]
            .as_array()
            .map(|m| {
                m.iter()
                    .map(|v| {
                        let id = v.as_str().unwrap();
                        make_method(id, r["method_versions"][id].as_u64().unwrap_or(1))
                    })
                    .collect()
            })
            .unwrap_or_default();
        let focus: Vec<_> = r["focus"]
            .as_array()
            .map(|f| {
                f.iter()
                    .map(|v| make_goal(v.as_str().unwrap(), None))
                    .collect()
            })
            .unwrap_or_default();
        if source == "family" {
            let input:Input=serde_json::from_value(json!({"business_date":r["day"],"time_slot":"晚上","summary":{"confirmation":if r["methods"].is_null(){"unknown"}else{"used"},"tags":r["tags"].as_array().cloned().unwrap_or_default()}})).unwrap();
            let s:Session=serde_json::from_value(json!({"id":r["id"],"version":r["version"].as_u64().unwrap_or(1),"status":r["status"],"creator_id":"synthetic","creator_name":"测试","created_at":"2026-10-24T00:00:00Z","input":input,"planned_targets":[],"actual_targets":ts,"planned_methods":[],"actual_methods":ms,"focus_goals":focus,"lesson_sources":[],"action_snapshots":[]})).unwrap();
            db.execute(
                "INSERT INTO prd_sessions VALUES(?1,?2,?3)",
                rusqlite::params![s.id, s.version as i64, serde_json::to_string(&s).unwrap()],
            )
            .unwrap();
        } else {
            let input:skarma_core::foundation::lesson_model::ObservationInput=serde_json::from_value(json!({"business_date":r["day"],"course_id":r["course"],"teacher_id":r["teacher"]})).unwrap();
            let l:skarma_core::foundation::lesson_model::Lesson=serde_json::from_value(json!({"id":r["id"],"version":1,"observation_version":1,"organization_version":1,"created_at":"2026-10-24T00:00:00Z","creator_id":"test","creator_name":"测试","status":r["status"],"marked_error":r["marked_error"].as_bool().unwrap_or(false),"observation":input,"course":{"id":r["course"],"version":1,"kind":"course","name":"测试课程"},"teacher":{"id":r["teacher"],"version":1,"kind":"teacher","name":r["teacher_name"]},"organization":{"seen_observation_version":if r["methods"].is_null(){0}else{1},"targets":ls,"methods":ms,"focus_goals":focus,"processed_request_generation":0,"state":"pending","claimed_by":null,"claimed_name":null,"outcome":""},"request_generation":0,"requested_by":null,"request_name":null,"request_message":"","acknowledgements":[]})).unwrap();
            db.execute(
                "INSERT INTO prd_lessons VALUES(?1,1,?2)",
                rusqlite::params![l.id, serde_json::to_string(&l).unwrap()],
            )
            .unwrap();
        }
    }
    let expected = [
        (7, "family", vec![3, 2, 1, 2, 0, 2, 3]),
        (7, "lesson", vec![4, 2, 0, 1, 0, 2, 1]),
        (7, "combined", vec![7, 4, 1, 3, 0, 4, 4]),
        (30, "family", vec![4, 2, 2, 2, 0, 3, 3]),
        (30, "lesson", vec![5, 2, 1, 1, 0, 3, 1]),
        (30, "combined", vec![9, 4, 3, 3, 0, 6, 4]),
        (90, "family", vec![5, 3, 2, 2, 0, 4, 3]),
        (90, "lesson", vec![5, 2, 1, 1, 0, 3, 1]),
        (90, "combined", vec![10, 5, 3, 3, 0, 7, 4]),
    ];
    let selected = |family, lesson, source: &str| match source {
        "family" => family,
        "lesson" => lesson,
        _ => family + lesson,
    };
    for (days, source, want) in expected {
        let r = store::foundation_coverage_at(
            &mut db,
            coverage::Request {
                days,
                source: source.into(),
            },
            "2026-10-24",
        )
        .unwrap();
        assert_eq!(
            r.start,
            match days {
                7 => "2026-10-18",
                30 => "2026-09-25",
                _ => "2026-07-27",
            }
        );
        assert_eq!(r.end, "2026-10-24");
        let mut got = vec![r.records.selected];
        for id in ["S1", "S2", "S3", "S4", "M1", "M2"] {
            got.push(r.goals.iter().find(|g| g.id == id).unwrap().counts.selected);
        }
        assert_eq!(got, want, "D.1 coverage row: {days} {source}");

        for (id, family, lesson, combined) in [
            (
                "S1",
                Some("2026-10-24"),
                Some("2026-10-24"),
                Some("2026-10-24"),
            ),
            (
                "S2",
                Some("2026-10-18"),
                Some("2026-09-30"),
                Some("2026-10-18"),
            ),
            (
                "S3",
                Some("2026-10-24"),
                Some("2026-10-24"),
                Some("2026-10-24"),
            ),
            ("S4", Some("2026-07-26"), None, Some("2026-07-26")),
            (
                "M1",
                Some("2026-10-24"),
                Some("2026-10-24"),
                Some("2026-10-24"),
            ),
            (
                "M2",
                Some("2026-10-24"),
                Some("2026-10-24"),
                Some("2026-10-24"),
            ),
        ] {
            let want = match source {
                "family" => family,
                "lesson" => lesson,
                _ => combined,
            };
            assert_eq!(
                r.goals
                    .iter()
                    .find(|goal| goal.id == id)
                    .unwrap()
                    .last_date
                    .as_deref(),
                want,
                "D.1 last date: {id} {source}"
            );
        }

        let (a_family, a_lesson, b_family, b_lesson, below_family, below_lesson) = match days {
            7 => (1, 0, 1, 3, 3, 3),
            30 => (1, 1, 2, 3, 4, 4),
            _ => (2, 1, 2, 3, 5, 4),
        };
        let a = &r.methods["A"];
        let b = &r.methods["B"];
        assert_eq!(
            (a.family, a.lesson, a.selected),
            (a_family, a_lesson, selected(a_family, a_lesson, source)),
            "D.2 method A: {days} {source}"
        );
        assert_eq!(
            (b.family, b.lesson, b.selected),
            (b_family, b_lesson, selected(b_family, b_lesson, source)),
            "D.2 method B: {days} {source}"
        );
        assert_eq!(
            (
                r.method_unknown.family,
                r.method_unknown.lesson,
                r.method_unknown.selected
            ),
            (1, 1, selected(1, 1, source)),
            "D.2 unknown methods: {days} {source}"
        );

        let lg = r.goals.iter().find(|g| g.id == "LG").unwrap();
        assert_eq!(
            (lg.focus.family, lg.focus.lesson, lg.focus.selected),
            (1, 1, selected(1, 1, source)),
            "D.2 explicit long focus: {days} {source}"
        );
        assert_eq!(
            (
                lg.descendants.family,
                lg.descendants.lesson,
                lg.descendants.selected
            ),
            (
                below_family,
                below_lesson,
                selected(below_family, below_lesson, source)
            ),
            "D.2 long descendant evidence: {days} {source}"
        );

        assert_eq!(r.tags.get("游戏"), Some(&2), "D.2 game tags: {days}");
        assert_eq!(
            r.tags.get("阅读"),
            Some(if days == 7 { &1 } else { &2 }),
            "D.2 reading tags: {days}"
        );
        assert_eq!(
            r.tags.get("生活自理").copied().unwrap_or(0),
            if days == 90 { 1 } else { 0 },
            "D.2 self-care tags: {days}"
        );
        assert_eq!(r.unclassified, 1, "D.2 unclassified: {days}");

        let heatmap: Vec<_> = r
            .heatmap
            .iter()
            .map(|(day, count)| (day.as_str(), *count))
            .collect();
        assert_eq!(
            heatmap,
            match days {
                7 => vec![("2026-10-18", 1), ("2026-10-24", 2)],
                30 => vec![("2026-09-30", 1), ("2026-10-18", 1), ("2026-10-24", 2),],
                _ => vec![
                    ("2026-09-20", 1),
                    ("2026-09-30", 1),
                    ("2026-10-18", 1),
                    ("2026-10-24", 2),
                ],
            },
            "D.2 family heatmap: {days}"
        );
    }
}

#[test]
fn development_v2_upgrade_preserves_lesson_knowledge_json_and_source_links() {
    let mut db = Connection::open_in_memory().unwrap();
    db.execute_batch(include_str!("../../core/src/foundation/schema.sql"))
        .unwrap();
    db.execute_batch(include_str!("../../core/src/foundation/lessons_v2.sql"))
        .unwrap();
    let u = actor();
    let g = put(
        &mut db,
        &u,
        Mutation::Goal(
            serde_json::from_value(json!({"title":"测试旧开发库本领","level":"short"})).unwrap(),
        ),
    );
    let c = put(
        &mut db,
        &u,
        Mutation::Catalog(
            serde_json::from_value(json!({"kind":"course","name":"测试旧课程","active":true}))
                .unwrap(),
        ),
    );
    let t = put(
        &mut db,
        &u,
        Mutation::Catalog(
            serde_json::from_value(json!({"kind":"teacher","name":"测试旧老师","active":true}))
                .unwrap(),
        ),
    );
    let lid = id();
    let lesson=store::write_foundation_lesson(&mut db,&u,serde_json::from_value(json!({"id":lid,"operation_id":id(),"expected_version":null,"action":{"kind":"observation","value":{"business_date":"2020-01-01","course_id":c.id(),"teacher_id":t.id(),"notes":"测试原观察"}}})).unwrap()).unwrap();
    let kid = id();
    store::write_foundation_lesson(&mut db,&u,serde_json::from_value(json!({"id":lid,"operation_id":id(),"expected_version":lesson.organization_version,"action":{"kind":"organization","value":{"seen_observation_version":1,"new_knowledge":[{"id":kid,"kind":"difficulty","content":"测试旧困难","goal_ids":[g.id()]}]}}})).unwrap()).unwrap();
    // Remove newly optional fields to reproduce a v2 row generated by the prior code.
    db.execute("UPDATE prd_knowledge SET data=json_remove(data,'$.source_session_id','$.source_session_version','$.source_summary')",[]).unwrap();
    let before: String = db
        .query_row("SELECT data FROM prd_knowledge", [], |r| r.get(0))
        .unwrap();
    store::initialize_foundation(&db).unwrap();
    let after: String = db
        .query_row("SELECT data FROM prd_knowledge", [], |r| r.get(0))
        .unwrap();
    assert_eq!(before, after);
    let state = store::foundation_snapshot(&mut db).unwrap();
    assert_eq!(state.lessons.len(), 1);
    assert_eq!(
        state.knowledge[0].source_lesson_id.as_deref(),
        Some(lid.as_str())
    );
    assert!(state.knowledge[0].source_session_id.is_none());
    assert_eq!(
        state.knowledge[0]
            .source_observation
            .as_ref()
            .unwrap()
            .notes,
        "测试原观察"
    );
    assert_eq!(state.knowledge[0].input.id, kid);
    assert!(state.sessions.is_empty());
}

#[test]
fn adding_an_edited_action_requires_its_current_version_and_keeps_reservation_atomic() {
    let (mut db, u, input, g, _) = fixture();
    let source = id();
    let aid = id();
    let mut seed = summarized(input.clone(), &g, Confirmation::None);
    seed.summary.as_mut().unwrap().new_actions.push(
        serde_json::from_value(json!({"id":aid,"content":"原下一步","goal_ids":[g]})).unwrap(),
    );
    store::write_foundation_session(
        &mut db,
        &u,
        cmd(&source, None, Action::Complete, Some(seed)),
    )
    .unwrap();
    store::foundation_process(&mut db,&u,serde_json::from_value(json!({"kind":"action","value":{"id":aid,"operation_id":id(),"expected_version":1,"input":{"id":aid,"content":"核对后的下一步","goal_ids":[g]},"state":"pending","note":"调整实施安排"}})).unwrap()).unwrap();
    let mut plan = input.clone();
    plan.action_ids.push(aid.clone());
    plan.action_versions.insert(aid.clone(), 1);
    let pid = id();
    assert_eq!(
        store::write_foundation_session(
            &mut db,
            &u,
            cmd(&pid, None, Action::SaveDraft, Some(plan.clone()))
        )
        .unwrap_err()
        .0,
        axum::http::StatusCode::CONFLICT
    );
    let actions: serde_json::Value = store::foundation_process(
        &mut db,
        &u,
        serde_json::from_value(json!({"kind":"read","value":{"view":"board"}})).unwrap(),
    )
    .unwrap();
    assert_eq!(actions["inbox"]["actions"][0]["status"], "pending");
    assert_eq!(
        db.query_row(
            "SELECT COUNT(*) FROM prd_sessions WHERE id=?1",
            [&pid],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    plan.action_versions.insert(aid.clone(), 2);
    let request = cmd(&pid, None, Action::SaveDraft, Some(plan));
    let saved = store::write_foundation_session(&mut db, &u, request.clone()).unwrap();
    assert_eq!(saved.action_snapshots[0].version, 2);
    assert_eq!(saved.action_snapshots[0].input.content, "核对后的下一步");
    assert_eq!(
        store::write_foundation_session(&mut db, &u, request)
            .unwrap()
            .version,
        1
    );
    // Reservation advances the live action, but saving the same owning plan remains valid.
    let mut next = saved.input;
    next.plan = "继续补充安排".into();
    assert_eq!(
        store::write_foundation_session(
            &mut db,
            &u,
            cmd(&pid, Some(1), Action::SaveDraft, Some(next))
        )
        .unwrap()
        .version,
        2
    );
}
