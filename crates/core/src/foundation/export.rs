//! Explicit business-data whitelist. Authentication and operation-cache tables are never exported.
use crate::{
    sql::Sql,
    store::{ApiError, Result},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
#[derive(Deserialize, Serialize)]
pub struct Snapshot {
    pub schema: String,
    pub exported_at: String,
    pub database_version: u32,
    pub tables: BTreeMap<String, Vec<Value>>,
}
fn table(db: &dyn Sql, name: &str) -> Result<Vec<Value>> {
    db.query(&format!("SELECT data FROM {name} ORDER BY rowid"), &[])?
        .iter()
        .map(|r| serde_json::from_str(&r[0]).map_err(ApiError::from))
        .collect()
}
pub fn snapshot(db: &dyn Sql) -> Result<Snapshot> {
    let state = super::snapshot(db)?;
    let mut tables = BTreeMap::new();
    macro_rules! add {
        ($name:expr,$rows:expr) => {
            tables.insert(
                $name.into(),
                $rows
                    .into_iter()
                    .map(serde_json::to_value)
                    .collect::<std::result::Result<Vec<_>, _>>()?,
            );
        };
    }
    add!("目标", state.goals);
    add!("方法", state.methods);
    add!("课程老师", state.catalog);
    add!("机构课记", state.lessons.clone());
    add!("家庭干预", state.sessions.clone());
    add!("经验困难", state.knowledge.clone());
    add!("下一步", state.actions);
    for (name, kind) in [
        ("观察指标", "metric"),
        ("实际测量", "measurement"),
        ("瞬间观察", "moment"),
        ("共享需要", "needs"),
    ] {
        add!(name, super::observations::current_list(db, kind)?);
    }
    for (name, sql) in [
        ("实践反馈", "prd_feedback"),
        ("反馈附注", "prd_annotations"),
        ("复盘记录", "prd_reviews"),
        ("复盘安排", "prd_schedules"),
        ("家庭设置", "prd_settings"),
        ("对象修订", "prd_revisions"),
        ("课记修订", "prd_lesson_revisions"),
        ("干预修订", "prd_session_revisions"),
        ("知识行动修订", "prd_output_revisions"),
        ("反馈修订", "prd_feedback_revisions"),
        ("复盘修订", "prd_review_revisions"),
        ("复盘安排修订", "prd_schedule_revisions"),
        ("观察修订", "prd_observation_revisions"),
        ("导入来源", "prd_import_rows"),
        ("导入批次", "prd_import_batches"),
        ("旧模型转换", "prd_upgrade"),
    ] {
        tables.insert(name.into(), table(db, sql)?);
    }
    tables.insert(
        "知识行动修订".into(),
        db.query(
            "SELECT kind,id,data FROM prd_output_revisions ORDER BY kind,id,version",
            &[],
        )?
        .into_iter()
        .map(|r| {
            let record: Value = serde_json::from_str(&r[2])?;
            Ok(json!({"record_kind":r[0],"id":r[1],"version":record["version"],"record":record}))
        })
        .collect::<Result<Vec<_>>>()?,
    );
    tables.insert(
        "目标方法关系".into(),
        db.query(
            "SELECT method_id,goal_id FROM prd_method_goals ORDER BY method_id,goal_id",
            &[],
        )?
        .into_iter()
        .map(|r| json!({"method_id":r[0],"goal_id":r[1]}))
        .collect(),
    );
    tables.insert("课记来源引用".into(),db.query("SELECT lesson_id,source_kind,source_id FROM prd_lesson_refs ORDER BY lesson_id,source_kind,source_id",&[])?.into_iter().map(|r|json!({"lesson_id":r[0],"source_kind":r[1],"source_id":r[2]})).collect());
    let mut targets = vec![];
    let mut methods = vec![];
    for s in state.sessions {
        for (phase, rows) in [("计划", s.planned_targets), ("实际", s.actual_targets)] {
            for t in rows {
                targets.push(json!({"session_id":s.id,"session_version":s.version,"status":s.status,"phase":phase,"business_date":s.input.business_date,"goal_id":t.goal.id,"goal_version":t.goal.version,"title":t.goal.input.title,"role":t.input.role,"observation":t.input.note,"ancestors":t.ancestors}));
            }
        }
        for (phase, rows) in [("计划", s.planned_methods), ("实际", s.actual_methods)] {
            for m in rows {
                methods.push(json!({"session_id":s.id,"session_version":s.version,"status":s.status,"business_date":s.input.business_date,"phase":phase,"method_id":m.id,"method_version":m.version,"title":m.input.title,"description":m.input.description}));
            }
        }
    }
    tables.insert("干预目标明细".into(), targets);
    tables.insert("干预方法明细".into(), methods);
    let mut targets = vec![];
    let mut methods = vec![];
    for l in state.lessons {
        for t in l.organization.targets {
            targets.push(json!({"lesson_id":l.id,"status":l.status,"business_date":l.observation.business_date,"goal_id":t.goal.id,"goal_version":t.goal.version,"title":t.goal.input.title,"observation":t.target.note,"ancestors":t.ancestors}));
        }
        for m in l.organization.methods {
            methods.push(json!({"lesson_id":l.id,"status":l.status,"business_date":l.observation.business_date,"method_id":m.id,"method_version":m.version,"title":m.input.title}));
        }
    }
    tables.insert("课程目标明细".into(), targets);
    tables.insert("课程方法明细".into(), methods);
    let mut links = vec![];
    for k in state.knowledge {
        for (typ, ids) in [("目标", k.input.goal_ids), ("方法", k.input.method_ids)] {
            for id in ids {
                links.push(json!({"knowledge_id":k.input.id,"kind":k.input.kind,"deleted":k.deleted,"object_kind":typ,"object_id":id}));
            }
        }
    }
    tables.insert("知识关联".into(), links);
    Ok(Snapshot {
        schema: "skarmar.prd.export.v1".into(),
        exported_at: chrono::Utc::now().to_rfc3339(),
        database_version: 6,
        tables,
    })
}
