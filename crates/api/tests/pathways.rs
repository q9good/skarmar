use axum::http::StatusCode;
use rusqlite::Connection;
use serde::{Serialize, de::DeserializeOwned};
use skarma_api::{model::*, store};
use skarma_core::Command;
use uuid::Uuid;

fn db() -> Connection {
    let mut db = Connection::open_in_memory().unwrap();
    store::initialize(&db).unwrap();
    store::seed_demo(&mut db).unwrap();
    db
}
fn run<T: DeserializeOwned>(db: &mut Connection, command: Command) -> T {
    serde_json::from_value(store::execute(db, command).unwrap()).unwrap()
}
fn same<T: Serialize>(left: &T, right: &T) {
    assert_eq!(
        serde_json::to_value(left).unwrap(),
        serde_json::to_value(right).unwrap()
    );
}
fn new_pathway(goals: &[String]) -> NewPathway {
    NewPathway {
        id: Uuid::new_v4().to_string(),
        title: "视觉提示".into(),
        description: "通过步骤图片帮助理解".into(),
        steps: "展示图片，再等待回应".into(),
        goal_ids: goals.to_vec(),
    }
}
fn edit(pathway: &Pathway, goals: Vec<String>) -> Command {
    Command::UpdatePathway {
        id: pathway.id.clone(),
        request: UpdatePathway {
            expected_version: pathway.version,
            title: "调整后的方式".into(),
            description: pathway.description.clone(),
            steps: pathway.steps.clone(),
            status: pathway.status.clone(),
            goal_ids: goals,
        },
    }
}
fn experience(goals: &[String], pathways: &[String]) -> NewExperience {
    NewExperience {
        id: Uuid::new_v4().to_string(),
        content: "先理解需要，再选择支持方式".into(),
        goal_ids: goals.to_vec(),
        pathway_ids: pathways.to_vec(),
    }
}
fn update(item: &FollowUp) -> UpdateFollowUp {
    UpdateFollowUp {
        expected_version: item.version,
        status: item.status.clone(),
        conclusion: item.conclusion.clone(),
        next_review_date: None,
        content: None,
        goal_ids: None,
        pathway_ids: None,
    }
}
fn completed(db: &mut Connection) -> SaveResult {
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
    store::save_session(
        db,
        &Uuid::new_v4().to_string(),
        SaveSession {
            operation_id: Uuid::new_v4().to_string(),
            expected_version: None,
            action: "complete".into(),
            session: SessionInput {
                business_date: "2026-10-17".into(),
                time_slot: "上午".into(),
                activities: vec!["游戏".into()],
                organization: "居家".into(),
                plan: String::new(),
                observation: String::new(),
                targets,
                has_difficulty: false,
                difficulty: String::new(),
                has_experience: true,
                experience: "训练中的观察".into(),
                experience_kind: "general".into(),
                difficulty_goal_ids: vec![],
                difficulty_pathway_ids: vec![],
                experience_goal_ids: vec![],
                experience_pathway_ids: vec![],
            },
        },
    )
    .unwrap()
}

#[test]
fn pathways_are_reused_across_purposes_with_foreign_keys_on_both_sides() {
    let mut db = db();
    let goals: Vec<_> = store::state(&db)
        .unwrap()
        .goals
        .into_iter()
        .take(2)
        .map(|goal| goal.id)
        .collect();
    let first: Pathway = run(&mut db, Command::CreatePathway(new_pathway(&goals)));
    let second: Pathway = run(&mut db, Command::CreatePathway(new_pathway(&goals)));
    assert_eq!(store::state(&db).unwrap().pathways.len(), 2);
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM goal_pathways", [], |row| row
            .get::<_, i64>(0))
            .unwrap(),
        4
    );
    assert!(
        db.execute(
            "INSERT INTO goal_pathways VALUES ('missing',?1)",
            [&first.id]
        )
        .is_err()
    );
    assert!(
        db.execute(
            "INSERT INTO goal_pathways VALUES (?1,'missing')",
            [&goals[0]]
        )
        .is_err()
    );
    let changed: Pathway = run(&mut db, edit(&first, vec![goals[1].clone()]));
    assert_eq!(changed.version, 2);
    let state = store::state(&db).unwrap();
    assert_eq!(
        state
            .pathways
            .iter()
            .find(|pathway| pathway.id == second.id)
            .unwrap()
            .goal_ids,
        goals
    );
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM goal_pathways", [], |row| row
            .get::<_, i64>(0))
            .unwrap(),
        3
    );
}

