use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use rusqlite::Connection;
use skarma_api::{Database, model::*, router, store};
use std::sync::{Arc, Mutex};
use tower::ServiceExt;
use uuid::Uuid;

fn database() -> Connection {
    let mut db = Connection::open_in_memory().unwrap();
    store::initialize(&db).unwrap();
    store::seed_demo(&mut db).unwrap();
    db
}

fn input(db: &Connection) -> SessionInput {
    let goals = store::state(db).unwrap().goals;
    let targets = goals
        .iter()
        .filter(|g| g.level == Level::Short && g.status == GoalStatus::Active)
        .enumerate()
        .map(|(i, g)| TargetInput {
            goal_id: g.id.clone(),
            role: if i == 0 {
                Role::Primary
            } else {
                Role::Secondary
            },
            plan: "示例计划".into(),
            progress: String::new(),
            outcome: Some(Outcome::Continue),
            next_step: String::new(),
        })
        .collect();
    SessionInput {
        business_date: "2026-10-08".into(),
        time_slot: "上午".into(),
        activities: vec!["游戏".into()],
        organization: "居家".into(),
        plan: "测试计划".into(),
        observation: "测试观察".into(),
        targets,
        has_difficulty: true,
        difficulty: "测试困难".into(),
        has_experience: true,
        experience: "测试经验".into(),
        experience_kind: "task".into(),
    }
}

fn request(input: SessionInput, version: Option<u64>, action: &str) -> SaveSession {
    SaveSession {
        operation_id: Uuid::new_v4().to_string(),
        expected_version: version,
        action: action.into(),
        session: input,
    }
}

fn copy<T: serde::Serialize + serde::de::DeserializeOwned>(value: &T) -> T {
    serde_json::from_str(&serde_json::to_string(value).unwrap()).unwrap()
}

