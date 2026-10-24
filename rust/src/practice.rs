//! GUI-independent practice state. Old 0.2 session JSON remains readable.
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use crate::{domain::Question, store::{id, Store}};

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Answer {
    #[serde(default)] pub text: String,
    #[serde(default)] pub selected: Vec<String>,
    #[serde(default)] pub verdict: Option<bool>,
    #[serde(default)] pub image: String,
}
impl Answer {
    pub fn answered(&self) -> bool {
        !self.text.trim().is_empty() || !self.selected.is_empty() || self.verdict.is_some() || !self.image.is_empty()
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String, pub questions: Vec<Question>, pub answers: BTreeMap<String, Answer>,
    pub index: usize, pub exam: bool, pub deadline: u64, pub submitted: bool,
    #[serde(default)] pub started_at: u64,
    #[serde(default)] pub elapsed_ms: u64,
    #[serde(default)] pub bank_id: String,
    #[serde(default)] pub directory_id: String,
    #[serde(default)] pub title: String,
    #[serde(default)] pub question_elapsed: BTreeMap<String, u64>,
    #[serde(default)] pub active_since: u64,
}
impl Session {
    pub fn new(questions: Vec<Question>, exam: bool, minutes: u32, time: u64) -> Result<Self> {
        ensure!(!questions.is_empty(), "当前范围没有题目，请先导入或调整筛选");
        ensure!((1..=360).contains(&minutes), "考试时长应为1–360分钟");
        Ok(Self { id: id(), questions, answers: BTreeMap::new(), index: 0, exam,
            deadline: if exam { time + minutes as u64 * 60_000 } else { 0 },
            submitted: false, started_at: time, active_since: time, elapsed_ms: 0,
            bank_id: String::new(), directory_id: String::new(), title: String::new(),
            question_elapsed: BTreeMap::new() })
    }
    pub fn current(&self) -> Result<&Question> {
        self.questions.get(self.index).ok_or_else(|| anyhow::anyhow!("练习位置无效，请从学习页恢复"))
    }
    pub fn answered(&self) -> usize {
        self.questions.iter().filter(|q| self.answers.get(&q.id).is_some_and(Answer::answered)).count()
    }
    pub fn checkpoint(&mut self, time: u64) {
        if !self.submitted && self.active_since > 0 {
            if let Some(q) = self.questions.get(self.index) {
                *self.question_elapsed.entry(q.id.clone()).or_default() += time.saturating_sub(self.active_since);
            }
        }
        self.active_since = time;
        self.elapsed_ms = time.saturating_sub(self.started_at);
    }
    pub fn navigate(&mut self, index: usize, time: u64) -> Result<()> {
        ensure!(index < self.questions.len(), "题号超出范围");
        self.checkpoint(time); self.index = index; Ok(())
    }
    pub fn attempt_payload(&self, question: &Question, time: u64) -> Result<Value> {
        let answer = self.answers.get(&question.id).cloned().unwrap_or_default();
        let value = if question.kind == "boolean" { json!(answer.verdict) } else { json!(answer.selected) };
        let active = if self.current()?.id == question.id && !self.submitted && self.active_since > 0 {
            time.saturating_sub(self.active_since)
        } else { 0 };
        Ok(json!({"id":format!("{}:{}",self.id,question.id), "sessionId":self.id,
            "bankId":self.bank_id, "directoryId":self.directory_id, "paperTitle":self.title,
            "questionId":question.id, "question":question, "answer":answer,
            "title":crate::domain::clip(&question.stem,100), "maxScore":question.max_score,
            "score":if question.subjective(){0.}else{question.score(&value)?},
            "pending":question.subjective()&&answer.answered(), "unanswered":!answer.answered(),
            "answeredAt":time, "elapsedMs":self.question_elapsed.get(&question.id).copied().unwrap_or(0)+active,
            "explanation":question.explanation, "feedback":if !answer.answered(){"未作答"}else if question.subjective(){"等待AI批改"}else{"按标准答案判定"}}))
    }
    /// One SQLite transaction commits all grades and the submitted session.
    /// Repeated submission is a no-op, including after an application restart.
    pub fn submit(&mut self, store: &Store, time: u64) -> Result<()> {
        if self.submitted { return Ok(()); }
        let mut next = self.clone(); next.checkpoint(time); next.submitted = true;
        let mut writes = Vec::new();
        for q in &next.questions { writes.push(("attempt".into(), next.attempt_payload(q,time)?)); }
        writes.push(("session".into(), serde_json::to_value(&next)?));
        store.put_many(writes)?; *self = next; Ok(())
    }
}

#[cfg(test)] mod tests {
    use super::*;
    fn session() -> Session { Session::new(serde_json::from_str::<crate::domain::Bank>(include_str!("../../examples/sample-bank.json")).unwrap().questions,true,30,1_000).unwrap() }
    #[test] fn navigation_accounts_each_question_once() {
        let mut s=session(); s.navigate(1,4_000).unwrap(); s.navigate(0,9_000).unwrap();
        assert_eq!(s.attempt_payload(&s.questions[0],10_000).unwrap()["elapsedMs"],4_000);
        assert_eq!(s.attempt_payload(&s.questions[1],10_000).unwrap()["elapsedMs"],5_000);
        assert!(s.navigate(99,11_000).is_err()); assert_eq!(s.index,0);
    }
    #[test] fn atomic_submission_is_restart_idempotent_and_includes_unanswered() {
        let root=std::env::temp_dir().join(id()); let store=Store::open(&root).unwrap(); let mut s=session();
        s.submit(&store,5_000).unwrap(); let count=store.revisions(None).unwrap().len();
        let mut restored:Session=serde_json::from_value(store.get(&s.id).unwrap().unwrap().payload).unwrap();
        restored.submit(&store,9_000).unwrap(); assert_eq!(store.revisions(None).unwrap().len(),count);
        assert_eq!(store.list("attempt").unwrap().len(),s.questions.len());
        assert!(store.list("attempt").unwrap().iter().all(|r|r.payload["unanswered"]==true&&r.payload["pending"]==false));
        assert_eq!(restored.elapsed_ms,4_000); std::fs::remove_dir_all(root).unwrap();
    }
    #[test] fn legacy_sessions_accept_new_timing_fields() {
        let mut value=serde_json::to_value(session()).unwrap();
        value.as_object_mut().unwrap().remove("question_elapsed"); value.as_object_mut().unwrap().remove("active_since");
        let s:Session=serde_json::from_value(value).unwrap(); assert_eq!(s.active_since,0);
        assert!(s.current().is_ok()); assert_eq!(s.answered(),0);
    }
}