#[test]
fn invalid_stale_and_late_failing_mapping_updates_leave_the_old_pathway_intact() {
    let mut db = db();
    let goals: Vec<_> = store::state(&db)
        .unwrap()
        .goals
        .into_iter()
        .take(2)
        .map(|goal| goal.id)
        .collect();
    let input = new_pathway(&goals);
    let pathway: Pathway = run(&mut db, Command::CreatePathway(input.clone()));
    let retry: Pathway = run(&mut db, Command::CreatePathway(input));
    same(&pathway, &retry);
    for bad in [
        vec![goals[0].clone(), goals[0].clone()],
        vec![Uuid::new_v4().to_string()],
    ] {
        assert_eq!(
            store::execute(&mut db, edit(&pathway, bad)).unwrap_err().0,
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    db.execute_batch("CREATE TRIGGER fail_pathway BEFORE INSERT ON goal_pathways BEGIN SELECT RAISE(ABORT,'synthetic'); END;").unwrap();
    assert_eq!(
        store::execute(&mut db, edit(&pathway, vec![goals[1].clone()]))
            .unwrap_err()
            .0,
        StatusCode::INTERNAL_SERVER_ERROR
    );
    same(&store::state(&db).unwrap().pathways[0], &pathway);
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM goal_pathways", [], |row| row
            .get::<_, i64>(0))
            .unwrap(),
        2
    );
    db.execute_batch("DROP TRIGGER fail_pathway;").unwrap();
    let _: Pathway = run(&mut db, edit(&pathway, vec![goals[0].clone()]));
    assert_eq!(
        store::execute(&mut db, edit(&pathway, goals))
            .unwrap_err()
            .0,
        StatusCode::CONFLICT
    );
}

