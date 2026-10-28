//! Explicit starter content, using the existing transactional import ledger.
use super::*;

const NS: &str = "skarmar:starter:family-v1";
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    pub keys: Vec<String>,
}
#[derive(Deserialize)]
struct Entry {
    key: String,
    input: Value,
    #[serde(default)]
    goal_keys: Vec<String>,
}
#[derive(Deserialize)]
struct Pack {
    goals: Vec<Entry>,
    methods: Vec<Entry>,
}
struct Binding {
    id: String,
    imported: bool,
    available: bool,
    saved: Option<m::Saved>,
}
pub(super) fn catalog_kind(kind: &m::CatalogKind) -> &'static str {
    match kind {
        m::CatalogKind::Course => "course",
        m::CatalogKind::Teacher => "teacher",
        m::CatalogKind::Domain => "domain",
    }
}
fn entries() -> Result<Vec<(String, Entry)>> {
    let pack: Pack = serde_json::from_str(include_str!(
        "../../../../data/seeds/family-starter-v1.json"
    ))?;
    let mut rows: Vec<_> = pack
        .goals
        .into_iter()
        .map(|e| ("goal".into(), e))
        .chain(pack.methods.into_iter().map(|e| ("method".into(), e)))
        .collect();
    let catalog: Value =
        serde_json::from_str(include_str!("../../../../data/seeds/initial-catalog.json"))?;
    for c in catalog["catalog"]
        .as_array()
        .ok_or_else(ApiError::storage)?
    {
        rows.push((
            c["kind"].as_str().ok_or_else(ApiError::storage)?.into(),
            Entry {
                key: format!(
                    "catalog.{}",
                    c["key"].as_str().ok_or_else(ApiError::storage)?
                ),
                input: value!({"kind":c["kind"],"name":c["name"],"active":c["active"]}),
                goal_keys: vec![],
            },
        ));
    }
    Ok(rows)
}
pub(super) fn preview(db: &dyn Sql, actor: &User, input: Selection) -> Result<Value> {
    let entries = entries()?;
    let selected: BTreeSet<_> = input.keys.iter().cloned().collect();
    if selected.is_empty()
        || selected.len() != input.keys.len()
        || selected
            .iter()
            .any(|key| !entries.iter().any(|(_, e)| &e.key == key))
    {
        return Err(ApiError::invalid("请选择不重复且有效的初始内容"));
    }
    let mut bindings = BTreeMap::new();
    for (kind, e) in &entries {
        if !selected.contains(&e.key) && kind != "goal" {
            continue;
        }
        let sk = source_key(NS, "初始参考", &e.key)?;
        let marker = db.query(
            "SELECT data FROM prd_import_rows WHERE source_key=?1",
            params![sk],
        )?;
        let imported = !marker.is_empty();
        let (id, saved, available) = if let Some(row) = marker.first() {
            let old: Value = serde_json::from_str(&row[0])?;
            let id = old["target_id"]
                .as_str()
                .ok_or_else(ApiError::storage)?
                .to_string();
            let current = super::super::get(db, &id)?;
            let available = current.is_some();
            // Keep the original receipt for a removed item, never recreate it.
            (
                id,
                current.or(Some(serde_json::from_value(old["result"].clone())?)),
                available,
            )
        } else {
            let title = e.input[if kind == "goal" || kind == "method" {
                "title"
            } else {
                "name"
            }]
            .as_str()
            .ok_or_else(ApiError::storage)?;
            let matches = if selected.contains(&e.key) {
                db.query(
                    "SELECT data FROM prd_entities WHERE kind=?1 AND name=?2",
                    params![kind.as_str(), title],
                )?
            } else {
                vec![]
            };
            if matches.len() > 1 {
                return Err(ApiError::conflict(&format!(
                    "有多个同名项目「{title}」，请先在原页面核对；初始内容不会猜测合并"
                )));
            }
            let saved: Option<m::Saved> = matches
                .first()
                .map(|r| serde_json::from_str(&r[0]))
                .transpose()?;
            if matches!(&saved, Some(m::Saved::Goal(g)) if g.input.level != crate::model::Level::Long)
            {
                return Err(ApiError::conflict(
                    "已有同名目标不是长期方向，请先在目标页核对；初始内容不会改变目标期限",
                ));
            }
            (
                saved
                    .as_ref()
                    .map(|s| s.id().to_string())
                    .unwrap_or_else(|| Uuid::new_v4().to_string()),
                saved,
                true,
            )
        };
        bindings.insert(
            e.key.clone(),
            Binding {
                id,
                imported,
                available,
                saved,
            },
        );
    }
    let mut rows = vec![];
    for (kind, e) in &entries {
        if !selected.contains(&e.key) {
            continue;
        }
        let b = &bindings[&e.key];
        let mapped = if let Some(saved) = &b.saved {
            Mapped::Existing(Box::new(saved.clone()))
        } else {
            match kind.as_str() {
                "goal" => Mapped::Goal(serde_json::from_value(e.input.clone())?),
                "method" => {
                    let mut method: m::MethodInput = serde_json::from_value(e.input.clone())?;
                    for key in &e.goal_keys {
                        let target = bindings.get(key).filter(|g| g.available && (g.imported || selected.contains(key))).ok_or_else(|| ApiError::invalid("所选方法的关联方向尚未选择或已删除，请先勾选这些方向，或在方法页面自行调整关联"))?;
                        method.goal_ids.push(target.id.clone());
                    }
                    Mapped::Method(method)
                }
                _ => Mapped::Catalog(serde_json::from_value(e.input.clone())?),
            }
        };
        let status = if !b.available {
            "removed"
        } else if b.imported {
            "already_added"
        } else if b.saved.is_some() {
            "reuse"
        } else {
            "new"
        };
        let raw = value!({"source":NS,"reference_key":e.key,"starter_status":status,"reference":e.input,"goal_keys":e.goal_keys});
        rows.push(Row {
            key: source_key(NS, "初始参考", &e.key)?,
            sheet: "初始参考".into(),
            row: rows.len() + 1,
            fingerprint: hash(&json(&(&raw, &mapped))?),
            raw,
            target_id: b.id.clone(),
            mapped,
            skipped: b.imported,
        });
    }
    let target_digest = fingerprint(db)?;
    let digest = hash(&json(&(NS, &rows, &target_digest))?);
    let p = Preview {
        id: Uuid::new_v4().to_string(),
        actor_id: actor.id.clone(),
        expires_at: (chrono::Utc::now() + chrono::Duration::hours(1)).to_rfc3339(),
        input: serde_json::from_value(
            value!({"filename":"skarmar-starter-v1.json","sha256":hash(&json(&rows)?),"sheets":[]}),
        )?,
        plan: Some(Plan {
            namespace: NS.into(),
            rows,
            target_digest,
            digest,
        }),
    };
    db.execute(
        "INSERT INTO prd_import_previews(id,data) VALUES(?1,?2)",
        params![p.id, bounded(&p)?],
    )?;
    Ok(plan_summary(&p.id, p.plan.as_ref().expect("starter plan")))
}
