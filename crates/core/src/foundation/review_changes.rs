//! Only explicit recorded context; never infer a course start, diagnosis or cause.
use super::{lesson_model::Lesson, session_model::Session};
use serde_json::{Value, json};
pub(super) fn session(s: &Session, goals: &[String], methods: &[String]) -> Vec<Value> {
    let Some(summary) = &s.input.summary else {
        return vec![];
    };
    let mut changes = vec![];
    for m in &s.actual_methods {
        let relevant = methods.contains(&m.id)
            || goals.iter().any(|id| {
                m.input.goal_ids.contains(id)
                    || s.actual_targets.iter().any(|t| {
                        m.input.goal_ids.contains(&t.goal.id)
                            && t.ancestors.iter().any(|g| &g.id == id)
                    })
            });
        if relevant && summary.first_try_ids.contains(&m.id) {
            changes.push(json!({"kind":"method_first_try","text":format!("家人标记本次开始尝试：{}（第{}版）",m.input.title,m.version),"method_id":m.id,"method_version":m.version}));
        }
    }
    if !summary.context.trim().is_empty() {
        changes.push(json!({"kind":"context","text":summary.context}));
    }
    changes
}
pub(super) fn lesson(l: &Lesson) -> Vec<Value> {
    if l.observation.course_started == Some(true) {
        vec![
            json!({"kind":"course_started","text":"家人明确标记这门课程刚开始；作为同期背景，不代表它造成了变化"}),
        ]
    } else {
        vec![]
    }
}