#[test]
fn independent_experiences_bind_to_goals_pathways_or_both_without_transitive_inference() {
    let mut db = db();
    let goals: Vec<_> = store::state(&db)
        .unwrap()
        .goals
        .into_iter()
        .take(2)
        .map(|goal| goal.id)
        .collect();
    let pathway: Pathway = run(&mut db, Command::CreatePathway(new_pathway(&goals)));
    let cases = [
        (vec![goals[0].clone()], vec![]),
        (vec![], vec![pathway.id.clone()]),
        (goals.clone(), vec![pathway.id]),
    ];
    for (goal_ids, pathway_ids) in cases {
        let input = experience(&goal_ids, &pathway_ids);
        let saved: FollowUp = run(&mut db, Command::CreateExperience(input.clone()));
        let retry: FollowUp = run(&mut db, Command::CreateExperience(input));
        same(&saved, &retry);
        assert!(saved.source_session_id.is_none());
        assert!(saved.kind.is_none());
        assert_eq!(saved.status, "pending");
        assert_eq!(saved.goal_ids, goal_ids);
        assert_eq!(saved.pathway_ids, pathway_ids);
    }
    let state = store::state(&db).unwrap();
    assert_eq!(state.experiences.len(), 3);
    assert!(state.sessions.is_empty());
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM experience_goals", [], |row| row
            .get::<_, i64>(0))
            .unwrap(),
        3
    );
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM experience_pathways", [], |row| row
            .get::<_, i64>(0))
            .unwrap(),
        2
    );
    assert_eq!(
        store::execute(&mut db, Command::CreateExperience(experience(&[], &[])))
            .unwrap_err()
            .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[test]
fn experience_binding_edits_roll_back_atomically_and_old_clients_preserve_links() {
    let mut db = db();
    let goals: Vec<_> = store::state(&db)
        .unwrap()
        .goals
        .into_iter()
        .take(2)
        .map(|goal| goal.id)
        .collect();
    let pathway: Pathway = run(&mut db, Command::CreatePathway(new_pathway(&goals)));
    let item: FollowUp = run(
        &mut db,
        Command::CreateExperience(experience(&goals[..1], std::slice::from_ref(&pathway.id))),
    );
    let mut change = update(&item);
    change.content = Some("修改后的原则".into());
    change.goal_ids = Some(vec![goals[1].clone()]);
    db.execute_batch("CREATE TRIGGER fail_binding BEFORE INSERT ON experience_pathways BEGIN SELECT RAISE(ABORT,'synthetic'); END;").unwrap();
    let command = Command::UpdateFollowUp {
        table: "experiences".into(),
        id: item.id.clone(),
        request: change,
    };
    assert_eq!(
        store::execute(&mut db, command).unwrap_err().0,
        StatusCode::INTERNAL_SERVER_ERROR
    );
    same(&store::state(&db).unwrap().experiences[0], &item);
    let retained: String = db
        .query_row(
            "SELECT goal_id FROM experience_goals WHERE experience_id=?1",
            [&item.id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(retained, goals[0]);
    db.execute_batch("DROP TRIGGER fail_binding;").unwrap();
    let mut old_client = update(&item);
    old_client.conclusion = "补充观察".into();
    let saved = store::update_follow_up(&mut db, "experiences", &item.id, old_client).unwrap();
    assert_eq!(saved.goal_ids, item.goal_ids);
    assert_eq!(saved.pathway_ids, item.pathway_ids);
    assert_eq!(
        store::update_follow_up(&mut db, "experiences", &item.id, update(&item))
            .unwrap_err()
            .0,
        StatusCode::CONFLICT
    );
    let mut invalid = update(&saved);
    invalid.pathway_ids = Some(vec![Uuid::new_v4().to_string()]);
    assert_eq!(
        store::update_follow_up(&mut db, "experiences", &item.id, invalid)
            .unwrap_err()
            .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let mut clear = update(&saved);
    clear.goal_ids = Some(vec![]);
    clear.pathway_ids = Some(vec![]);
    let cleared = store::update_follow_up(&mut db, "experiences", &item.id, clear).unwrap();
    assert!(cleared.goal_ids.is_empty() && cleared.pathway_ids.is_empty());
}

#[test]
fn upgrading_old_experiences_keeps_their_source_and_does_not_guess_bindings() {
    let mut db = db();
    let result = completed(&mut db);
    let item_id = result.experience_id.unwrap();
    let mut legacy = serde_json::to_value(&store::state(&db).unwrap().experiences[0]).unwrap();
    legacy.as_object_mut().unwrap().remove("goal_ids");
    legacy.as_object_mut().unwrap().remove("pathway_ids");
    let raw = serde_json::to_string(&legacy).unwrap();
    db.execute(
        "UPDATE experiences SET data=?1 WHERE id=?2",
        [&raw, &item_id],
    )
    .unwrap();
    db.execute_batch("DROP TABLE experience_pathways; DROP TABLE experience_goals; DROP TABLE goal_pathways; DROP TABLE pathways; DELETE FROM schema_migrations WHERE version=4;").unwrap();
    store::initialize(&db).unwrap();
    store::initialize(&db).unwrap();
    let upgraded = &store::state(&db).unwrap().experiences[0];
    assert_eq!(
        upgraded.source_session_id.as_deref(),
        Some(result.session.id.as_str())
    );
    assert_eq!(upgraded.kind.as_deref(), Some("general"));
    assert!(upgraded.goal_ids.is_empty() && upgraded.pathway_ids.is_empty());
    let retained: String = db
        .query_row(
            "SELECT data FROM experiences WHERE id=?1",
            [&item_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(retained, raw);
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM experience_goals", [], |row| row
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[test]
fn archiving_or_reassigning_a_pathway_does_not_rewrite_training_or_experience_bindings() {
    let mut db = db();
    let result = completed(&mut db);
    let goals: Vec<_> = result
        .session
        .input
        .targets
        .iter()
        .map(|target| target.goal_id.clone())
        .collect();
    let pathway: Pathway = run(&mut db, Command::CreatePathway(new_pathway(&goals)));
    let original = store::state(&db).unwrap().experiences[0].clone();
    let mut request = update(&original);
    request.goal_ids = Some(goals[..1].to_vec());
    request.pathway_ids = Some(vec![pathway.id.clone()]);
    let item = store::update_follow_up(&mut db, "experiences", &original.id, request).unwrap();
    let mut command = edit(&pathway, vec![]);
    if let Command::UpdatePathway { request, .. } = &mut command {
        request.status = PathwayStatus::Archived;
    }
    let _: Pathway = run(&mut db, command);
    let state = store::state(&db).unwrap();
    same(&state.sessions[0], &result.session);
    same(&state.experiences[0], &item);
    assert_eq!(
        item.source_session_id.as_deref(),
        Some(result.session.id.as_str())
    );
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM experience_pathways", [], |row| row
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
}
