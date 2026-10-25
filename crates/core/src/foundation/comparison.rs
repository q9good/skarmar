//! Derive the comparison phase from explicit practice revisions. Do not erase
//! earlier evidence or mutate a knowledge editor behind its owner's back.
use super::{
    lesson_model::Knowledge,
    model::{Revision, Saved},
    process_model::{Feedback, Review},
};
use crate::{
    sql::{Sql, params},
    store::{ApiError, Result},
};
use chrono::{DateTime, FixedOffset};
use std::collections::BTreeMap;
pub(super) struct Boundary {
    pub at: String,
    pub date: String,
    pub method_id: String,
    pub version: u64,
    pub cause: &'static str,
    methods: BTreeMap<String, u64>,
}
pub(super) fn boundary(db: &dyn Sql, k: &Knowledge) -> Result<Option<Boundary>> {
    let history: Vec<Knowledge> = db.query("SELECT data FROM prd_output_revisions WHERE kind='knowledge' AND id=?1 ORDER BY version DESC", params![k.input.id])?.into_iter().map(|r| serde_json::from_str(&r[0]).map_err(ApiError::from)).collect::<Result<_>>()?;
    let mut latest: Option<Boundary> = None;
    let mut methods = BTreeMap::new();
    for row in db.query("SELECT data FROM prd_revisions WHERE json_extract(data,'$.event')='practice_adjusted' ORDER BY json_extract(data,'$.recorded_at') DESC,rowid DESC", &[])? {
        let revision: Revision = serde_json::from_str(&row[0])?;
        if let Saved::Method(m) = revision.entity {

            let stamp = DateTime::parse_from_rfc3339(&revision.recorded_at).map_err(|_| ApiError::storage())?;
            // A later creation/binding must not retrospectively split evidence.
            let related = history.iter().find(|old| DateTime::parse_from_rfc3339(old.audit.as_ref().map_or(&old.created_at, |a| &a.at)).is_ok_and(|at| at <= stamp)).is_some_and(|old| old.input.method_ids.contains(&m.id));
            if !related && !m.input.basis.iter().any(|b| b.id == k.input.id) { continue; }
            methods.entry(m.id.clone()).or_insert(m.version);

            if latest.as_ref().is_none_or(|b| DateTime::parse_from_rfc3339(&b.at).is_ok_and(|v| stamp > v)) {
                latest = Some(Boundary { date: stamp.with_timezone(&FixedOffset::east_opt(8*3600).unwrap()).date_naive().to_string(), at: revision.recorded_at, method_id: m.id, version: m.version, cause: "method", methods: BTreeMap::new() });
            }
        }
    }
    if k.input.kind == super::lesson_model::KnowledgeKind::Difficulty {
        for pair in history.windows(2) {
            let (current, old) = (&pair[0], &pair[1]);
            let reopened = super::followups::effective_state(old) == "resolved"
                && ["pending", "following"]
                    .contains(&super::followups::effective_state(current).as_str());
            if current.details.basis != old.details.basis
                || current.details.support != old.details.support
                || current.details.comparison_start != old.details.comparison_start
                || reopened
            {
                let at = current
                    .audit
                    .as_ref()
                    .map_or(&current.created_at, |a| &a.at);
                consider(&mut latest, at, "difficulty", current.version)?;
            }
        }
        // The same business day can contain several adjustments. Only the
        // original review audit starts a phase; correcting its note cannot.
        for row in db.query("SELECT r.data FROM prd_review_revisions r JOIN prd_reviews p ON p.id=r.review_id WHERE r.version=1 AND p.object_kind='knowledge' AND p.object_id=?1", params![k.input.id])? {
            let review: Review = serde_json::from_str(&row[0])?;
            if ["adjust", "reopen"].contains(&review.decision.as_str()) {
                consider(&mut latest, &review.audit.at, "difficulty", k.version)?;
            }
        }
    }
    if let Some(b) = latest.as_mut() {
        b.methods = methods;
    }
    Ok(latest)
}

fn consider(
    latest: &mut Option<Boundary>,
    at: &str,
    cause: &'static str,
    version: u64,
) -> Result<()> {
    let stamp = DateTime::parse_from_rfc3339(at).map_err(|_| ApiError::storage())?;
    if latest
        .as_ref()
        .is_none_or(|b| DateTime::parse_from_rfc3339(&b.at).is_ok_and(|v| stamp > v))
    {
        *latest = Some(Boundary {
            at: at.into(),
            date: stamp
                .with_timezone(&FixedOffset::east_opt(8 * 3600).unwrap())
                .date_naive()
                .to_string(),
            method_id: String::new(),
            version,
            cause,
            methods: BTreeMap::new(),
        });
    }
    Ok(())
}

pub(super) fn in_phase(e: &Feedback, b: &Option<Boundary>) -> bool {
    b.as_ref().is_none_or(|b| {
        e.input.business_date >= b.date
            && DateTime::parse_from_rfc3339(&e.created_at)
                .ok()
                .zip(DateTime::parse_from_rfc3339(&b.at).ok())
                .is_some_and(|(e, b)| e >= b)
            && !e
                .input
                .method_refs
                .iter()
                .any(|m| b.methods.get(&m.id).is_some_and(|v| m.version < *v))
    })
}