#[test]
fn incomplete_draft_is_saved_but_completion_is_rejected_without_mutation() {
    let mut db = database();
    let id = Uuid::new_v4().to_string();
    let mut data = input(&db);
    data.targets.clear();
    data.activities.clear();
    let saved =
        store::save_session(&mut db, &id, request(data.clone(), None, "save_draft")).unwrap();
    assert_eq!(saved.session.version, 1);
    assert_eq!(
        store::save_session(&mut db, &id, request(data, Some(1), "complete"))
            .unwrap_err()
            .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let state = store::state(&db).unwrap();
    assert_eq!(state.sessions[0].version, 1);
    assert!(state.difficulties.is_empty());
    assert!(state.experiences.is_empty());
}

#[test]
fn completion_preserves_id_and_snapshot_and_generates_children_once() {
    let mut db = database();
    let id = Uuid::new_v4().to_string();
    let mut data = input(&db);
    let first =
        store::save_session(&mut db, &id, request(data.clone(), None, "save_draft")).unwrap();
    data.targets[0].outcome = Some(Outcome::Archive);
    let command = request(data, Some(1), "complete");
    let second = store::save_session(&mut db, &id, copy(&command)).unwrap();
    let retry = store::save_session(&mut db, &id, command).unwrap();
    assert_eq!(first.session.id, second.session.id);
    assert_eq!(second.session.version, 2);
    assert_eq!(second.difficulty_id, retry.difficulty_id);
    assert_eq!(second.experience_id, retry.experience_id);
    let state = store::state(&db).unwrap();
    assert_eq!(state.sessions.len(), 1);
    assert_eq!(state.difficulties.len(), 1);
    assert_eq!(state.experiences.len(), 1);
    assert_eq!(state.difficulties[0].source_session_id, id);
    assert_eq!(state.experiences[0].status, "pending");
    let archived = &second.session.input.targets[0].goal_id;
    assert_eq!(
        state
            .goals
            .iter()
            .find(|g| &g.id == archived)
            .unwrap()
            .status,
        GoalStatus::Archived
    );
    assert_eq!(
        second.session.target_snapshots[0].goal_snapshot.status,
        GoalStatus::Active
    );
}

#[test]
fn operation_id_cannot_be_reused_for_another_body_or_record() {
    let mut db = database();
    let id = Uuid::new_v4().to_string();
    let command = request(input(&db), None, "save_draft");
    store::save_session(&mut db, &id, copy(&command)).unwrap();
    let mut changed: SaveSession = copy(&command);
    changed.session.plan = "不同内容".into();
    assert_eq!(
        store::save_session(&mut db, &id, changed).unwrap_err().0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        store::save_session(&mut db, &Uuid::new_v4().to_string(), command)
            .unwrap_err()
            .0,
        StatusCode::CONFLICT
    );
    assert_eq!(store::state(&db).unwrap().sessions.len(), 1);
}

#[test]
fn stale_session_version_does_not_overwrite() {
    let mut db = database();
    let id = Uuid::new_v4().to_string();
    let data = input(&db);
    store::save_session(&mut db, &id, request(data.clone(), None, "save_draft")).unwrap();
    let mut edited = data.clone();
    edited.plan = "另一人的计划".into();
    store::save_session(&mut db, &id, request(edited, Some(1), "save_draft")).unwrap();
    assert_eq!(
        store::save_session(&mut db, &id, request(data, Some(1), "complete"))
            .unwrap_err()
            .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        store::state(&db).unwrap().sessions[0].input.plan,
        "另一人的计划"
    );
}

#[test]
fn changed_goal_blocks_stale_completion_without_partial_archiving() {
    let mut db = database();
    let one = Uuid::new_v4().to_string();
    let two = Uuid::new_v4().to_string();
    let data = input(&db);
    store::save_session(&mut db, &one, request(data.clone(), None, "save_draft")).unwrap();
    let mut other = data.clone();
    other.targets[1].outcome = Some(Outcome::Archive);
    store::save_session(&mut db, &two, request(other, None, "complete")).unwrap();
    let mut stale = data.clone();
    stale.targets[0].outcome = Some(Outcome::Archive);
    assert_eq!(
        store::save_session(&mut db, &one, request(stale, Some(1), "complete"))
            .unwrap_err()
            .0,
        StatusCode::CONFLICT
    );
    let state = store::state(&db).unwrap();
    assert_eq!(
        state
            .goals
            .iter()
            .find(|g| g.id == data.targets[0].goal_id)
            .unwrap()
            .status,
        GoalStatus::Active
    );
    assert_eq!(
        state.sessions.iter().find(|s| s.id == one).unwrap().version,
        1
    );
    assert_eq!(state.difficulties.len(), 1);
}

#[test]
fn invalid_roles_and_missing_details_are_rejected() {
    let mut db = database();
    let original = input(&db);
    let mut duplicate = original.clone();
    duplicate.targets[1].role = Role::Primary;
    let mut missing_secondary = original.clone();
    missing_secondary.targets.pop();
    let mut duplicate_goal = original.clone();
    duplicate_goal.targets[1].goal_id = duplicate_goal.targets[0].goal_id.clone();
    let mut invalid_review = original.clone();
    invalid_review.targets[1].role = Role::Review;
    let mut missing_detail = original.clone();
    missing_detail.difficulty.clear();
    for data in [
        duplicate,
        missing_secondary,
        duplicate_goal,
        invalid_review,
        missing_detail,
    ] {
        assert_eq!(
            store::save_session(
                &mut db,
                &Uuid::new_v4().to_string(),
                request(data, None, "complete")
            )
            .unwrap_err()
            .0,
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    assert!(store::state(&db).unwrap().sessions.is_empty());
}

#[test]
fn archived_review_is_valid_and_does_not_change_current_goals() {
    let mut db = database();
    let mut data = input(&db);
    let archived = store::state(&db)
        .unwrap()
        .goals
        .into_iter()
        .find(|g| g.status == GoalStatus::Archived)
        .unwrap();
    data.targets.push(TargetInput {
        goal_id: archived.id.clone(),
        role: Role::Review,
        plan: String::new(),
        progress: String::new(),
        outcome: Some(Outcome::Continue),
        next_step: String::new(),
    });
    let saved = store::save_session(
        &mut db,
        &Uuid::new_v4().to_string(),
        request(data, None, "complete"),
    )
    .unwrap();
    assert_eq!(
        saved.session.target_snapshots[2].goal_snapshot.status,
        GoalStatus::Archived
    );
}

#[test]
fn failing_child_insert_rolls_back_session_goal_changes_and_operation() {
    let mut db = database();
    let id = Uuid::new_v4().to_string();
    let mut data = input(&db);
    store::save_session(&mut db, &id, request(data.clone(), None, "save_draft")).unwrap();
    data.targets[0].outcome = Some(Outcome::Archive);
    let command = request(data, Some(1), "complete");
    let op_id = command.operation_id.clone();
    db.execute_batch("CREATE TRIGGER fail_difficulty BEFORE INSERT ON difficulties BEGIN SELECT RAISE(ABORT,'test failure'); END;").unwrap();
    assert_eq!(
        store::save_session(&mut db, &id, copy(&command))
            .unwrap_err()
            .0,
        StatusCode::INTERNAL_SERVER_ERROR
    );
    let state = store::state(&db).unwrap();
    assert_eq!(state.sessions[0].version, 1);
    assert_eq!(state.sessions[0].status, "draft");
    assert!(state.difficulties.is_empty());
    assert!(state.experiences.is_empty());
    assert!(
        state
            .goals
            .iter()
            .filter(|g| g.title != "复习：整理书包")
            .all(|g| g.status == GoalStatus::Active)
    );
    let count: i64 = db
        .query_row(
            "SELECT count(*) FROM operations WHERE id=?1",
            [op_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
    db.execute_batch("DROP TRIGGER fail_difficulty;").unwrap();
    assert!(store::save_session(&mut db, &id, command).is_ok());
}

#[test]
fn follow_up_requires_evidence_and_detects_stale_versions() {
    let mut db = database();
    let command = request(input(&db), None, "complete");
    let result = store::save_session(&mut db, &Uuid::new_v4().to_string(), command).unwrap();
    let id = result.experience_id.unwrap();
    let update = |conclusion: &str, expected_version| UpdateFollowUp {
        expected_version,
        status: "verified".into(),
        conclusion: conclusion.into(),
        next_review_date: None,
    };
    assert_eq!(
        store::update_follow_up(&mut db, "experiences", &id, update("", 1))
            .unwrap_err()
            .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        store::update_follow_up(&mut db, "experiences", &id, update("测试验证依据", 1))
            .unwrap()
            .version,
        2
    );
    assert_eq!(
        store::update_follow_up(&mut db, "experiences", &id, update("旧修改", 1))
            .unwrap_err()
            .0,
        StatusCode::CONFLICT
    );
}

#[test]
fn invalid_dates_and_parent_levels_are_rejected() {
    let mut db = database();
    let mut data = input(&db);
    data.business_date = "2026-02-30".into();
    assert_eq!(
        store::save_session(
            &mut db,
            &Uuid::new_v4().to_string(),
            request(data, None, "save_draft")
        )
        .unwrap_err()
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let long = store::state(&db)
        .unwrap()
        .goals
        .into_iter()
        .find(|g| g.level == Level::Long)
        .unwrap();
    let new = NewGoal {
        id: Uuid::new_v4().to_string(),
        title: "错误层级".into(),
        level: Level::Short,
        parent_id: Some(long.id),
        area: String::new(),
        criteria: String::new(),
    };
    assert_eq!(
        store::create_goal(&mut db, new).unwrap_err().0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(store::state(&db).unwrap().goals.len(), 5);
}

#[tokio::test]
async fn concurrent_http_updates_only_accept_one_writer() {
    let mut db = database();
    let id = Uuid::new_v4().to_string();
    let data = input(&db);
    store::save_session(&mut db, &id, request(data.clone(), None, "save_draft")).unwrap();
    let app = router(
        Database(Arc::new(Mutex::new(db))),
        "http://localhost:8081".parse().unwrap(),
        "/tmp/skarma-no-assets",
    );
    let make = |plan: &str| {
        let mut edited = data.clone();
        edited.plan = plan.into();
        Request::builder()
            .method("PUT")
            .uri(format!("/api/sessions/{id}"))
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&request(edited, Some(1), "save_draft")).unwrap(),
            ))
            .unwrap()
    };
    let (first, second) = tokio::join!(
        app.clone().oneshot(make("甲")),
        app.clone().oneshot(make("乙"))
    );
    let mut statuses = [first.unwrap().status(), second.unwrap().status()];
    statuses.sort();
    assert_eq!(statuses, [StatusCode::OK, StatusCode::CONFLICT]);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/state")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let state: AppState =
        serde_json::from_slice(&to_bytes(response.into_body(), 1_000_000).await.unwrap()).unwrap();
    assert_eq!(state.sessions[0].version, 2);
    assert!(state.difficulties.is_empty());
}
