use axum::http::StatusCode;
use rusqlite::{Connection, params};
use skarma_api::{model::*, store};
use uuid::Uuid;

fn database() -> Connection {
    let mut db = Connection::open_in_memory().unwrap();
    store::initialize(&db).unwrap();
    store::seed_demo(&mut db).unwrap();
    db
}

fn edit(goal: &Goal) -> UpdateGoal {
    UpdateGoal {
        expected_version: goal.version,
        title: goal.title.clone(),
        parent_id: goal.parent_id.clone(),
        area: goal.area.clone(),
        criteria: goal.criteria.clone(),
        status: goal.status.clone(),
        progress_status: goal.progress_status.clone(),
        start_date: goal.start_date.clone(),
        due_date: goal.due_date.clone(),
        last_review_date: goal.last_review_date.clone(),
        next_review_date: goal.next_review_date.clone(),
        completion_date: goal.completion_date.clone(),
        review_notes: goal.review_notes.clone(),
        change_note: "测试复盘依据".into(),
    }
}

fn command(db: &Connection, action: &str, version: Option<u64>) -> SaveSession {
    let targets = store::state(db)
        .unwrap()
        .goals
        .into_iter()
        .filter(Goal::is_trainable)
        .take(2)
        .enumerate()
        .map(|(index, goal)| TargetInput {
            goal_id: goal.id,
            role: if index == 0 {
                Role::Primary
            } else {
                Role::Secondary
            },
            plan: String::new(),
            progress: String::new(),
            outcome: Some(Outcome::Continue),
            next_step: String::new(),
        })
        .collect();
    SaveSession {
        operation_id: Uuid::new_v4().to_string(),
        expected_version: version,
        action: action.into(),
        session: SessionInput {
            business_date: "2026-10-08".into(),
            time_slot: "上午".into(),
            activities: vec!["游戏".into()],
            organization: "居家".into(),
            plan: String::new(),
            observation: String::new(),
            targets,
            has_difficulty: false,
            difficulty: String::new(),
            has_experience: false,
            experience: String::new(),
            experience_kind: "task".into(),
        },
    }
}

