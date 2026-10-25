//! All lesson data and actors in this file are synthetic.
use axum::http::StatusCode;
use rusqlite::Connection;
use skarma_api::store;
use skarma_core::{
    auth::{AccountRole, User},
    foundation::{lesson_model::*, model::*},
};
use uuid::Uuid;
fn id() -> String {
    Uuid::new_v4().to_string()
}
fn actor() -> User {
    User {
        id: id(),
        username: "synthetic".into(),
        display_name: "测试家人".into(),
        role: AccountRole::Member,
        active: true,
    }
}
fn put(db: &mut Connection, user: &User, mutation: Mutation) -> Saved {
    store::write_foundation(
        db,
        user,
        Write {
            provided_fields: None,
            method_change: None,
            id: id(),
            operation_id: id(),
            expected_version: None,
            change_note: String::new(),
            mutation,
        },
    )
    .unwrap()
}
fn fixture() -> (Connection, User, ObservationInput, String, String) {
    let mut db = Connection::open_in_memory().unwrap();
    store::initialize_foundation(&db).unwrap();
    let user = actor();
    let c = put(
        &mut db,
        &user,
        Mutation::Catalog(CatalogInput {
            kind: CatalogKind::Course,
            name: "测试课程".into(),
            active: true,
        }),
    );
    let t = put(
        &mut db,
        &user,
        Mutation::Catalog(CatalogInput {
            kind: CatalogKind::Teacher,
            name: "测试老师旧名".into(),
            active: true,
        }),
    );
    let g = put(
        &mut db,
        &user,
        Mutation::Goal(
            serde_json::from_value(serde_json::json!({"title":"测试本领","level":"short"}))
                .unwrap(),
        ),
    );
    let m = put(
        &mut db,
        &user,
        Mutation::Method(
            serde_json::from_value(serde_json::json!({"title":"测试方法","goal_ids":[g.id()]}))
                .unwrap(),
        ),
    );
    let date = "2020-01-01".to_owned();
    let input = ObservationInput {
        business_date: date,
        course_id: c.id().into(),
        teacher_id: t.id().into(),
        ..Default::default()
    };
    (db, user, input, g.id().into(), m.id().into())
}
fn cmd(id: &str, v: Option<u64>, action: LessonAction) -> LessonWrite {
    LessonWrite {
        id: id.into(),
        operation_id: Uuid::new_v4().to_string(),
        expected_version: v,
        action,
    }
}
fn org(l: &Lesson) -> OrganizationInput {
    OrganizationInput {
        feedback: vec![],
        feedback_knowledge_versions: Default::default(),
        seen_observation_version: l.observation_version,
        targets: vec![],
        method_ids: vec![],
        focus_goal_ids: vec![],
        processing: Processing::Keep,
        request_generation: l.request_generation,
        outcome: String::new(),
        new_knowledge: vec![],
        new_actions: vec![],
    }
}
#[test]
fn independent_partitions_conflict_only_within_their_own_version() {
    let (mut db, u, input, g, m) = fixture();
    let identifier = id();
    let l = store::write_foundation_lesson(
        &mut db,
        &u,
        cmd(&identifier, None, LessonAction::Observation(input.clone())),
    )
    .unwrap();
    let mut organize = org(&l);
    organize.targets.push(LessonTarget {
        goal_id: g,
        note: "测试观察".into(),
    });
    organize.method_ids.push(m);
    let mut changed = input;
    changed.notes = "老人补充观察".into();
    store::write_foundation_lesson(
        &mut db,
        &u,
        cmd(&identifier, Some(1), LessonAction::Observation(changed)),
    )
    .unwrap();
    let saved = store::write_foundation_lesson(
        &mut db,
        &u,
        cmd(
            &identifier,
            Some(1),
            LessonAction::Organization(organize.clone()),
        ),
    )
    .unwrap();
    assert_eq!(saved.observation.notes, "老人补充观察");
    assert_eq!(saved.observation_version, 2);
    assert_eq!(saved.organization_version, 2);
    assert_eq!(saved.organization.seen_observation_version, 1);
    assert_eq!(saved.organization.targets.len(), 1);
    assert_eq!(
        store::write_foundation_lesson(
            &mut db,
            &u,
            cmd(&identifier, Some(1), LessonAction::Organization(organize))
        )
        .unwrap_err()
        .0,
        StatusCode::CONFLICT
    );
}
#[test]
fn duplicate_retry_is_one_lesson_and_names_do_not_follow_catalog_rename() {
    let (mut db, u, input, _, _) = fixture();
    let identifier = id();
    let request = cmd(&identifier, None, LessonAction::Observation(input.clone()));
    let one = store::write_foundation_lesson(&mut db, &u, request.clone()).unwrap();
    let two = store::write_foundation_lesson(&mut db, &u, request).unwrap();
    assert_eq!(one.version, two.version);
    store::write_foundation(
        &mut db,
        &u,
        Write {
            provided_fields: None,
            method_change: None,
            id: input.teacher_id.clone(),
            operation_id: id(),
            expected_version: Some(1),
            change_note: String::new(),
            mutation: Mutation::Catalog(CatalogInput {
                kind: CatalogKind::Teacher,
                name: "测试老师新名".into(),
                active: true,
            }),
        },
    )
    .unwrap();
    let edited = store::write_foundation_lesson(
        &mut db,
        &u,
        cmd(
            &identifier,
            Some(1),
            LessonAction::Observation(ObservationInput {
                notes: "补充".into(),
                ..input.clone()
            }),
        ),
    )
    .unwrap();
    assert_eq!(edited.teacher.name, "测试老师旧名");
    let next = store::write_foundation_lesson(
        &mut db,
        &u,
        cmd(&id(), None, LessonAction::Observation(input)),
    )
    .unwrap();
    assert_eq!(next.teacher.id, edited.teacher.id);
    assert_eq!(next.teacher.name, "测试老师新名");
    assert_eq!(
        store::foundation_snapshot(&mut db).unwrap().lessons.len(),
        2
    );
}
#[test]
fn organizer_outputs_are_atomic_idempotent_and_block_withdrawal() {
    let (mut db, u, input, g, m) = fixture();
    let identifier = id();
    let l = store::write_foundation_lesson(
        &mut db,
        &u,
        cmd(
            &identifier,
            None,
            LessonAction::Observation(ObservationInput {
                notes: "原始观察".into(),
                ..input
            }),
        ),
    )
    .unwrap();
    let mut o = org(&l);
    o.new_knowledge.push(NewKnowledge {
        id: id(),
        kind: KnowledgeKind::Difficulty,
        content: "测试困难".into(),
        goal_ids: vec![g.clone()],
        method_ids: vec![m],
        stage: None,
        next_review_date: None,
    });
    o.new_actions.push(NewAction {
        suggested_date: None,
        id: id(),
        content: "下次试试".into(),
        goal_ids: vec![g],
        method_ids: vec![],
        assignee_id: None,
    });
    let request = cmd(&identifier, Some(1), LessonAction::Organization(o));
    db.execute_batch("CREATE TRIGGER synthetic_fail BEFORE INSERT ON prd_lesson_revisions BEGIN SELECT RAISE(ABORT,'failure'); END;").unwrap();
    assert!(store::write_foundation_lesson(&mut db, &u, request.clone()).is_err());
    let state = store::foundation_snapshot(&mut db).unwrap();
    assert!(state.knowledge.is_empty());
    assert!(state.actions.is_empty());
    assert_eq!(state.lessons[0].organization_version, 1);
    db.execute_batch("DROP TRIGGER synthetic_fail").unwrap();
    let saved = store::write_foundation_lesson(&mut db, &u, request.clone()).unwrap();
    store::write_foundation_lesson(&mut db, &u, request).unwrap();
    let state = store::foundation_snapshot(&mut db).unwrap();
    assert_eq!(state.knowledge.len(), 1);
    assert_eq!(state.actions.len(), 1);
    assert_eq!(
        state.knowledge[0]
            .source_observation
            .as_ref()
            .unwrap()
            .notes,
        "原始观察"
    );
    assert!(state.knowledge[0].input.next_review_date.is_some());
    assert_eq!(
        store::write_foundation_lesson(
            &mut db,
            &u,
            cmd(&identifier, Some(saved.version), LessonAction::Withdraw)
        )
        .unwrap_err()
        .0,
        StatusCode::CONFLICT
    );
}
#[test]
fn creator_today_withdraw_and_admin_delete_are_distinct_from_marking_error() {
    let (mut db, u, mut input, _, _) = fixture();
    input.business_date = "2020-01-01".into();
    let identifier = id();
    let saved = store::write_foundation_lesson(
        &mut db,
        &u,
        cmd(&identifier, None, LessonAction::Observation(input)),
    )
    .unwrap();
    assert_eq!(
        store::write_foundation_lesson(
            &mut db,
            &actor(),
            cmd(&identifier, Some(saved.version), LessonAction::Withdraw)
        )
        .unwrap_err()
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        store::write_foundation_lesson(
            &mut db,
            &u,
            cmd(&identifier, Some(saved.version), LessonAction::Delete)
        )
        .unwrap_err()
        .0,
        StatusCode::FORBIDDEN
    );
    let marked = store::write_foundation_lesson(
        &mut db,
        &actor(),
        cmd(
            &identifier,
            Some(saved.version),
            LessonAction::MarkError { marked: true },
        ),
    )
    .unwrap();
    assert_eq!(marked.status, "recorded");
    assert!(marked.marked_error);
    let removed = store::write_foundation_lesson(
        &mut db,
        &u,
        cmd(&identifier, Some(marked.version), LessonAction::Withdraw),
    )
    .unwrap();
    assert_eq!(removed.status, "withdrawn");
    let mut future = removed.observation;
    future.business_date = "2099-01-01".into();
    assert_eq!(
        store::write_foundation_lesson(
            &mut db,
            &u,
            cmd(&id(), None, LessonAction::Observation(future))
        )
        .unwrap_err()
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
}
#[test]
fn request_generation_claim_takeover_and_thanks_do_not_complete_each_other() {
    let (mut db, u, input, _, _) = fixture();
    let identifier = id();
    let l = store::write_foundation_lesson(
        &mut db,
        &u,
        cmd(
            &identifier,
            None,
            LessonAction::Observation(ObservationInput {
                request_review: true,
                ..input
            }),
        ),
    )
    .unwrap();
    assert_eq!(l.request_generation, 1);
    let other = actor();
    let thanks = store::write_foundation_lesson(
        &mut db,
        &other,
        cmd(&identifier, Some(1), LessonAction::Acknowledge),
    )
    .unwrap();
    assert_eq!(thanks.acknowledgements.len(), 1);
    assert_ne!(thanks.organization.state, "done");
    let mut claim = org(&thanks);
    claim.processing = Processing::Claim;
    let claimed = store::write_foundation_lesson(
        &mut db,
        &u,
        cmd(&identifier, Some(1), LessonAction::Organization(claim)),
    )
    .unwrap();
    let mut take = org(&claimed);
    take.processing = Processing::Claim;
    assert_eq!(
        store::write_foundation_lesson(
            &mut db,
            &other,
            cmd(
                &identifier,
                Some(2),
                LessonAction::Organization(take.clone())
            )
        )
        .unwrap_err()
        .0,
        StatusCode::CONFLICT
    );
    take.processing = Processing::TakeOver;
    let changed = store::write_foundation_lesson(
        &mut db,
        &other,
        cmd(&identifier, Some(2), LessonAction::Organization(take)),
    )
    .unwrap();
    assert_eq!(changed.organization.claimed_by, Some(other.id));
}

#[test]
fn creation_day_not_business_day_controls_withdrawal_and_admin_can_delete_old_unreferenced_record()
{
    let (mut db, u, input, _, _) = fixture();
    let identifier = id();
    let l = store::write_foundation_lesson(
        &mut db,
        &u,
        cmd(&identifier, None, LessonAction::Observation(input)),
    )
    .unwrap();
    db.execute("UPDATE prd_lessons SET data=json_set(data,'$.created_at','2000-01-01T16:00:00Z') WHERE id=?1",[&identifier]).unwrap();
    assert_eq!(
        store::write_foundation_lesson(
            &mut db,
            &u,
            cmd(&identifier, Some(l.version), LessonAction::Withdraw)
        )
        .unwrap_err()
        .0,
        StatusCode::FORBIDDEN
    );
    let administrator = User {
        role: AccountRole::Admin,
        ..actor()
    };
    let deleted = store::write_foundation_lesson(
        &mut db,
        &administrator,
        cmd(&identifier, Some(l.version), LessonAction::Delete),
    )
    .unwrap();
    assert_eq!(deleted.status, "deleted");
}

#[test]
fn v1_development_schema_upgrades_without_changing_objects_or_history() {
    let mut db = Connection::open_in_memory().unwrap();
    db.execute_batch(include_str!("../../core/src/foundation/schema.sql"))
        .unwrap();
    let u = actor();
    let saved = put(
        &mut db,
        &u,
        Mutation::Goal(
            serde_json::from_value(serde_json::json!({"title":"旧开发库目标","level":"short"}))
                .unwrap(),
        ),
    );
    store::initialize_foundation(&db).unwrap();
    let state = store::foundation_snapshot(&mut db).unwrap();
    assert_eq!(state.goals[0].id, saved.id());
    assert_eq!(
        store::foundation_history(&mut db, saved.id())
            .unwrap()
            .len(),
        1
    );
    assert!(state.lessons.is_empty());
    let version: i64 = db
        .query_row("SELECT version FROM prd_schema", [], |r| r.get(0))
        .unwrap();
    assert_eq!(version, 6);
}

#[test]
fn explicit_revisit_creates_a_review_request_but_ordinary_organization_does_not() {
    let (mut db, u, input, _, _) = fixture();
    let identifier = id();
    let first = store::write_foundation_lesson(
        &mut db,
        &u,
        cmd(&identifier, None, LessonAction::Observation(input)),
    )
    .unwrap();
    let saved = store::write_foundation_lesson(
        &mut db,
        &u,
        cmd(
            &identifier,
            Some(1),
            LessonAction::Organization(org(&first)),
        ),
    )
    .unwrap();
    assert_eq!(saved.request_generation, 0);
    let mut revisit = org(&saved);
    revisit.processing = Processing::Reopen;
    revisit.outcome = "测试：请再看看".into();
    let queued = store::write_foundation_lesson(
        &mut db,
        &u,
        cmd(&identifier, Some(2), LessonAction::Organization(revisit)),
    )
    .unwrap();
    assert_eq!(queued.request_generation, 1);
    assert_eq!(queued.organization.state, "pending");
    assert_eq!(queued.requested_by, Some(u.id));
    assert_eq!(queued.request_message, "测试：请再看看");
}
