//! Owner-authorized, bounded catalogue removal. No public mutation endpoint.
//! Keep entities/revisions/receipts for audit; a tombstone removes only catalogue
//! membership. The adapter encloses the entire operation in one transaction.
use super::model::Saved;
use crate::{
    model::Level,
    sql::{Sql, params},
    store::{ApiError, Result, json},
};
use serde_json::{Value, json as value};
use sha2::{Digest, Sha256};

pub const ACK: &str = "remove-reviewed-two-goals-20261105";
pub const TEST_ACK: &str = "remove-synthetic-two-goals-20261105";
pub const BATCH: &str = "reviewed-goals-20261105";
const MARKER: &str = "goal-removal-20261105";
const SELECTORS: [&str; 2] = [
    "8f543e798a94b145efdd6f6dd789fa52ec125cef82beb217b34fa09f91f12390",
    "50c3fa9078406f542526ea1833cc608fb856c81c948ad1016314d48dea87c3fa",
];
const SYNTHETIC: [&str; 2] = [
    "170e0bbb310880690c332eef6ec48bcfbef2a73d5245b5c27c6e70e4eb957fa2",
    "b98289fd58460893d1623a57be57ae77b9cfe76a06ce7fca2be752e68635f28b",
];
fn marker(db: &dyn Sql) -> Result<Option<Value>> {
    if db
        .query(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='prd_upgrade'",
            &[],
        )?
        .is_empty()
    {
        return Ok(None);
    }
    db.query("SELECT data FROM prd_upgrade WHERE id=?1", params![MARKER])?
        .first()
        .map(|r| serde_json::from_str(&r[0]).map_err(ApiError::from))
        .transpose()
}
pub fn removed_ids(db: &dyn Sql) -> Result<Vec<String>> {
    let Some(m) = marker(db)? else {
        return Ok(vec![]);
    };
    let ids: Vec<String> = serde_json::from_value(m["ids"].clone())?;
    if m["schema"] != "skarmar.goal-removal.v1"
        || ids.len() != 2
        || ids[0] == ids[1]
        || !matches!(
            m["batch"].as_str(),
            Some(BATCH | "synthetic-goals-20261105")
        )
    {
        return Err(ApiError::conflict("目标删除标记异常，请核对，禁止重复处理"));
    }
    let selectors = if m["batch"] == BATCH {
        SELECTORS
    } else {
        SYNTHETIC
    };
    for (i, id) in ids.iter().enumerate() {
        crate::store::valid_id(id)?;
        let row = db.query(
            "SELECT data FROM prd_entities WHERE id=?1 AND kind='goal'",
            params![id.as_str()],
        )?;
        let Some(row) = row.first() else {
            return Err(ApiError::conflict("目标删除留存记录异常"));
        };
        let Saved::Goal(g) = serde_json::from_str(&row[0])? else {
            return Err(ApiError::storage());
        };
        if g.input.level == Level::Long
            || format!("{:x}", Sha256::digest(g.input.title.as_bytes())) != selectors[i]
        {
            return Err(ApiError::conflict("目标删除留存记录不匹配"));
        }
    }
    Ok(ids)
}
pub fn status(db: &dyn Sql) -> Result<Value> {
    let m = marker(db)?;
    if m.is_some() {
        removed_ids(db)?;
        return Ok(
            value!({"batch":m.as_ref().and_then(|m|m["batch"].as_str()).unwrap_or(BATCH),"completed":true}),
        );
    }
    let blocker = match inspect(db, SELECTORS)? {
        Ok(_) => "ready",
        Err(reason) => reason,
    };
    // Bounded classification only: no target names, IDs, counts or row content.
    Ok(value!({"batch":BATCH,"completed":false,"blocker":blocker}))
}
fn mentions(v: &Value, ids: &[String]) -> bool {
    match v {
        Value::String(s) => ids.contains(s),
        Value::Array(a) => a.iter().any(|v| mentions(v, ids)),
        Value::Object(o) => {
            o.keys().any(|k| ids.contains(k)) || o.values().any(|v| mentions(v, ids))
        }
        _ => false,
    }
}
pub fn apply_once(db: &dyn Sql, ack: &str) -> Result<Value> {
    let (selectors, batch) = match ack {
        ACK => (SELECTORS, BATCH),
        TEST_ACK => (SYNTHETIC, "synthetic-goals-20261105"),
        _ => return Err(ApiError::invalid("缺少本次两项目标删除的精确确认")),
    };
    if marker(db)?.is_some() {
        return status(db);
    }
    if !super::enabled(db)?
        || db
            .query(
                "SELECT id FROM auth_users WHERE active=1 AND role='admin' LIMIT 1",
                &[],
            )?
            .is_empty()
    {
        return Err(ApiError::conflict(
            "只能维护原新模型和既有管理员，禁止初始化或重设账号",
        ));
    }
    let ids = match inspect(db, selectors)? {
        Ok(ids) => ids,
        Err(reason) => return Ok(value!({"batch":batch,"completed":false,"blocker":reason})),
    };
    let schedules = ids
        .iter()
        .map(|id| {
            db.query(
                "SELECT data FROM prd_schedules WHERE object_kind='goal' AND object_id=?1",
                params![id.as_str()],
            )
        })
        .collect::<Result<Vec<_>>>()?;
    for id in &ids {
        db.execute(
            "DELETE FROM prd_schedules WHERE object_kind='goal' AND object_id=?1",
            params![id.as_str()],
        )?;
    }
    db.execute("INSERT INTO prd_upgrade(id,data) VALUES(?1,?2)",params![MARKER,json(&value!({"schema":"skarmar.goal-removal.v1","batch":batch,"ids":ids,"prior_schedules":schedules,"completed_at":chrono::Utc::now().to_rfc3339()}))?])?;
    if !db.query("PRAGMA foreign_key_check", &[])?.is_empty() {
        return Err(ApiError::conflict("目标删除引用校验失败，必须整次回滚"));
    }
    status(db)
}