#[test]
fn progress_and_archive_are_independent_and_completed_goals_can_be_reviewed() {
    let mut db = database();
    let goal = store::state(&db)
        .unwrap()
        .goals
        .into_iter()
        .find(Goal::is_trainable)
        .unwrap();
    let mut change = edit(&goal);
    change.progress_status = Some(GoalProgress::Completed);
    let completed = store::update_goal(&mut db, &goal.id, change).unwrap();
    assert_eq!(completed.status, GoalStatus::Active);
    assert!(completed.is_reviewable());
    assert!(!completed.is_trainable());
    let mut invalid = command(&db, "complete", None);
    invalid.session.targets[0].goal_id = completed.id.clone();
    assert_eq!(
        store::save_session(&mut db, &Uuid::new_v4().to_string(), invalid)
            .unwrap_err()
            .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let mut valid = command(&db, "complete", None);
    valid.session.targets.push(TargetInput {
        goal_id: completed.id.clone(),
        role: Role::Review,
        plan: String::new(),
        progress: String::new(),
        outcome: Some(Outcome::Continue),
        next_step: String::new(),
    });
    store::save_session(&mut db, &Uuid::new_v4().to_string(), valid).unwrap();
    let mut archive = edit(&completed);
    archive.status = GoalStatus::Archived;
    let archived = store::update_goal(&mut db, &completed.id, archive).unwrap();
    assert_eq!(archived.progress_status, Some(GoalProgress::Completed));
}

#[test]
fn long_goals_remain_after_internalization_and_reject_deadlines_or_archive() {
    let mut db = database();
    let goal = store::state(&db)
        .unwrap()
        .goals
        .into_iter()
        .find(|g| g.level == Level::Long)
        .unwrap();
    let mut change = edit(&goal);
    change.progress_status = Some(GoalProgress::Internalized);
    let updated = store::update_goal(&mut db, &goal.id, change).unwrap();
    assert_eq!(updated.status, GoalStatus::Active);
    assert!(!updated.is_trainable());
    assert!(!updated.is_reviewable());
    for invalid in [
        UpdateGoal {
            due_date: Some("2026-10-09".into()),
            ..edit(&updated)
        },
        UpdateGoal {
            status: GoalStatus::Archived,
            ..edit(&updated)
        },
    ] {
        assert_eq!(
            store::update_goal(&mut db, &goal.id, invalid)
                .unwrap_err()
                .0,
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    assert_eq!(store::goal_history(&db, &goal.id).unwrap().len(), 2);
}

#[test]
fn stale_edit_and_invalid_dates_preserve_goal_and_history() {
    let mut db = database();
    let goal = store::state(&db)
        .unwrap()
        .goals
        .into_iter()
        .find(Goal::is_trainable)
        .unwrap();
    let mut change = edit(&goal);
    change.next_review_date = Some("2026-10-10".into());
    change.review_notes = "观察条件有变化".into();
    let updated = store::update_goal(&mut db, &goal.id, change).unwrap();
    assert_eq!(
        store::update_goal(&mut db, &goal.id, edit(&goal))
            .unwrap_err()
            .0,
        StatusCode::CONFLICT
    );
    for invalid in [
        UpdateGoal {
            next_review_date: Some("2026-02-30".into()),
            ..edit(&updated)
        },
        UpdateGoal {
            start_date: Some("2026-10-10".into()),
            due_date: Some("2026-10-09".into()),
            ..edit(&updated)
        },
        UpdateGoal {
            change_note: " ".into(),
            ..edit(&updated)
        },
    ] {
        assert_eq!(
            store::update_goal(&mut db, &goal.id, invalid)
                .unwrap_err()
                .0,
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    let history = store::goal_history(&db, &goal.id).unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].goal.review_notes, "观察条件有变化");
    assert!(history[1].goal.review_notes.is_empty());
}

#[test]
fn resaving_plan_acknowledges_new_goal_version_but_completed_snapshot_is_immutable() {
    let mut db = database();
    let id = Uuid::new_v4().to_string();
    let draft = command(&db, "save_draft", None);
    let goal_id = draft.session.targets[0].goal_id.clone();
    store::save_session(&mut db, &id, draft).unwrap();
    let goal = store::state(&db)
        .unwrap()
        .goals
        .into_iter()
        .find(|g| g.id == goal_id)
        .unwrap();
    let changed = store::update_goal(
        &mut db,
        &goal_id,
        UpdateGoal {
            criteria: "新观察口径".into(),
            ..edit(&goal)
        },
    )
    .unwrap();
    let stale = command(&db, "complete", Some(1));
    assert_eq!(
        store::save_session(&mut db, &id, stale).unwrap_err().0,
        StatusCode::CONFLICT
    );
    let resave = command(&db, "save_draft", Some(1));
    store::save_session(&mut db, &id, resave).unwrap();
    let complete = command(&db, "complete", Some(2));
    let completed = store::save_session(&mut db, &id, complete).unwrap();
    assert_eq!(
        completed.session.target_snapshots[0].goal_snapshot.criteria,
        "新观察口径"
    );
    store::update_goal(
        &mut db,
        &goal_id,
        UpdateGoal {
            criteria: "再次修改".into(),
            ..edit(&changed)
        },
    )
    .unwrap();
    assert_eq!(
        store::state(&db).unwrap().sessions[0].target_snapshots[0]
            .goal_snapshot
            .criteria,
        "新观察口径"
    );
}

#[test]
fn training_archive_revision_is_transactional_and_references_its_source() {
    let mut db = database();
    let id = Uuid::new_v4().to_string();
    let mut completion = command(&db, "complete", None);
    completion.session.targets[0].outcome = Some(Outcome::Archive);
    completion.session.has_difficulty = true;
    completion.session.difficulty = "测试失败回滚".into();
    let goal_id = completion.session.targets[0].goal_id.clone();
    db.execute_batch("CREATE TRIGGER reject_difficulty BEFORE INSERT ON difficulties BEGIN SELECT RAISE(ABORT,'injected'); END;").unwrap();
    assert_eq!(
        store::save_session(&mut db, &id, completion).unwrap_err().0,
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(store::goal_history(&db, &goal_id).unwrap().len(), 1);
    assert!(store::state(&db).unwrap().sessions.is_empty());
    db.execute_batch("DROP TRIGGER reject_difficulty").unwrap();
    let mut retry = command(&db, "complete", None);
    retry.session.targets[0].outcome = Some(Outcome::Archive);
    store::save_session(&mut db, &id, retry).unwrap();
    let revisions = store::goal_history(&db, &goal_id).unwrap();
    assert_eq!(revisions.len(), 2);
    assert_eq!(revisions[0].source_session_id.as_deref(), Some(id.as_str()));
    assert_eq!(
        revisions[0].goal.completion_date.as_deref(),
        Some("2026-10-08")
    );
    assert_eq!(revisions[1].goal.status, GoalStatus::Active);
}

#[test]
fn legacy_goal_fields_stay_unknown_and_repeated_initialization_preserves_data() {
    let mut db = database();
    let goal = store::state(&db)
        .unwrap()
        .goals
        .into_iter()
        .find(Goal::is_trainable)
        .unwrap();
    let mut value = serde_json::to_value(&goal).unwrap();
    for field in [
        "progress_status",
        "start_date",
        "due_date",
        "last_review_date",
        "next_review_date",
        "completion_date",
        "review_notes",
    ] {
        value.as_object_mut().unwrap().remove(field);
    }
    db.execute(
        "UPDATE goals SET data=?1 WHERE id=?2",
        params![value.to_string(), goal.id],
    )
    .unwrap();
    db.execute("DELETE FROM goal_revisions WHERE goal_id=?1", [&goal.id])
        .unwrap();
    store::initialize(&db).unwrap();
    store::initialize(&db).unwrap();
    let legacy = store::state(&db)
        .unwrap()
        .goals
        .into_iter()
        .find(|g| g.id == goal.id)
        .unwrap();
    assert!(legacy.progress_status.is_none());
    assert!(legacy.next_review_date.is_none());
    store::update_goal(&mut db, &goal.id, edit(&legacy)).unwrap();
    let revisions = store::goal_history(&db, &goal.id).unwrap();
    assert_eq!(revisions.len(), 2);
    assert_eq!(revisions[1].change_note, "旧版本基线");
    assert!(revisions[1].goal.progress_status.is_none());
}
