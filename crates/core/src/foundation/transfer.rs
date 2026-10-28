//! Explicit, append-only transfer. All calls run inside the adapter's transaction.
//! Source rows are retained; no prose-derived relationships, progress, or practice counts.
use super::{lesson_model as lm, model as m, process_model as pm, session_model as sm};
use crate::{
    auth::User,
    import::ImportPreviewInput,
    sql::{Sql, params},
    store::{ApiError, Result, json, valid_id},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json as value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

const LIMIT: usize = 1000;
const MAX_JSON: usize = 1_000_000;
#[path = "starter.rs"]
mod starter;
pub use starter::Selection as StarterSelection;
const LEGACY: &[&str] = &[
    "schema_migrations",
    "goals",
    "sessions",
    "session_targets",
    "difficulties",
    "experiences",
    "operations",
    "goal_revisions",
    "pathways",
    "goal_pathways",
    "experience_goals",
    "experience_pathways",
    "import_previews",
    "import_batches",
    "import_source_rows",
    "lessons",
    "lesson_options",
    "deleted_records",
    "session_revisions",
    "lesson_targets",
];
const AUTH: &[&str] = &[
    "auth_users",
    "auth_sessions",
    "auth_login_attempts",
    "auth_registration_requests",
    "auth_registration_limits",
];
fn hash(s: &str) -> String {
    format!("{:x}", Sha256::digest(s.as_bytes()))
}
fn bounded<T: Serialize>(v: &T) -> Result<String> {
    let s = json(v)?;
    if s.len() > MAX_JSON {
        return Err(ApiError::invalid("本批资料过大，请分成更小的批次"));
    }
    Ok(s)
}
fn stable(parts: &[&str]) -> String {
    let digest = Sha256::digest(serde_json::to_vec(parts).expect("strings"));
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 15) | 0x50;
    bytes[8] = (bytes[8] & 63) | 0x80;
    Uuid::from_bytes(bytes).to_string()
}
fn source_key(namespace: &str, sheet: &str, key: &str) -> Result<String> {
    Ok(hash(&json(&(namespace, sheet, key))?))
}
fn read_json(db: &dyn Sql, table: &str, id: &str) -> Result<Option<Value>> {
    db.query(
        &format!("SELECT data FROM {table} WHERE id=?1"),
        params![id],
    )?
    .first()
    .map(|r| serde_json::from_str(&r[0]).map_err(ApiError::from))
    .transpose()
}
fn admin(actor: &User) -> Result<()> {
    if !actor.active {
        return Err(crate::auth::unauthorized());
    }
    crate::import::ensure_admin(actor)
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SheetRule {
    pub sheet: String,
    /// archive, goal, method, experience, difficulty, moment, lesson. Training is
    /// intentionally not inferred from historical narrative-only workbook rows.
    pub kind: String,
    #[serde(default)]
    pub fields: BTreeMap<String, String>,
    #[serde(default)]
    pub level: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MapInput {
    pub preview_id: String,
    pub namespace: String,
    pub rules: Vec<SheetRule>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Confirm {
    pub preview_id: String,
    pub digest: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UpgradeInput {
    pub source_digest: String,
    pub note: String,
    pub sessions: Vec<SessionDecision>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Command {
    StarterPreview(StarterSelection),
    AuthorizeUpload,
    Workbook(ImportPreviewInput),
    Map(MapInput),
    Commit(Confirm),
    History {
        #[serde(default)]
        page: u32,
    },
    Source {
        key: String,
    },
    Sources {
        batch_id: String,
        #[serde(default)]
        page: u32,
    },
    UpgradePreview,
    UpgradeCommit(UpgradeInput),
    Rollback {
        baseline: String,
        note: String,
    },
}
#[derive(Clone, Debug, Deserialize, Serialize)]
struct Preview {
    id: String,
    actor_id: String,
    expires_at: String,
    input: ImportPreviewInput,
    #[serde(default)]
    plan: Option<Plan>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
struct Plan {
    namespace: String,
    rows: Vec<Row>,
    target_digest: String,
    digest: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
struct Row {
    key: String,
    sheet: String,
    row: usize,
    raw: Value,
    fingerprint: String,
    target_id: String,
    mapped: Mapped,
    #[serde(default)]
    skipped: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
enum Mapped {
    Archive,
    Catalog(m::CatalogInput),
    Existing(Box<m::Saved>),
    Session {
        completed: bool,
        input: Box<sm::Input>,
    },
    Goal(m::GoalInput),
    Method(m::MethodInput),
    Knowledge {
        input: lm::NewKnowledge,
        details: pm::Details,
    },
    Moment(Value),
    Lesson {
        date: String,
        course: String,
        teacher: String,
        notes: String,
        teacher_notes: String,
        difficulty_notes: String,
        targets: Vec<String>,
        methods: Vec<String>,
    },
}
impl Mapped {
    fn kind(&self) -> &str {
        match self {
            Self::Catalog(c) => starter::catalog_kind(&c.kind),
            Self::Existing(s) => match s.as_ref() {
                m::Saved::Goal(_) => "goal",
                m::Saved::Method(_) => "method",
                m::Saved::Catalog(c) => starter::catalog_kind(&c.input.kind),
            },
            Self::Session { .. } => "session",
            Self::Archive => "archive",
            Self::Goal(_) => "goal",
            Self::Method(_) => "method",
            Self::Knowledge { input, .. } => {
                if input.kind == lm::KnowledgeKind::Experience {
                    "experience"
                } else {
                    "difficulty"
                }
            }
            Self::Moment(_) => "moment",
            Self::Lesson { .. } => "lesson",
        }
    }
}

pub fn command(db: &dyn Sql, actor: &User, cmd: Command) -> Result<Value> {
    admin(actor)?;
    let prd = super::enabled(db)?;
    match cmd {
        Command::StarterPreview(input) if prd => starter::preview(db, actor, input),
        Command::UpgradePreview if !prd => upgrade_preview(db),
        Command::UpgradeCommit(input) if !prd => upgrade_commit(db, actor, input),
        Command::UpgradeCommit(input) if prd => {
            let marker =
                read_json(db, "prd_upgrade", "legacy-v10")?.ok_or_else(ApiError::missing)?;
            if marker["source_digest"] != input.source_digest
                || marker["receipt"]["decisions"] != serde_json::to_value(&input.sessions)?
                || marker["audit"]["actor_id"] != actor.id
                || marker["audit"]["note"] != input.note
            {
                return Err(ApiError::conflict("此库已转换，请读取原回执，不能重新转换"));
            }
            Ok(value!({"receipt":marker["receipt"],"baseline":marker["baseline"],"replayed":true}))
        }
        Command::Rollback { baseline, note } if prd => rollback(db, &baseline, &note),
        Command::AuthorizeUpload if prd => Ok(value!({"authorized":true})),
        Command::Workbook(input) if prd => upload(db, actor, input),
        Command::Map(input) if prd => map(db, actor, input),
        Command::Commit(input) if prd => commit(db, actor, input),
        Command::History { page } if prd => Ok(
            value!({"batches":db.query("SELECT data FROM prd_import_batches ORDER BY rowid DESC LIMIT 31 OFFSET ?1",params![i64::from(page)*30])?.into_iter().map(|r|serde_json::from_str::<Value>(&r[0]).map_err(ApiError::from)).collect::<Result<Vec<_>>>()?}),
        ),
        Command::Sources { batch_id, page } if prd => {
            valid_id(&batch_id)?;
            let rows=db.query("SELECT data FROM prd_import_rows WHERE json_extract(data,'$.batch_id')=?1 ORDER BY rowid LIMIT 31 OFFSET ?2",params![batch_id,i64::from(page)*30])?;
            let mut sources=rows.iter().take(30).map(|r|{let v:Value=serde_json::from_str(&r[0])?;Ok(value!({"key":v["key"],"sheet":v["sheet"],"row":v["row"],"target_id":v["target_id"]}))}).collect::<Result<Vec<_>>>()?;
            sources.shrink_to_fit();
            Ok(value!({"sources":sources,"has_more":rows.len()>30,"page":page}))
        }
        Command::Source { key } if prd => db
            .query(
                "SELECT data FROM prd_import_rows WHERE source_key=?1",
                params![key],
            )?
            .first()
            .map(|r| serde_json::from_str(&r[0]).map_err(ApiError::from))
            .unwrap_or_else(|| Err(ApiError::missing())),
        _ => Err(ApiError::conflict("转换操作与当前数据模型不匹配")),
    }
}
fn upload(db: &dyn Sql, actor: &User, input: ImportPreviewInput) -> Result<Value> {
    if input.sheets.is_empty() || input.sheets.iter().map(|s| s.rows.len()).sum::<usize>() > LIMIT {
        return Err(ApiError::invalid("每批最多1000行，请分批导出"));
    }
    // Also bound direct command callers, not only the XLSX parser.
    bounded(&input)?;
    let preview = Preview {
        id: Uuid::new_v4().to_string(),
        actor_id: actor.id.clone(),
        expires_at: (chrono::Utc::now() + chrono::Duration::hours(1)).to_rfc3339(),
        input,
        plan: None,
    };
    db.execute(
        "INSERT INTO prd_import_previews(id,data) VALUES(?1,?2)",
        params![preview.id, bounded(&preview)?],
    )?;
    Ok(
        value!({"preview_id":preview.id,"expires_at":preview.expires_at,"filename":preview.input.filename,"sheets":preview.input.sheets.iter().map(|s|value!({"name":s.name,"headers":s.headers,"rows":s.rows.len(),"sample":s.rows.iter().take(3).collect::<Vec<_>>()})).collect::<Vec<_>>()}),
    )
}
fn preview(db: &dyn Sql, actor: &User, id: &str) -> Result<Preview> {
    valid_id(id)?;
    let p: Preview = serde_json::from_value(
        read_json(db, "prd_import_previews", id)?.ok_or_else(ApiError::missing)?,
    )?;
    if p.actor_id != actor.id {
        return Err(crate::auth::unauthorized());
    }
    // Committed receipts remain retryable after preview expiry.
    if p.expires_at < chrono::Utc::now().to_rfc3339()
        && read_json(db, "prd_import_batches", id)?.is_none()
    {
        return Err(ApiError::conflict("预览已过期，请重新上传并核对"));
    }
    Ok(p)
}
fn field(rule: &SheetRule, headers: &[String], cells: &[String], key: &str) -> Result<String> {
    let Some(name) = rule.fields.get(key).filter(|s| !s.is_empty()) else {
        return Ok(String::new());
    };
    let matched = headers
        .iter()
        .enumerate()
        .filter(|(_, h)| *h == name)
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    if matched.len() != 1 {
        return Err(ApiError::invalid("所选字段不存在或表头重复，请整理文件"));
    }
    Ok(cells.get(matched[0]).cloned().unwrap_or_default())
}
fn required(v: String, label: &str) -> Result<String> {
    if v.trim().is_empty() {
        Err(ApiError::invalid(label))
    } else {
        Ok(v.trim().into())
    }
}
fn refs(raw: &str, ids: &BTreeMap<String, String>) -> Result<Vec<String>> {
    if raw.trim().is_empty() {
        return Ok(vec![]);
    } // Explicit sheet:key references only, never title fuzzy matching.
    raw.split(['\n', ';', '；'])
        .map(|s| {
            ids.get(s.trim()).cloned().ok_or_else(|| {
                ApiError::invalid("关联须使用工作表名:来源编号，多项用分号分隔；未找到明确来源")
            })
        })
        .collect()
}
fn parse_mapped(
    rule: &SheetRule,
    headers: &[String],
    cells: &[String],
    id: &str,
    ids: &BTreeMap<String, String>,
) -> Result<Mapped> {
    let allowed = match rule.kind.as_str() {
        "archive" => vec![],
        "goal" => vec!["key", "title", "level", "description", "criteria", "parent"],
        "method" => vec!["key", "title", "description", "steps", "goals"],
        "experience" => vec![
            "key", "title", "evidence", "goals", "methods", "stage", "state",
        ],
        "difficulty" => vec!["key", "title", "evidence", "goals", "methods"],
        "moment" => vec!["key", "date", "title", "goals"],
        "session" => vec![
            "key",
            "date",
            "time_slot",
            "status",
            "plan",
            "main_goal",
            "goals",
            "methods",
            "actual_goals",
            "actual_methods",
            "confirmation",
            "observation",
        ],
        "lesson" => vec![
            "key",
            "date",
            "course",
            "teacher",
            "description",
            "teacher_notes",
            "difficulty_notes",
            "goals",
            "methods",
        ],
        _ => return Err(ApiError::invalid("请明确选择工作表用途")),
    };
    if rule.fields.keys().any(|k| !allowed.contains(&k.as_str())) {
        return Err(ApiError::invalid(
            "所选类型包含不适用的映射字段，请重新选择",
        ));
    }
    let f = |key| field(rule, headers, cells, key);
    let title = || required(f("title")?, "请为每条业务记录提供名称／内容");
    let gs = || refs(&f("goals")?, ids);
    let ms = || refs(&f("methods")?, ids);
    match rule.kind.as_str() {
        "archive" => Ok(Mapped::Archive),
        "goal" => {
            let level = if rule.level.is_empty() {
                f("level")?
            } else {
                rule.level.clone()
            };
            let level = match level.trim() {
                "long" | "长期" => "long",
                "medium" | "中期" => "medium",
                "short" | "短期" => "short",
                _ => return Err(ApiError::invalid("目标期限须明确选择长期、中期或短期")),
            };
            let parents = refs(&f("parent")?, ids)?;
            if parents.len() > 1 {
                return Err(ApiError::invalid("目标只能有一个明确上级"));
            }
            Ok(Mapped::Goal(serde_json::from_value(
                value!({"title":title()?,"level":level,"meaning":f("description")?,"criteria":f("criteria")?,"parent_id":parents.first(),"state":"active"}),
            )?))
        }
        "method" => Ok(Mapped::Method(serde_json::from_value(
            value!({"title":title()?,"description":f("description")?,"steps":f("steps")?,"goal_ids":gs()?,"state":"active"}),
        )?)),
        "experience" | "difficulty" => {
            let stage = if rule.kind == "experience" {
                Some(match f("stage")?.trim() {
                    "" | "idea" | "提出" | "待验证" => "idea",
                    "try" | "已尝试" | "已使用" | "可用" => "try",
                    "helpful" | "verified" | "已验证" | "观察好用" => "helpful",
                    _ => {
                        return Err(ApiError::invalid(
                            "经验阶段请明确为 idea／try／helpful，或提出／已使用／已验证",
                        ));
                    }
                })
            } else {
                None
            };
            let state = if rule.kind == "experience" {
                match f("state")?.trim() {
                    "" | "active" | "pending" | "verified" | "启用" | "已验证" => "active",
                    "paused" | "暂停" => "paused",
                    "retired" | "stopped" | "停用" => "retired",
                    _ => return Err(ApiError::invalid("经验使用状态请明确为启用／暂停／停用")),
                }
            } else {
                ""
            };
            Ok(Mapped::Knowledge {
                input: serde_json::from_value(
                    value!({"id":id,"kind":rule.kind,"content":title()?,"goal_ids":gs()?,"method_ids":ms()?,"stage":stage}),
                )?,
                details: pm::Details {
                    title: title()?.chars().take(120).collect(),
                    evidence: f("evidence")?,
                    state: state.into(),
                    ..Default::default()
                },
            })
        }
        "session" => {
            let status = required(f("status")?, "家庭记录必须有明确状态列：planned／completed")?;
            if !["planned", "completed"].contains(&status.as_str()) {
                return Err(ApiError::invalid(
                    "家庭记录状态只能明确映射 planned／completed，不能从观察文字推断",
                ));
            }
            let date = crate::import::date(f("date")?.trim())?
                .ok_or_else(|| ApiError::invalid("请提供干预日期"))?;
            let main = refs(&f("main_goal")?, ids)?;
            if main.len() > 1 {
                return Err(ApiError::invalid("主要目标只能有一个"));
            }
            let mut targets = main
                .iter()
                .map(|g| sm::Target {
                    goal_id: g.clone(),
                    role: sm::Role::Main,
                    note: String::new(),
                })
                .collect::<Vec<_>>();
            targets.extend(gs()?.into_iter().map(|g| sm::Target {
                goal_id: g,
                role: sm::Role::Secondary,
                note: String::new(),
            }));
            let methods = ms()?
                .into_iter()
                .map(|id| sm::MethodRef { id, version: 1 })
                .collect::<Vec<_>>();
            let actual = refs(&f("actual_goals")?, ids)?;
            let actual_methods = refs(&f("actual_methods")?, ids)?
                .into_iter()
                .map(|id| sm::MethodRef { id, version: 1 })
                .collect::<Vec<_>>();
            let summary = if status == "completed" {
                let confirmation = match f("confirmation")?.trim() {
                    "used" => sm::Confirmation::Used,
                    "none" => sm::Confirmation::None,
                    "unknown" => sm::Confirmation::Unknown,
                    _ => {
                        return Err(ApiError::invalid(
                            "已完成记录须明确方法使用情况：used／none／unknown",
                        ));
                    }
                };
                let observation =
                    required(f("observation")?, "已完成记录须有实际表现，不能只填计划")?;
                Some(sm::Summary {
                    targets: actual
                        .into_iter()
                        .map(|goal_id| sm::ActualTarget {
                            goal_id,
                            performed: true,
                            observation: observation.clone(),
                        })
                        .collect(),
                    confirmation,
                    methods: actual_methods,
                    ..Default::default()
                })
            } else {
                if !actual.is_empty() || !actual_methods.is_empty() {
                    return Err(ApiError::invalid("待干预计划不能同时导入实际计数"));
                }
                None
            };
            Ok(Mapped::Session {
                completed: status == "completed",
                input: Box::new(sm::Input {
                    action_versions: Default::default(),
                    business_date: date,
                    time_slot: required(f("time_slot")?, "请选择明确时段：夜间／上午／下午／晚上")?,
                    plan: f("plan")?,
                    targets,
                    methods,
                    focus_goal_ids: vec![],
                    tags: vec![],
                    context: "已核对字段的工作簿导入；完整原文见来源资料".into(),
                    lesson_sources: vec![],
                    action_ids: vec![],
                    summary,
                }),
            })
        }
        "moment" => {
            let date = crate::import::date(f("date")?.trim())?
                .ok_or_else(|| ApiError::invalid("请提供实际日期"))?;
            Ok(Mapped::Moment(
                value!({"business_date":date,"text":title()?,"goal_ids":gs()?}),
            ))
        }
        "lesson" => {
            let date = crate::import::date(f("date")?.trim())?
                .ok_or_else(|| ApiError::invalid("请提供实际日期"))?;
            Ok(Mapped::Lesson {
                date,
                course: required(f("course")?, "课记必须有课程名称")?,
                teacher: required(f("teacher")?, "课记必须有授课老师")?,
                notes: f("description")?,
                teacher_notes: f("teacher_notes")?,
                difficulty_notes: f("difficulty_notes")?,
                targets: gs()?,
                methods: ms()?,
            })
        }
        _ => Err(ApiError::invalid(
            "每张工作表须选择明确类型或仅存档，历史训练文字可存为瞬间观察，不自动计训练次数",
        )),
    }
}
fn map(db: &dyn Sql, actor: &User, input: MapInput) -> Result<Value> {
    let mut p = preview(db, actor, &input.preview_id)?;
    if read_json(db, "prd_import_batches", &p.id)?.is_some() {
        return Err(ApiError::conflict("此预览已经导入，请查看批次回执"));
    }
    let ns = required(input.namespace, "请填写稳定的资料来源标识，重导时保持相同")?;
    if ns.starts_with("skarmar:starter:") || p.input.filename == "skarmar-starter-v1.json" {
        return Err(ApiError::invalid(
            "初始内容请使用专用预览入口，不能改作文件映射",
        ));
    }
    if ns.len() > 200 {
        return Err(ApiError::invalid("资料来源标识过长"));
    }
    if input.rules.len() != p.input.sheets.len() {
        return Err(ApiError::invalid("请为每张工作表核对映射"));
    }
    let mut ids = BTreeMap::new();
    let mut keys = BTreeSet::new();
    let mut sheet_names = BTreeSet::new();
    for sheet in &p.input.sheets {
        if !sheet_names.insert(&sheet.name) {
            return Err(ApiError::invalid("工作表名不能重复"));
        }
        let rules = input
            .rules
            .iter()
            .filter(|r| r.sheet == sheet.name)
            .collect::<Vec<_>>();
        if rules.len() != 1 {
            return Err(ApiError::invalid("工作表映射重复或遗漏"));
        }
        let rule = rules[0];
        let mut occurrences = BTreeMap::<String, usize>::new();
        for row in &sheet.rows {
            let key = if rule.kind == "archive" {
                let h = hash(&json(&(&sheet.headers, &row.cells, &row.formulas))?);
                let n = occurrences.entry(h.clone()).or_default();
                *n += 1;
                format!("{h}:{}", *n)
            } else {
                required(
                    field(rule, &sheet.headers, &row.cells, "key")?,
                    "业务导入必须选择非空且唯一的来源编号列；不能按行号猜测身份",
                )?
            };
            let external = format!("{}:{}", sheet.name, key);
            if !keys.insert(external.clone()) {
                return Err(ApiError::invalid("来源编号重复，请在原表中补充唯一编号"));
            }
            ids.insert(external, stable(&[&ns, &sheet.name, &key]));
        }
    }
    let mut rows = vec![];
    for sheet in &p.input.sheets {
        let rule = input
            .rules
            .iter()
            .find(|r| r.sheet == sheet.name)
            .expect("checked");
        let mut occurrences = BTreeMap::<String, usize>::new();
        for row in &sheet.rows {
            let raw = value!({"headers":sheet.headers,"cells":row.cells,"formulas":row.formulas});
            let key = if rule.kind == "archive" {
                let h = hash(&json(&(&sheet.headers, &row.cells, &row.formulas))?);
                let n = occurrences.entry(h.clone()).or_default();
                *n += 1;
                format!("{h}:{}", *n)
            } else {
                field(rule, &sheet.headers, &row.cells, "key")?
                    .trim()
                    .into()
            };
            let target_id = ids[&format!("{}:{}", sheet.name, key)].clone();
            let mapped = parse_mapped(rule, &sheet.headers, &row.cells, &target_id, &ids)
                .map_err(|e| ApiError(e.0, format!("{} 第{}行：{}", sheet.name, row.row, e.1)))?;
            let fingerprint = hash(&json(&(&raw, &mapped))?);
            let sk = source_key(&ns, &sheet.name, &key)?;
            let existing = db.query(
                "SELECT data FROM prd_import_rows WHERE source_key=?1",
                params![sk],
            )?;
            if let Some(old) = existing.first() {
                let old: Value = serde_json::from_str(&old[0])?;
                if old["fingerprint"] != fingerprint {
                    return Err(ApiError::conflict(
                        "同一来源编号的内容或映射已改变；请在原业务记录中显式修订，导入不会覆盖已有记录",
                    ));
                }
            }
            rows.push(Row {
                key: sk,
                sheet: sheet.name.clone(),
                row: row.row,
                raw,
                fingerprint,
                target_id,
                mapped,
                skipped: !existing.is_empty(),
            });
        }
    }
    // Deterministic dependency order; invalid/cyclic parents still fail atomically.
    rows.sort_by_key(|r| match &r.mapped {
        Mapped::Catalog(_) | Mapped::Existing(_) => 0,
        Mapped::Goal(g) => match g.level {
            crate::model::Level::Long => 0,
            crate::model::Level::Medium => 1,
            crate::model::Level::Short => 2,
        },
        Mapped::Method(_) => 3,
        Mapped::Lesson { .. } | Mapped::Session { .. } => 4,
        Mapped::Knowledge { .. } => 5,
        Mapped::Moment(_) => 6,
        Mapped::Archive => 7,
    });
    // A re-uploaded old definition is NOT consent to bind a new historical
    // record to a definition subsequently edited in the app. Repeated imports
    // may still skip all original records; new dependent records need review.
    for row in rows.iter().filter(|r| !r.skipped) {
        let references: Vec<&String> = match &row.mapped {
            Mapped::Goal(g) => g.parent_id.iter().collect(),
            Mapped::Method(m) => m.goal_ids.iter().collect(),
            Mapped::Knowledge { input, .. } => {
                input.goal_ids.iter().chain(&input.method_ids).collect()
            }
            Mapped::Lesson {
                targets, methods, ..
            } => targets.iter().chain(methods).collect(),
            Mapped::Session { input, .. } => input
                .targets
                .iter()
                .map(|t| &t.goal_id)
                .chain(input.methods.iter().map(|m| &m.id))
                .chain(input.summary.iter().flat_map(|s| {
                    s.targets
                        .iter()
                        .map(|t| &t.goal_id)
                        .chain(s.methods.iter().map(|m| &m.id))
                }))
                .collect(),
            Mapped::Moment(_) => vec![],
            Mapped::Archive | Mapped::Catalog(_) | Mapped::Existing(_) => vec![],
        };
        // Moment references use JSON only at the observation boundary.
        let moment_refs = if let Mapped::Moment(input) = &row.mapped {
            input["goal_ids"]
                .as_array()
                .map(|ids| {
                    ids.iter()
                        .filter_map(Value::as_str)
                        .map(String::from)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        } else {
            vec![]
        };
        for id in references.into_iter().chain(moment_refs.iter()) {
            if let Some(source) = rows.iter().find(|r| {
                r.target_id == *id
                    && r.skipped
                    && matches!(r.mapped, Mapped::Goal(_) | Mapped::Method(_))
            }) {
                let old = db.query(
                    "SELECT data FROM prd_import_rows WHERE source_key=?1",
                    params![source.key],
                )?;
                let stored: Value =
                    serde_json::from_str(&old.first().ok_or_else(ApiError::missing)?[0])?;
                let current = super::get(db, id)?.ok_or_else(ApiError::missing)?;
                if stored["result"]["value"]["version"].as_u64() != Some(current.version()) {
                    return Err(ApiError::conflict(
                        "被引用的目标或方法在上次导入后已修订。不会按旧文件把新历史记录关联到新定义；请先在业务页核对，再另行明确关联",
                    ));
                }
            }
        }
    }
    let target_digest = fingerprint(db)?;
    let digest = hash(&json(&(&ns, &rows, &target_digest))?);
    let plan = Plan {
        namespace: ns,
        rows,
        target_digest,
        digest,
    };
    let result = plan_summary(&p.id, &plan);
    p.plan = Some(plan);
    db.execute(
        "UPDATE prd_import_previews SET data=?2 WHERE id=?1",
        params![p.id, bounded(&p)?],
    )?;
    Ok(result)
}
fn plan_summary(id: &str, plan: &Plan) -> Value {
    value!({"preview_id":id,"digest":plan.digest,"namespace":plan.namespace,"rows":plan.rows.iter().map(|r|value!({"key":r.key,"sheet":r.sheet,"row":r.row,"kind":r.mapped.kind(),"target_id":r.target_id,"skipped":r.skipped,"mapped":r.mapped,"starter_status":r.raw.get("starter_status")})).collect::<Vec<_>>(),"warning":"只追加；经验保留明确映射的阶段，未指定时为想法。文字不换算成训练次数或量化进度。未映射字段完整存档。预览不替代提交时校验。"})
}
fn catalog(db: &dyn Sql, actor: &User, kind: m::CatalogKind, name: &str) -> Result<String> {
    let key = match kind {
        m::CatalogKind::Course => "course",
        m::CatalogKind::Teacher => "teacher",
        m::CatalogKind::Domain => "domain",
    };
    if let Some(row) = db
        .query(
            "SELECT data FROM prd_entities WHERE kind=?1 AND name=?2",
            params![key, name],
        )?
        .first()
    {
        let m::Saved::Catalog(item) = serde_json::from_str(&row[0])? else {
            return Err(ApiError::storage());
        };
        if !item.input.active {
            return Err(ApiError::conflict(
                "同名课程或老师已停用，请先明确恢复再导入",
            ));
        }
        return Ok(item.id);
    }
    let id = Uuid::new_v4().to_string();
    super::write(
        db,
        actor,
        m::Write {
            provided_fields: None,
            method_change: None,
            method_create_mode: None,
            id: id.clone(),
            operation_id: Uuid::new_v4().to_string(),
            expected_version: None,
            change_note: "导入明确课程／老师名称".into(),
            mutation: m::Mutation::Catalog(m::CatalogInput {
                kind,
                name: name.into(),
                active: true,
            }),
        },
    )?;
    Ok(id)
}
fn apply(db: &dyn Sql, actor: &User, row: &Row) -> Result<Option<Value>> {
    let op = Uuid::new_v4().to_string();
    let id = row.target_id.clone();
    Ok(Some(match &row.mapped {
        Mapped::Archive => return Ok(None),
        Mapped::Existing(saved) => serde_json::to_value(saved)?,
        Mapped::Catalog(c) => serde_json::to_value(super::write(
            db,
            actor,
            m::Write {
                provided_fields: None,
                method_change: None,
                method_create_mode: None,
                id,
                operation_id: op,
                expected_version: None,
                change_note: "明确预览并创建初始目录".into(),
                mutation: m::Mutation::Catalog(c.clone()),
            },
        )?)?,
        Mapped::Session { completed, input } => serde_json::to_value(super::sessions::write(
            db,
            actor,
            sm::Write {
                id,
                operation_id: op,
                expected_version: None,
                action: if *completed {
                    sm::Action::Complete
                } else {
                    sm::Action::SaveDraft
                },
                input: Some(*input.clone()),
                change_note: "明确确认来源状态与实际目标，不从文字推断完成".into(),
            },
        )?)?,
        Mapped::Goal(g) => serde_json::to_value(super::write(
            db,
            actor,
            m::Write {
                provided_fields: None,
                method_change: None,
                method_create_mode: None,
                id,
                operation_id: op,
                expected_version: None,
                change_note: "明确映射导入；旧字段见来源存档".into(),
                mutation: m::Mutation::Goal(g.clone()),
            },
        )?)?,
        Mapped::Method(m) => serde_json::to_value(super::write(
            db,
            actor,
            m::Write {
                provided_fields: None,
                method_change: None,
                method_create_mode: None,
                id,
                operation_id: op,
                expected_version: None,
                change_note: "明确映射导入".into(),
                mutation: m::Mutation::Method(m.clone()),
            },
        )?)?,
        Mapped::Knowledge { input, details } => super::followups::command(
            db,
            actor,
            pm::Command::Knowledge(pm::KnowledgeWrite {
                id,
                operation_id: op,
                expected_version: None,
                action: "create".into(),
                input: Some(input.clone()),
                details: details.clone(),
                derived_from: vec![],
                note: "导入，历史依据见完整来源；不生成验证票数".into(),
            }),
        )?,
        Mapped::Moment(input) => super::observations::command(
            db,
            actor,
            super::observation_model::Write {
                id,
                operation_id: op,
                expected_version: None,
                kind: "moment".into(),
                action: "save".into(),
                input: input.clone(),
                note: "导入历史观察；不计训练次数".into(),
            },
        )?,
        Mapped::Lesson {
            date,
            course,
            teacher,
            notes,
            teacher_notes,
            difficulty_notes,
            targets,
            methods,
        } => {
            let course_id = catalog(db, actor, m::CatalogKind::Course, course)?;
            let teacher_id = catalog(db, actor, m::CatalogKind::Teacher, teacher)?;
            let lesson = super::lessons::write(
                db,
                actor,
                lm::LessonWrite {
                    id: id.clone(),
                    operation_id: op,
                    expected_version: None,
                    action: lm::LessonAction::Observation(lm::ObservationInput {
                        business_date: date.clone(),
                        course_id,
                        teacher_id,
                        notes: notes.clone(),
                        teacher_notes: teacher_notes.clone(),
                        difficulty_notes: difficulty_notes.clone(),
                        ..Default::default()
                    }),
                },
            )?;
            if targets.is_empty() && methods.is_empty() {
                serde_json::to_value(lesson)?
            } else {
                serde_json::to_value(super::lessons::write(
                    db,
                    actor,
                    lm::LessonWrite {
                        id,
                        operation_id: Uuid::new_v4().to_string(),
                        expected_version: Some(lesson.organization_version),
                        action: lm::LessonAction::Organization(lm::OrganizationInput {
                            feedback: vec![],
                            feedback_knowledge_versions: Default::default(),
                            seen_observation_version: lesson.observation_version,
                            targets: targets
                                .iter()
                                .map(|g| lm::LessonTarget {
                                    goal_id: g.clone(),
                                    note: String::new(),
                                })
                                .collect(),
                            method_ids: methods.clone(),
                            focus_goal_ids: vec![],
                            processing: lm::Processing::Keep,
                            request_generation: 0,
                            outcome: "导入明确关联，原始字段见来源".into(),
                            new_knowledge: vec![],
                            new_actions: vec![],
                        }),
                    },
                )?)?
            }
        }
    }))
}
fn commit(db: &dyn Sql, actor: &User, input: Confirm) -> Result<Value> {
    let p = preview(db, actor, &input.preview_id)?;
    let plan = p
        .plan
        .ok_or_else(|| ApiError::invalid("请先核对字段映射"))?;
    if plan.digest != input.digest {
        return Err(ApiError::conflict("映射已更改，请核对最新预览"));
    }
    if let Some(receipt) = read_json(db, "prd_import_batches", &p.id)? {
        return Ok(receipt);
    }
    if fingerprint(db)? != plan.target_digest {
        return Err(ApiError::conflict(
            "预览后空间已有改动，请保留映射并重新核对预览",
        ));
    }
    let mut saved = 0;
    let mut archived = 0;
    let mut skipped = 0;
    let mut reused = 0;
    for row in &plan.rows {
        if row.skipped {
            skipped += 1;
            continue;
        }
        let result = apply(db, actor, row)?;
        if matches!(row.mapped, Mapped::Existing(_)) {
            reused += 1;
        } else if result.is_some() {
            saved += 1;
        } else {
            archived += 1;
        }
        let record = value!({"batch_id":p.id,"namespace":plan.namespace,"key":row.key,"sheet":row.sheet,"row":row.row,"fingerprint":row.fingerprint,"raw":row.raw,"mapped":row.mapped,"target_id":if result.is_some(){Some(&row.target_id)}else{None},"result":result,"file_sha256":p.input.sha256,"filename":p.input.filename,"audit":super::followups::audit(actor,"已确认字段映射并导入")});
        db.execute(
            "INSERT INTO prd_import_rows(source_key,data) VALUES(?1,?2)",
            params![row.key, bounded(&record)?],
        )?;
    }
    let result = value!({"batch_id":p.id,"digest":plan.digest,"namespace":plan.namespace,"saved":saved,"reused":reused,"archived":archived,"skipped":skipped,"total":plan.rows.len(),"sources":plan.rows.iter().map(|r|value!({"key":r.key,"sheet":r.sheet,"row":r.row,"kind":r.mapped.kind(),"target_id":if matches!(r.mapped,Mapped::Archive){None}else{Some(&r.target_id)},"skipped":r.skipped})).collect::<Vec<_>>(),"audit":super::followups::audit(actor,"导入完成")});
    db.execute(
        "INSERT INTO prd_import_batches(id,data) VALUES(?1,?2)",
        params![p.id, bounded(&result)?],
    )?;
    Ok(result)
}

/// Hash every business table (including operation IDs and source archives), but
/// exclude expiring previews and auth. A rollback must not discard even an edit
/// later undone to an apparently identical value.
pub fn fingerprint(db: &dyn Sql) -> Result<String> {
    let mut all = BTreeMap::new();
    for row in db.query(
        "SELECT name FROM sqlite_master WHERE type='table' AND name LIKE 'prd_%' ORDER BY name",
        &[],
    )? {
        let name = &row[0];
        if ["prd_import_previews", "prd_upgrade"].contains(&name.as_str()) {
            continue;
        }
        if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(ApiError::storage());
        }
        let cols = db.query(
            &format!("SELECT name FROM pragma_table_info('{name}') ORDER BY cid"),
            &[],
        )?;
        if cols
            .iter()
            .any(|c| !c[0].chars().all(|x| x.is_ascii_alphanumeric() || x == '_'))
        {
            return Err(ApiError::conflict("数据库列结构不在支持范围"));
        }
        let fields = cols
            .iter()
            .map(|c| format!("'{}',\"{}\"", c[0], c[0]))
            .collect::<Vec<_>>()
            .join(",");
        all.insert(
            name.clone(),
            db.query(
                &format!("SELECT json_object({fields}) FROM {name} ORDER BY rowid"),
                &[],
            )?,
        );
    }
    Ok(hash(&json(&all)?))
}

// Upgrade functions follow below. Legacy tables remain byte-for-byte intact.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SessionDecision {
    pub id: String,
    /// archive / planned / completed. A completed source still needs explicit actual targets.
    pub disposition: String,
    #[serde(default)]
    pub actual_goal_ids: Vec<String>,
    #[serde(default)]
    pub observations: BTreeMap<String, String>,
}
fn legacy_rows(db: &dyn Sql) -> Result<BTreeMap<String, Vec<Value>>> {
    let mut names=db.query("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name<>'_cf_KV' ORDER BY name",&[])?;
    if super::enabled(db)? {
        super::initialize(db)?;
        names.retain(|r| !r[0].starts_with("prd_"));
    }
    if names
        .iter()
        .any(|r| !LEGACY.contains(&r[0].as_str()) && !AUTH.contains(&r[0].as_str()))
    {
        return Err(ApiError::conflict(
            "旧库含未知表，请先扩展转换方案，不能自动忽略",
        ));
    }
    let versions = db.query(
        "SELECT CAST(version AS TEXT) FROM schema_migrations ORDER BY version",
        &[],
    )?;
    if versions.iter().map(|r| r[0].as_str()).collect::<Vec<_>>()
        != (1..=10)
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
    {
        return Err(ApiError::conflict("请先将旧库完整升级至版本10，再预览转换"));
    }
    let mut all = BTreeMap::new();
    for name in LEGACY {
        if !names.iter().any(|r| r[0] == *name) {
            return Err(ApiError::conflict("旧库表不完整，转换已中止"));
        }
        // Caches are not business archives; their content is still protected by
        // the upgrade source fingerprint, and the original tables remain intact.
        let cols = db.query(
            &format!("SELECT name FROM pragma_table_info('{name}') ORDER BY cid"),
            &[],
        )?;
        if cols
            .iter()
            .any(|c| !c[0].chars().all(|x| x.is_ascii_alphanumeric() || x == '_'))
        {
            return Err(ApiError::conflict("数据库列结构不在支持范围"));
        }
        let fields = cols
            .iter()
            .map(|r| format!("'{}',\"{}\"", r[0], r[0]))
            .collect::<Vec<_>>()
            .join(",");
        let rows = db.query(
            &format!("SELECT json_object({fields}) FROM {name} ORDER BY rowid"),
            &[],
        )?;
        all.insert(
            name.to_string(),
            rows.into_iter()
                .map(|r| serde_json::from_str(&r[0]).map_err(ApiError::from))
                .collect::<Result<Vec<_>>>()?,
        );
    }
    Ok(all)
}
fn legacy_digest(db: &dyn Sql) -> Result<String> {
    Ok(hash(&json(&legacy_rows(db)?)?))
}
fn upgrade_preview(db: &dyn Sql) -> Result<Value> {
    let raw = legacy_rows(db)?;
    let state = crate::store::state(db)?;
    Ok(
        value!({"schema":"skarmar.upgrade.preview.v1","source_digest":hash(&json(&raw)?),"counts":raw.iter().map(|(k,v)|(k.clone(),v.len())).collect::<BTreeMap<_,_>>(),"sessions":state.sessions,"warning":"原表和账号保留；目标、方法、课记、经验困难转成有来源的基线。每条家庭记录必须确认仅存档／待干预计划／已完成及实际目标；方法使用情况默认为未知。不从文字提取频次、反馈票数或测量。备份并停止其他客户端后执行。"}),
    )
}
/// Only conversion uses this view: temporarily hide retained legacy/auth tables
/// from the strict fresh-schema initializer. SQL writes still reach the SAME transaction.
struct NewSchema<'a>(&'a dyn Sql);
impl Sql for NewSchema<'_> {
    fn execute(&self, s: &str, p: &[crate::sql::Value]) -> Result<()> {
        self.0.execute(s, p)
    }
    fn execute_batch(&self, s: &str) -> Result<()> {
        self.0.execute_batch(s)
    }
    fn query(&self, s: &str, p: &[crate::sql::Value]) -> Result<Vec<Vec<String>>> {
        let mut r = self.0.query(s, p)?;
        if s.starts_with("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE") {
            r.retain(|r| r[0].starts_with("prd_"));
        }
        Ok(r)
    }
}
/// Explicit deployment baseline, NOT a conversion: leave every legacy/auth row
/// in place and start an empty PRD domain. The adapter MUST supply a transaction.
/// Ordinary startup still rejects legacy stores without this configuration.
pub fn initialize_empty_preserving_legacy(db: &dyn Sql) -> Result<()> {
    if super::enabled(db)? {
        // The flag can remain configured: never reset an existing PRD database.
        return super::initialize_authenticated(db);
    }
    let names = db.query(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name<>'_cf_KV'",
        &[],
    )?;
    if names.len() != LEGACY.len() + AUTH.len()
        || names
            .iter()
            .any(|r| !LEGACY.contains(&r[0].as_str()) && !AUTH.contains(&r[0].as_str()))
        || LEGACY
            .iter()
            .chain(AUTH)
            .any(|n| !names.iter().any(|r| &r[0] == n))
    {
        return Err(ApiError::conflict(
            "空白新基线要求完整的既有旧库和账号表，拒绝创建或忽略未知表",
        ));
    }
    let versions = db.query(
        "SELECT CAST(version AS TEXT) FROM schema_migrations ORDER BY version",
        &[],
    )?;
    if versions.iter().map(|r| r[0].clone()).collect::<Vec<_>>()
        != (1..=10).map(|n| n.to_string()).collect::<Vec<_>>()
        || db
            .query(
                "SELECT id FROM auth_users WHERE role='admin' AND active=1 LIMIT 1",
                &[],
            )?
            .is_empty()
    {
        return Err(ApiError::conflict(
            "空白新基线要求旧库版本1至10及已有有效管理员",
        ));
    }
    // Only metadata was read. No business data or authentication row is copied,
    // changed, removed, inferred, or exposed in a deployment receipt.
    super::initialize(&NewSchema(db))?;
    db.execute(
        "INSERT INTO prd_upgrade(id,data) VALUES('legacy-v10',?1)",
        params![json(&value!({
            "schema":"skarmar.empty-baseline.v1", "legacy_tables_preserved":true
        }))?],
    )?;
    super::initialize_authenticated(db)
}
pub fn filter_retained_legacy(db: &dyn Sql, names: &mut Vec<Vec<String>>) -> Result<()> {
    if !names.iter().any(|r| LEGACY.contains(&r[0].as_str())) {
        return Ok(());
    }
    if !names.iter().any(|r| r[0] == "prd_upgrade") {
        return Ok(());
    }
    let marker = read_json(db, "prd_upgrade", "legacy-v10")?;
    if !marker.as_ref().is_some_and(|m| {
        m["schema"] == "skarmar.upgrade.v1"
            || (m["schema"] == "skarmar.empty-baseline.v1" && m["legacy_tables_preserved"] == true)
    }) {
        return Err(ApiError::conflict("旧表保留标记不完整，拒绝启动"));
    }
    if names
        .iter()
        .filter(|r| LEGACY.contains(&r[0].as_str()))
        .count()
        != LEGACY.len()
    {
        return Err(ApiError::conflict("转换保留的旧表不完整，拒绝启动"));
    }
    names.retain(|r| !LEGACY.contains(&r[0].as_str()));
    Ok(())
}
fn goal_input(g: &crate::model::Goal) -> Result<m::GoalInput> {
    let state = match g.progress_status {
        Some(crate::model::GoalProgress::NotStarted) if g.level != crate::model::Level::Long => {
            "not_started"
        }
        Some(crate::model::GoalProgress::Achieved | crate::model::GoalProgress::Completed)
            if g.level != crate::model::Level::Long =>
        {
            "achieved"
        }
        _ => "active",
    };
    Ok(serde_json::from_value(
        value!({"title":g.title,"level":g.level,"meaning":g.review_notes,"criteria":g.criteria,"parent_id":g.parent_id,"archived":g.status==crate::model::GoalStatus::Archived,"state":state,"attention_start":g.start_date}),
    )?)
}
fn converted_goal(g: &crate::model::Goal) -> Result<m::Goal> {
    Ok(m::Goal {
        id: g.id.clone(),
        version: g.version,
        input: goal_input(g)?,
    })
}
fn upgrade_commit(db: &dyn Sql, actor: &User, input: UpgradeInput) -> Result<Value> {
    if input.note.trim().is_empty() {
        return Err(ApiError::invalid("请记录本次转换的核对和回滚说明"));
    }
    if legacy_digest(db)? != input.source_digest {
        return Err(ApiError::conflict(
            "旧数据在预览后已变化，转换未开始，请重新预览",
        ));
    }
    let source = legacy_rows(db)?;
    let state = crate::store::state(db)?;
    if state.goals.len()
        + state.pathways.len()
        + state.sessions.len()
        + state.lessons.len()
        + state.experiences.len()
        + state.difficulties.len()
        > LIMIT
    {
        return Err(ApiError::invalid(
            "自动转换当前限1000条业务对象；大库须单独分阶段方案",
        ));
    }
    let decisions = input
        .sessions
        .iter()
        .map(|s| (&s.id, s))
        .collect::<BTreeMap<_, _>>();
    if decisions.len() != input.sessions.len()
        || decisions.len() != state.sessions.len()
        || state
            .sessions
            .iter()
            .any(|s| !decisions.contains_key(&s.id))
    {
        return Err(ApiError::invalid(
            "必须逐条确认全部旧家庭记录的用途，不能缺省推断完成",
        ));
    }
    super::initialize(&NewSchema(db))?;
    let batch = Uuid::new_v4().to_string();
    let mut goals = state.goals.clone();
    goals.sort_by_key(|g| match g.level {
        crate::model::Level::Long => 0,
        crate::model::Level::Medium => 1,
        crate::model::Level::Short => 2,
    });
    for g in &goals {
        super::write(
            db,
            actor,
            m::Write {
                provided_fields: None,
                method_change: None,
                method_create_mode: None,
                id: g.id.clone(),
                operation_id: Uuid::new_v4().to_string(),
                expected_version: None,
                change_note: "旧模型转换基线；完整原始字段和历史保留".into(),
                mutation: m::Mutation::Goal(goal_input(g)?),
            },
        )?;
        let saved = m::Saved::Goal(converted_goal(g)?);
        db.execute(
            "UPDATE prd_entities SET version=?2,data=?3 WHERE id=?1",
            params![g.id, g.version as i64, json(&saved)?],
        )?;
        // Preserve known historical goal versions and snapshots. Unknown history
        // is never filled in with today's definition.
        db.execute(
            "DELETE FROM prd_revisions WHERE entity_id=?1",
            params![g.id],
        )?;
        let mut hist = crate::store::goal_history(db, &g.id)?;
        if !hist.iter().any(|h| h.goal.version == g.version) {
            hist.push(crate::model::GoalRevision {
                goal: g.clone(),
                recorded_at: chrono::Utc::now().to_rfc3339(),
                change_note: "转换时当前基线".into(),
                source_session_id: None,
            });
        }
        for h in hist {
            let op = Uuid::new_v4().to_string();
            let entity = m::Saved::Goal(converted_goal(&h.goal)?);
            db.execute(
                "INSERT INTO prd_operations(id,actor_id,request,response) VALUES(?1,?2,?3,?4)",
                params![op, actor.id, "legacy-goal-revision", json(&entity)?],
            )?;
            db.execute("INSERT INTO prd_revisions(entity_id,version,operation_id,data) VALUES(?1,?2,?3,?4)",params![g.id,h.goal.version as i64,op,json(&m::Revision{entity,operation_id:op.clone(),actor_id:actor.id.clone(),recorded_at:h.recorded_at,event:"legacy_baseline".into(),change_note:format!("旧历史转换；{}",h.change_note)})?])?;
        }
        super::reviews::sync(db, "goal", &g.id)?;
        // Explicit manual next date is retained; completed review history is
        // retained as old source, not invented as a new-model review decision.
        if let Some(date) = &g.next_review_date {
            crate::store::valid_date(date)?;
            let rows = db.query(
                "SELECT data FROM prd_schedules WHERE object_kind='goal' AND object_id=?1",
                params![g.id],
            )?;
            if let Some(r) = rows.first() {
                let mut s: pm::Schedule = serde_json::from_str(&r[0])?;
                s.due_date = Some(date.clone());
                s.manual = true;
                s.note = "保留旧目标明确的下次复盘日期".into();
                db.execute(
                    "UPDATE prd_schedules SET data=?2 WHERE object_kind='goal' AND object_id=?1",
                    params![g.id, json(&s)?],
                )?;
                db.execute("UPDATE prd_schedule_revisions SET data=?2 WHERE object_kind='goal' AND object_id=?1 AND version=?3",params![g.id,json(&s)?,s.version as i64])?;
            }
        }
    }
    for method in &state.pathways {
        let input: m::MethodInput = serde_json::from_value(
            value!({"title":method.title,"description":method.description,"steps":method.steps,"goal_ids":method.goal_ids,"state":if method.status==crate::model::PathwayStatus::Archived{"retired"}else{"active"}}),
        )?;
        let mut saved = super::write(
            db,
            actor,
            m::Write {
                provided_fields: None,
                method_change: None,
                method_create_mode: None,
                id: method.id.clone(),
                operation_id: Uuid::new_v4().to_string(),
                expected_version: None,
                change_note: "旧方法转换基线".into(),
                mutation: m::Mutation::Method(input),
            },
        )?;
        if let m::Saved::Method(m) = &mut saved {
            m.version = method.version;
        }
        db.execute(
            "UPDATE prd_entities SET version=?2,data=?3 WHERE id=?1",
            params![method.id, method.version as i64, json(&saved)?],
        )?;
        // No earlier method versions were stored by the legacy schema.
        let hist = super::history(db, &method.id)?;
        let mut rev = hist[0].clone();
        rev.entity = saved;
        rev.event = "legacy_baseline".into();
        db.execute(
            "UPDATE prd_revisions SET version=?2,data=?3 WHERE entity_id=?1",
            params![method.id, method.version as i64, json(&rev)?],
        )?;
        super::reviews::sync(db, "method", &method.id)?;
    }
    // Create exact historical lesson names before restoring inactive directory
    // status. A renamed option does not rewrite old lesson snapshots.
    for l in &state.lessons {
        let row = Row {
            key: String::new(),
            sheet: "lessons".into(),
            row: 0,
            raw: Value::Null,
            fingerprint: String::new(),
            target_id: l.id.clone(),
            skipped: false,
            mapped: Mapped::Lesson {
                date: l.input.business_date.clone(),
                course: l.input.course.clone(),
                teacher: l.input.teacher.clone(),
                notes: format!("{}\n旧课记目标说明：{}", l.input.content, l.input.goals),
                teacher_notes: l.input.home_cooperation.clone(),
                difficulty_notes: l.input.problems.clone(),
                targets: l.input.links.as_ref().map_or(vec![], |links| {
                    links.targets.iter().map(|t| t.goal_id.clone()).collect()
                }),
                methods: l
                    .input
                    .links
                    .as_ref()
                    .map_or(vec![], |links| links.pathway_ids.clone()),
            },
        };
        apply(db, actor, &row)?;
        // Historical explicit goal snapshots remain their saved versions.
        let mut lesson = super::lessons::get_lesson(db, &l.id)?.ok_or_else(ApiError::missing)?;
        for target in &mut lesson.organization.targets {
            if let Some(old) = l
                .target_snapshots
                .iter()
                .find(|t| t.goal_id == target.goal.id)
            {
                target.goal = converted_goal(&old.goal_snapshot)?;
                target.ancestors.clear();
            }
        }
        db.execute(
            "UPDATE prd_lessons SET data=?2 WHERE id=?1",
            params![l.id, json(&lesson)?],
        )?;
        db.execute("UPDATE prd_lesson_revisions SET data=json_set(data,'$.lesson',json(?2)) WHERE lesson_id=?1 AND version=?3",params![l.id,json(&lesson)?,lesson.version as i64])?;
    }
    for option in &state.lesson_options {
        let kind = match option.kind.as_str() {
            "course" => m::CatalogKind::Course,
            "teacher" => m::CatalogKind::Teacher,
            _ => return Err(ApiError::invalid("未知旧课程目录类型")),
        };
        let id = catalog(db, actor, kind, &option.value)?;
        if !option.active {
            let Some(m::Saved::Catalog(item)) = super::get(db, &id)? else {
                return Err(ApiError::storage());
            };
            let mut input = item.input;
            input.active = false;
            super::write(
                db,
                actor,
                m::Write {
                    provided_fields: None,
                    method_change: None,
                    method_create_mode: None,
                    id,
                    operation_id: Uuid::new_v4().to_string(),
                    expected_version: Some(item.version),
                    change_note: "保留旧目录停用状态和历史名称".into(),
                    mutation: m::Mutation::Catalog(input),
                },
            )?;
        }
    }
    let mut formal = 0;
    let mut archived = 0;
    for old in &state.sessions {
        let d = decisions[&old.id];
        if d.disposition == "archive" {
            if !d.actual_goal_ids.is_empty() {
                return Err(ApiError::invalid("仅存档不选择实际目标"));
            }
            archived += 1;
            continue;
        }
        if !["planned", "completed"].contains(&d.disposition.as_str()) {
            return Err(ApiError::invalid("请选择仅存档、待干预或已完成"));
        }
        if d.disposition == "completed" && old.status != "completed" {
            return Err(ApiError::invalid(
                "旧记录并非已完成，不能在转换时补造完成状态",
            ));
        }
        let targets = old
            .input
            .targets
            .iter()
            .map(|t| sm::Target {
                goal_id: t.goal_id.clone(),
                role: match t.role {
                    crate::model::Role::Primary => sm::Role::Main,
                    crate::model::Role::Secondary => sm::Role::Secondary,
                    crate::model::Role::Review => sm::Role::Review,
                },
                note: t.plan.clone(),
            })
            .collect::<Vec<_>>();
        if d.actual_goal_ids.iter().collect::<BTreeSet<_>>().len() != d.actual_goal_ids.len()
            || d.actual_goal_ids
                .iter()
                .any(|id| !old.input.targets.iter().any(|t| t.goal_id == *id))
        {
            return Err(ApiError::invalid(
                "实际目标须明确来自原记录，不能猜测额外关联",
            ));
        }
        if d.disposition == "planned" && !d.actual_goal_ids.is_empty() {
            return Err(ApiError::invalid("待干预计划不生成实际目标"));
        }
        let summary = if d.disposition == "completed" {
            Some(sm::Summary {
                targets: old
                    .input
                    .targets
                    .iter()
                    .map(|t| sm::ActualTarget {
                        goal_id: t.goal_id.clone(),
                        performed: d.actual_goal_ids.contains(&t.goal_id),
                        observation: d
                            .observations
                            .get(&t.goal_id)
                            .cloned()
                            .unwrap_or_else(|| t.progress.clone()),
                    })
                    .collect(),
                confirmation: sm::Confirmation::Unknown,
                context: format!("旧模型转换，方法使用情况未记录。{}", old.input.observation),
                ..Default::default()
            })
        } else {
            None
        };
        let input = sm::Input {
            action_versions: Default::default(),
            business_date: old.input.business_date.clone(),
            time_slot: old.input.time_slot.clone(),
            plan: old.input.plan.clone(),
            targets,
            methods: vec![],
            focus_goal_ids: vec![],
            tags: vec![],
            context: format!(
                "旧记录转换基线；组织方式原文：{}；活动原文：{}",
                old.input.organization,
                old.input.activities.join("、")
            ),
            lesson_sources: vec![],
            action_ids: vec![],
            summary,
        };
        let mut saved = super::sessions::write(
            db,
            actor,
            sm::Write {
                id: old.id.clone(),
                operation_id: Uuid::new_v4().to_string(),
                expected_version: None,
                action: if d.disposition == "completed" {
                    sm::Action::Complete
                } else {
                    sm::Action::SaveDraft
                },
                input: Some(input),
                change_note: "逐条确认旧记录用途；未推断方法次数".into(),
            },
        )?;
        for target in saved
            .planned_targets
            .iter_mut()
            .chain(&mut saved.actual_targets)
        {
            if let Some(snapshot) = old
                .target_snapshots
                .iter()
                .find(|t| t.input.goal_id == target.goal.id)
            {
                target.goal = converted_goal(&snapshot.goal_snapshot)?;
                target.ancestors.clear();
            }
        }
        db.execute(
            "UPDATE prd_sessions SET data=?2 WHERE id=?1",
            params![old.id, json(&saved)?],
        )?;
        db.execute("UPDATE prd_session_revisions SET data=json_set(data,'$.session',json(?2)) WHERE session_id=?1",params![old.id,json(&saved)?])?;
        formal += 1;
    }
    for (kind, records) in [
        ("experience", &state.experiences),
        ("difficulty", &state.difficulties),
    ] {
        for k in records {
            let details = pm::Details {
                title: k.content.chars().take(120).collect(),
                evidence: k.conclusion.clone(),
                ..Default::default()
            };
            let stage = if kind == "experience" {
                Some(if k.status == "verified" {
                    "helpful"
                } else if k.source_session_id.is_some() || k.source_lesson_id.is_some() {
                    "try"
                } else {
                    "idea"
                })
            } else {
                None
            };
            let input: lm::NewKnowledge = serde_json::from_value(
                value!({"id":k.id,"kind":kind,"content":k.content,"goal_ids":k.goal_ids,"method_ids":k.pathway_ids,"next_review_date":k.next_review_date,"stage":stage}),
            )?;
            let mut details = details;
            if kind == "difficulty" {
                details.state = match k.status.as_str() {
                    "resolved" => "resolved",
                    "active" | "following" | "in_progress" => "following",
                    _ => "pending",
                }
                .into();
            } else if k.status == "stopped" {
                details.state = "retired".into();
            }
            super::followups::command(
                db,
                actor,
                pm::Command::Knowledge(pm::KnowledgeWrite {
                    id: k.id.clone(),
                    operation_id: Uuid::new_v4().to_string(),
                    expected_version: None,
                    action: "create".into(),
                    input: Some(input),
                    details,
                    derived_from: vec![],
                    note: "旧知识转换；来源和旧状态存档，未创建验证票数".into(),
                }),
            )?;
            let mut saved = super::followups::get(db, &k.id)?;
            if !k.source_deleted {
                if let Some(id) = &k.source_session_id
                    && let Some(s) = super::sessions::get_session(db, id)?
                {
                    saved.source_session_id = Some(id.clone());
                    saved.source_session_version = Some(s.version);
                    saved.source_summary = s.input.summary;
                } else if let Some(id) = &k.source_lesson_id
                    && let Some(l) = super::lessons::get_lesson(db, id)?
                {
                    saved.source_lesson_id = Some(id.clone());
                    saved.source_observation_version = l.observation_version;
                    saved.source_observation = Some(l.observation);
                }
            }
            db.execute("UPDATE prd_knowledge SET source_lesson_id=?2,source_session_id=?3,data=?4 WHERE id=?1",params![k.id,saved.source_lesson_id,saved.source_session_id,json(&saved)?])?;
            db.execute("UPDATE prd_output_revisions SET data=?2 WHERE kind='knowledge' AND id=?1 AND version=1",params![k.id,json(&saved)?])?;
        }
    }
    let mut archived_rows = 0;
    for (table, rows) in &source {
        if ["operations", "import_previews"].contains(&table.as_str()) {
            continue;
        }
        for (i, raw) in rows.iter().enumerate() {
            let key = source_key("legacy-v10", table, &i.to_string())?;
            let record = value!({"batch_id":batch,"namespace":"legacy-v10","sheet":table,"row":i+1,"key":key,"raw":raw,"fingerprint":hash(&json(raw)?),"target_id":raw.get("id"),"audit":super::followups::audit(actor,"原始旧模型资料完整保留；不是新实践次数")});
            db.execute(
                "INSERT INTO prd_import_rows(source_key,data) VALUES(?1,?2)",
                params![key, bounded(&record)?],
            )?;
            archived_rows += 1;
        }
    }
    let result = value!({"batch_id":batch,"kind":"legacy_upgrade","goals":state.goals.len(),"methods":state.pathways.len(),"lessons":state.lessons.len(),"knowledge":state.experiences.len()+state.difficulties.len(),"sessions":formal,"archived_sessions":archived,"archived_rows":archived_rows,"source_digest":input.source_digest,"decisions":input.sessions,"audit":super::followups::audit(actor,&input.note)});
    db.execute(
        "INSERT INTO prd_import_batches(id,data) VALUES(?1,?2)",
        params![batch, bounded(&result)?],
    )?;
    let baseline = fingerprint(db)?;
    let marker = value!({"schema":"skarmar.upgrade.v1","source_digest":input.source_digest,"baseline":baseline,"receipt":result,"audit":super::followups::audit(actor,&input.note)});
    db.execute(
        "INSERT INTO prd_upgrade(id,data) VALUES('legacy-v10',?1)",
        params![bounded(&marker)?],
    )?;
    // Re-run strict startup validation BEFORE committing the original transaction.
    super::initialize_authenticated(db)?;
    Ok(
        value!({"receipt":result,"baseline":baseline,"next":"维护模式中将 SKARMA_MODEL 切换为 prd_v1；原 Worker、DO、space、域名与账号均保持。任何新业务写入后禁止自动回滚。"}),
    )
}
fn rollback(db: &dyn Sql, baseline: &str, note: &str) -> Result<Value> {
    let marker = read_json(db, "prd_upgrade", "legacy-v10")?.ok_or_else(ApiError::missing)?;
    if note.trim().is_empty() || marker["baseline"] != baseline || fingerprint(db)? != baseline {
        return Err(ApiError::conflict(
            "新模型已有写入或基线不匹配，禁止丢弃新数据回滚；请采用前向修复或人工对账",
        ));
    }
    if legacy_digest(db)? != marker["source_digest"] {
        return Err(ApiError::conflict("保留旧表已有变化，禁止自动回滚"));
    }
    // Fixed reverse dependency order; never include auth or a client table name.
    for table in [
        "prd_observation_revisions",
        "prd_observations",
        "prd_annotations",
        "prd_review_revisions",
        "prd_schedule_revisions",
        "prd_feedback_revisions",
        "prd_revisions",
        "prd_lesson_revisions",
        "prd_session_revisions",
        "prd_method_goals",
        "prd_lesson_refs",
        "prd_output_revisions",
        "prd_knowledge",
        "prd_actions",
        "prd_reviews",
        "prd_feedback",
        "prd_schedules",
        "prd_settings",
        "prd_sessions",
        "prd_lessons",
        "prd_entities",
        "prd_operations",
        "prd_import_rows",
        "prd_import_batches",
        "prd_import_previews",
        "prd_upgrade",
        "prd_schema",
    ] {
        db.execute_batch(&format!("DROP TABLE {table};"))?;
    }
    crate::store::initialize(db)?;
    Ok(
        value!({"rolled_back":true,"note":note,"next":"维护模式中恢复 SKARMA_MODEL=legacy；原账号和旧表保留"}),
    )
}