// Inspect only; both status and the maintenance transaction use the same guard.
fn inspect(
    db: &dyn Sql,
    selectors: [&str; 2],
) -> Result<std::result::Result<Vec<String>, &'static str>> {
    let rows = db.query(
        "SELECT id,data FROM prd_entities WHERE kind='goal' ORDER BY id",
        &[],
    )?;
    let mut groups: [Vec<String>; 2] = Default::default();
    for row in &rows {
        let Saved::Goal(g) = serde_json::from_str(&row[1])? else {
            return Err(ApiError::storage());
        };
        if g.input.level == Level::Long {
            continue;
        }
        let digest = format!("{:x}", Sha256::digest(g.input.title.as_bytes()));
        for (i, selector) in selectors.iter().enumerate() {
            if digest == *selector {
                groups[i].push(g.id.clone())
            }
        }
    }
    // Missing, renamed or ambiguous candidates are never guessed. A blocked batch
    // leaves the application usable and reports completed:false, with zero changes.
    if groups.iter().any(|g| g.len() > 1) {
        return Ok(Err("candidate_ambiguous"));
    }
    if groups.iter().any(|g| g.is_empty()) {
        return Ok(Err("candidate_missing"));
    }
    let ids = groups.into_iter().map(|g| g[0].clone()).collect::<Vec<_>>();
    // Delete catalogue membership, never historical records or followups. These
    // retain original bindings; only same-record existing bindings may be reused.
    // Child goals, live methods and course organization still require explicit handling.
    for table in ["prd_entities", "prd_lessons"] {
        for row in db.query(&format!("SELECT data FROM {table}"), &[])? {
            let v: Value = serde_json::from_str(&row[0])?;
            let own = table == "prd_entities" && ids.iter().any(|id| v["value"]["id"] == *id);
            if !own && mentions(&v, &ids) {
                return Ok(Err(match table {
                    "prd_entities" => "linked_entities",
                    "prd_sessions" => "linked_sessions",
                    "prd_lessons" => "linked_lessons",
                    _ => "linked_followups",
                }));
            }
        }
    }
    if !db
        .query("SELECT goal_id FROM prd_method_goals", &[])?
        .iter()
        .all(|r| !ids.contains(&r[0]))
    {
        return Ok(Err("linked_methods"));
    }
    Ok(Ok(ids))
}
