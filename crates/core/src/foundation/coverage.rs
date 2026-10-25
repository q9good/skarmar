//! Coverage is recorded opportunity, never a measure of progress or efficacy.
use super::{
    model::{Frequency, GoalState},
    session_model::Confirmation,
};
use crate::{
    sql::{Sql, params},
    store::{ApiError, Result},
};
use chrono::{Duration, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Request {
    pub days: u32,
    pub source: String,
}
#[derive(Debug, Serialize)]
pub struct Counts {
    pub family: u32,
    pub lesson: u32,
    pub selected: u32,
}
#[derive(Debug, Serialize)]
pub struct GoalCoverage {
    pub id: String,
    pub counts: Counts,
    pub direct: Counts,
    pub descendants: Counts,
    pub focus: Counts,
    pub last_date: Option<String>,
    pub attention_anchor: String,
    pub ratio: Option<f64>,
    pub band: u8,
}
#[derive(Debug, Serialize)]
pub struct Report {
    pub days: u32,
    pub source: String,
    pub start: String,
    pub end: String,
    pub data_version: u64,
    pub records: Counts,
    pub goals: Vec<GoalCoverage>,
    pub methods: BTreeMap<String, Counts>,
    pub method_unknown: Counts,
    pub no_library_method: Counts,
    pub tags: BTreeMap<String, u32>,
    pub unclassified: u32,
    pub heatmap: BTreeMap<String, u32>,
    pub suggested_ids: Vec<String>,
}
struct Record {
    family: bool,
    day: String,
    direct: HashSet<String>,
    below: HashSet<String>,
    focus: HashSet<String>,
    methods: Option<Vec<String>>,
    tags: Vec<String>,
}
fn selected(f: u32, l: u32, source: &str) -> u32 {
    match source {
        "family" => f,
        "lesson" => l,
        _ => f + l,
    }
}
fn counts(records: &[Record], source: &str, predicate: impl Fn(&Record) -> bool) -> Counts {
    let mut f = 0;
    let mut l = 0;
    for r in records {
        if predicate(r) {
            if r.family { f += 1 } else { l += 1 }
        }
    }
    Counts {
        family: f,
        lesson: l,
        selected: selected(f, l, source),
    }
}
fn scope(r: &Record, source: &str) -> bool {
    source == "combined" || (source == "family") == r.family
}
fn band(n: u32, days: u32) -> u8 {
    let edges = match days {
        7 => [1, 2, 3, 5],
        30 => [1, 2, 4, 8],
        _ => [1, 4, 9, 21],
    };
    edges.into_iter().filter(|e| n >= *e).count() as u8
}
pub fn report(db: &dyn Sql, request: Request) -> Result<Report> {
    report_at(db, request, &super::sessions::today())
}
pub fn report_at(db: &dyn Sql, request: Request, today: &str) -> Result<Report> {
    if ![7, 30, 90].contains(&request.days)
        || !["family", "lesson", "combined"].contains(&request.source.as_str())
    {
        return Err(ApiError::invalid("请选择7／30／90天和家庭／机构／合计"));
    }
    let date =
        NaiveDate::parse_from_str(today, "%Y-%m-%d").map_err(|_| ApiError::invalid("日期无效"))?;
    let start = (date - Duration::days(i64::from(request.days) - 1)).to_string();
    let snapshot = super::snapshot(db)?;
    let mut all = vec![];
    for s in &snapshot.sessions {
        if s.status != "completed" || s.input.business_date.as_str() > today {
            continue;
        }
        let summary = s.input.summary.as_ref().ok_or_else(ApiError::storage)?;
        all.push(Record {
            family: true,
            day: s.input.business_date.clone(),
            direct: s.actual_targets.iter().map(|t| t.goal.id.clone()).collect(),
            below: s
                .actual_targets
                .iter()
                .flat_map(|t| t.ancestors.iter().map(|g| g.id.clone()))
                .collect(),
            focus: s.focus_goals.iter().map(|g| g.id.clone()).collect(),
            methods: if summary.confirmation == Confirmation::Unknown {
                None
            } else {
                Some(s.actual_methods.iter().map(|m| m.id.clone()).collect())
            },
            tags: summary.tags.clone(),
        });
    }
    for l in &snapshot.lessons {
        if l.status != "recorded" || l.observation.business_date.as_str() > today {
            continue;
        }
        all.push(Record {
            family: false,
            day: l.observation.business_date.clone(),
            direct: l
                .organization
                .targets
                .iter()
                .map(|t| t.goal.id.clone())
                .collect(),
            below: l
                .organization
                .targets
                .iter()
                .flat_map(|t| {
                    let mut ids = t.ancestors.iter().map(|g| g.id.clone()).collect::<Vec<_>>();
                    if let Some(id) = &t.goal.input.parent_id {
                        ids.push(id.clone());
                    }
                    ids
                })
                .collect(),
            focus: l
                .organization
                .focus_goals
                .iter()
                .map(|g| g.id.clone())
                .collect(),
            methods: if l.organization.seen_observation_version == 0
                || l.organization.methods.is_empty()
            {
                None
            } else {
                Some(
                    l.organization
                        .methods
                        .iter()
                        .map(|m| m.id.clone())
                        .collect(),
                )
            },
            tags: vec![],
        });
    }
    let window: Vec<_> = all.iter().filter(|r| r.day >= start).collect();
    let mut goals = vec![];
    for g in &snapshot.goals {
        let hit = |r: &Record| r.direct.contains(&g.id) || r.below.contains(&g.id);
        let count = |p: &dyn Fn(&Record) -> bool| {
            let mut f = 0;
            let mut l = 0;
            for r in &window {
                if p(r) {
                    if r.family { f += 1 } else { l += 1 }
                }
            }
            Counts {
                family: f,
                lesson: l,
                selected: selected(f, l, &request.source),
            }
        };
        let last = all
            .iter()
            .filter(|r| scope(r, &request.source) && hit(r))
            .map(|r| r.day.clone())
            .max();
        let anchor = g.input.attention_start.clone().unwrap_or_else(|| {
            db.query(
                "SELECT data FROM prd_revisions WHERE entity_id=?1 ORDER BY version LIMIT 1",
                params![g.id],
            )
            .ok()
            .and_then(|r| {
                r.first()
                    .and_then(|r| serde_json::from_str::<super::model::Revision>(&r[0]).ok())
            })
            .map(|r| {
                chrono::DateTime::parse_from_rfc3339(&r.recorded_at)
                    .map(|v| {
                        v.with_timezone(&chrono::FixedOffset::east_opt(28800).unwrap())
                            .date_naive()
                            .to_string()
                    })
                    .unwrap_or_else(|_| today.into())
            })
            .unwrap_or_else(|| today.into())
        });
        let interval = match g.input.frequency {
            Some(Frequency::None) => None,
            Some(Frequency::Weekly) => Some(7),
            Some(Frequency::Fortnightly) => Some(14),
            Some(Frequency::Monthly) => Some(30),
            None => match g.input.state {
                Some(GoalState::Maintaining) => Some(30),
                Some(GoalState::Active) | None => Some(7),
                _ => None,
            },
        };
        let eligible = g.input.level != crate::model::Level::Long
            && !g.input.archived
            && matches!(
                g.input.state,
                Some(GoalState::Active | GoalState::Maintaining) | None
            );
        let ratio = if eligible {
            interval.and_then(|n| {
                NaiveDate::parse_from_str(last.as_deref().unwrap_or(&anchor), "%Y-%m-%d")
                    .ok()
                    .map(|d| (date - d).num_days().max(0) as f64 / f64::from(n))
            })
        } else {
            None
        };
        let c = count(&hit);
        let b = band(c.selected, request.days);
        goals.push(GoalCoverage {
            id: g.id.clone(),
            counts: c,
            direct: count(&|r| r.direct.contains(&g.id)),
            descendants: count(&|r| r.below.contains(&g.id)),
            focus: count(&|r| r.focus.contains(&g.id)),
            last_date: last,
            attention_anchor: anchor,
            ratio,
            band: b,
        });
    }
    let mut suggested: Vec<_> = goals.iter().filter(|g| g.ratio.is_some()).collect();
    suggested.sort_by(|a, b| {
        b.ratio
            .unwrap()
            .total_cmp(&a.ratio.unwrap())
            .then(
                a.last_date
                    .as_deref()
                    .unwrap_or(&a.attention_anchor)
                    .cmp(b.last_date.as_deref().unwrap_or(&b.attention_anchor)),
            )
            .then(a.id.cmp(&b.id))
    });
    let suggested_ids = suggested
        .into_iter()
        .take(3)
        .map(|g| g.id.clone())
        .collect();
    let window: Vec<Record> = all.into_iter().filter(|r| r.day >= start).collect();
    let mut method_counts = BTreeMap::new();
    for m in &snapshot.methods {
        method_counts.insert(
            m.id.clone(),
            counts(&window, &request.source, |r| {
                r.methods.as_ref().is_some_and(|ids| ids.contains(&m.id))
            }),
        );
    }
    let mut tags = BTreeMap::new();
    let mut heatmap = BTreeMap::new();
    let mut unclassified = 0;
    for r in window.iter().filter(|r| r.family) {
        *heatmap.entry(r.day.clone()).or_insert(0) += 1;
        if r.tags.is_empty() {
            unclassified += 1;
        }
        for tag in &r.tags {
            *tags.entry(tag.clone()).or_insert(0) += 1;
        }
    }
    let version = snapshot.goals.iter().map(|g| g.version).sum::<u64>()
        + snapshot.methods.iter().map(|m| m.version).sum::<u64>()
        + snapshot.catalog.iter().map(|c| c.version).sum::<u64>()
        + snapshot.lessons.iter().map(|l| l.version).sum::<u64>()
        + snapshot.sessions.iter().map(|s| s.version).sum::<u64>();
    Ok(Report {
        days: request.days,
        source: request.source.clone(),
        start,
        end: today.into(),
        data_version: version,
        records: counts(&window, &request.source, |_| true),
        goals,
        methods: method_counts,
        method_unknown: counts(&window, &request.source, |r| r.methods.is_none()),
        no_library_method: counts(&window, &request.source, |r| {
            r.methods.as_ref().is_some_and(Vec::is_empty)
        }),
        tags,
        unclassified,
        heatmap,
        suggested_ids,
    })
}
