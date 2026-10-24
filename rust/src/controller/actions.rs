use super::*;

impl EasyGot {
    pub fn qa_state(&self) -> Value {
        json!({"sampledAt":now(),"route":self.view,"tab":self.tab,"questionCount":self.list("question").len(),"attemptCount":self.list("attempt").len(),"result":self.result,"session":self.session,"selectedBank":self.selected_bank,"selectedDirectory":self.selected_directory,"status":self.status})
    }
    pub fn title(&self) -> &str {
        match self.view.as_str() {
            "question" => "答题",
            "result" => "答题解析",
            "paper" => "纸上练习",
            "papers" => "我的试卷",
            "paper-review" => "识别核对",
            "bank-links" => "关联资料",
            "directory" => "选择套卷",
            "material-import" => "导入资料",
            "material" => "教材阅读",
            "editor" => "预览与编辑",
            "question-editor" => "编辑题目",
            "generate" => "生成新题",
            "history" => "版本记录",
            "conflicts" => "同步冲突",
            "summary" => "答题结果",
            _ => match self.tab {
                0 => "学习",
                1 => "题库",
                2 => "资料",
                _ => "设置",
            },
        }
    }
    pub fn go(&mut self, tab: usize) {
        if let Err(e) = self.save_session() {
            self.status = e.to_string();
            return;
        }
        self.navigation.clear();
        self.tab = tab;
        self.view.clear();
        self.filter.clear();
        self.visible_count = 40;
        self.dirty = true;
    }
    pub fn save_session(&mut self) -> Result<()> {
        if let Some(s) = self.session.as_mut() {
            if !s.submitted {
                s.checkpoint(now());
            }
            let revision = self.store.save(
                "session",
                &s.id,
                serde_json::to_value(&s)?,
                self.session_base.as_deref(),
                false,
                false,
            )?;
            self.session_base = Some(revision.version);
        }
        self.pending_save = false;
        Ok(())
    }
    pub(super) fn new_session(&mut self, questions: Vec<Question>, exam: bool) -> Result<()> {
        let questions = questions
            .iter()
            .map(|q| learning::effective_question(&self.store, q))
            .collect::<Result<Vec<_>>>()?;
        self.save_session()?;
        let mut session = Session::new(questions, exam, self.minutes, now())?;
        session.bank_id = self.selected_bank.clone();
        session.directory_id = self.selected_directory.clone();
        session.title = self.scope_title();
        self.session = Some(session);
        self.session_base = None;
        self.save_session()?;
        self.view = "question".into();
        Ok(())
    }
    pub(super) fn finish_exam(&mut self) -> Result<()> {
        let s = self
            .session
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("没有进行中的练习"))?;
        s.submit(&self.store, now())?;
        self.session_base = self.store.get(&s.id)?.map(|r| r.version);
        self.view = "summary".into();
        Ok(())
    }
    pub fn tick(&mut self) {
        self.poll();
        if self.pending_save && now().saturating_sub(self.last_edit) > 500 {
            let r = self.save_session();
            if let Err(e) = r {
                self.status = e.to_string();
                self.dirty = true;
            }
        }
        if self
            .session
            .as_ref()
            .is_some_and(|s| s.exam && !s.submitted && s.deadline <= now())
        {
            let r = self.finish_exam();
            self.info(r);
            self.dirty = true;
        }
    }
    pub fn clock(&self) -> String {
        if self.view == "question" {
            if let Some(s) = &self.session {
                return if s.exam {
                    format!(
                        "剩余 {} · 已答 {}/{}",
                        duration(s.deadline.saturating_sub(now())),
                        s.answered(),
                        s.questions.len()
                    )
                } else {
                    format!(
                        "已答 {}/{} · {}",
                        s.answered(),
                        s.questions.len(),
                        duration(now().saturating_sub(s.started_at))
                    )
                };
            }
        }
        if self.view == "paper" {
            if let Some(p) = &self.paper {
                return format!(
                    "{} · {}",
                    duration(p.elapsed(now())),
                    if p.started_at.is_some() {
                        "计时中"
                    } else {
                        "已暂停"
                    }
                );
            }
        }
        String::new()
    }
    pub fn stats(&self) -> [String; 6] {
        let attempts = self.list("attempt");
        let graded: Vec<_> = attempts
            .iter()
            .filter(|a| a.payload["pending"] != true)
            .collect();
        let correct = graded
            .iter()
            .filter(|a| a.payload["score"].as_f64() == a.payload["maxScore"].as_f64())
            .count();
        let millis = self
            .list("session")
            .iter()
            .map(|a| a.payload["elapsed_ms"].as_u64().unwrap_or(0))
            .sum::<u64>()
            + self
                .list("paper")
                .iter()
                .filter_map(|a| serde_json::from_value::<Paper>(a.payload.clone()).ok())
                .map(|p| p.elapsed(now()))
                .sum::<u64>();
        [
            attempts.len().to_string(),
            format!("{} 道题", self.list("question").len()),
            format!("{} 份资料", self.list("material").len()),
            format!("{} 分钟", millis / 60000),
            if graded.is_empty() {
                "—".into()
            } else {
                format!("{}%", correct * 100 / graded.len())
            },
            format!("{} 题待复习", self.store.wrong().map_or(0, |a| a.len())),
        ]
    }
    pub fn set_field(&mut self, key: &str, value: String) -> Result<()> {
        self.last_edit = now();
        match key {
            "filter" => {
                self.filter = value;
                self.visible_count = 40;
                self.dirty = true;
            }
            "strict" => self.strict = value,
            "editor" => self.editor = value,
            "query" => self.query = value,
            "minutes" => self.minutes = value.parse().unwrap_or(0),
            "aiBase" => self.config.ai_base = value,
            "aiKey" => self.config.ai_key = value,
            "model" => self.config.model = value,
            "searchBase" => self.config.search_base = value,
            "searchKey" => self.config.search_key = value,
            "davUrl" => self.config.dav_url = value,
            "davUser" => self.config.dav_user = value,
            "davPassword" => self.config.dav_password = value,
            "answer" => {
                let s = self
                    .session
                    .as_mut()
                    .ok_or_else(|| anyhow::anyhow!("练习已关闭"))?;
                ensure!(!s.submitted, "本轮已提交，请开启新的练习");
                let q = s.current()?.id.clone();
                s.answers.entry(q).or_default().text = value;
                self.pending_save = true;
            }
            _ if key.starts_with("spec:") => {
                let field = &key[5..];
                let mut spec: Value = serde_json::from_str(&self.spec)?;
                if let Some(kind) = field.strip_prefix("type-") {
                    spec["typeCounts"][kind] = json!(value.parse::<u32>().unwrap_or(0));
                } else if field == "count" || field == "difficulty" {
                    spec[field] = json!(value.parse::<u32>().unwrap_or(0));
                } else if field == "weights" {
                    spec[field] = serde_json::from_str(&value).unwrap_or(json!(null));
                } else {
                    spec[field] = json!(value);
                }
                self.spec = spec.to_string();
            }
            _ if key.starts_with("q:") => {
                let q = self
                    .edit_question
                    .as_mut()
                    .ok_or_else(|| anyhow::anyhow!("题目编辑已关闭"))?;
                match &key[2..] {
                    "stem" => q.stem = value,
                    "reference" => q.reference_answer = value,
                    "explanation" => q.explanation = value,
                    "score" => q.max_score = value.parse().unwrap_or(0.),
                    "chapter" => q.chapter = value,
                    "knowledge" => {
                        q.knowledge_points = value
                            .split(['，', ','])
                            .map(str::trim)
                            .filter(|s| !s.is_empty())
                            .map(str::to_owned)
                            .collect()
                    }
                    "strict" => q.strict_requirements = value,
                    option if option.starts_with("option-") => {
                        if let Some(item) = q.options.get_mut(option[7..].parse::<usize>()?) {
                            item.text = value;
                        }
                    }
                    _ => {}
                }
            }
            _ if key.starts_with("review:") => {
                let parts: Vec<_> = key.split(':').collect();
                let index = parts[1].parse::<usize>()?;
                let mut answers: Vec<RecognizedAnswer> = serde_json::from_str(&self.paper_review)?;
                let a = answers
                    .get_mut(index)
                    .ok_or_else(|| anyhow::anyhow!("识别题号无效"))?;
                match parts.get(2).copied().unwrap_or("") {
                    "text" => a.text = value,
                    "number" => a.number = value.parse().unwrap_or(0),
                    _ => {}
                }
                a.uncertain = true;
                self.paper_review_confirmed = false;
                self.paper_review = serde_json::to_string(&answers)?;
            }
            _ => {}
        }
        Ok(())
    }
    pub fn perform(&mut self, action: &str) {
        self.dirty = true;
        if self.busy && !matches!(action, "stop" | "assistant-close" | "confirm-no") {
            return;
        }
        let previous = (self.tab, self.view.clone());
        if let Err(error) = self.dispatch(action) {
            self.status = error.to_string();
        }
        if self.view != previous.1
            && !self.view.is_empty()
            && !matches!(action, "back" | "system-back" | "continue" | "confirm-yes")
            && !action.starts_with("nav:")
        {
            self.navigation.push(previous);
        }
    }
    pub(super) fn dispatch(&mut self, action: &str) -> Result<()> {
        let (verb, arg) = action.split_once(':').unwrap_or((action, ""));
        match verb {
            "nav" => {
                if matches!(self.view.as_str(), "editor" | "question-editor") {
                    self.confirm = format!("leave:{arg}");
                } else {
                    self.go(arg.parse()?);
                }
            }
            "system-back" => {
                if !self.confirm.is_empty() {
                    self.confirm.clear();
                } else if self.assistant_open {
                    self.assistant_open = false;
                } else if self.view.is_empty() {
                    if self.tab != 0 {
                        self.go(0);
                    } else {
                        self.save_session()?;
                        self.bridge.call("background", "")?;
                    }
                } else {
                    self.dispatch("back")?;
                }
            }
            "back" => {
                self.save_session()?;
                if matches!(self.view.as_str(), "editor" | "question-editor") {
                    self.confirm = "leave:back".into();
                } else {
                    self.back();
                }
            }
            "stop" => {
                self.cancel.store(true, Ordering::Relaxed);
                self.status = "将在当前请求返回后停止".into();
            }
            "sample" => {
                self.bank_import(serde_json::from_str(include_str!(
                    "../../../examples/sample-bank.json"
                ))?)?;
                self.status = "已载入4道示例题，可以开始练习".into();
            }
            "pick" => {
                self.bridge.call("pick", arg)?;
            }
            "more" => self.visible_count += 40,
            "bank" => {
                self.view.clear();
                self.selected_bank = arg.into();
                self.selected_directory.clear();
                self.filter.clear();
                self.visible_count = 40;
            }
            "choose-directory" => self.view = "directory".into(),
            "material-import" => self.view = "material-import".into(),
            "directory" => {
                self.view.clear();
                self.selected_directory = arg.into();
                self.visible_count = 40;
                if let Some(b) = learning::banks(&self.store)?
                    .iter()
                    .find(|b| b.id == self.selected_bank)
                {
                    if let Some(n) =
                        DirectoryEntry::find(&b.directory, arg).and_then(|e| e.time_limit_minutes)
                    {
                        self.minutes = n;
                    }
                }
            }
            "practice" => self.new_session(self.filtered_questions(), false)?,
            "exam" => self.new_session(self.filtered_questions(), true)?,
            "wrong" => {
                let questions = self
                    .store
                    .wrong()?
                    .into_iter()
                    .filter_map(|a| serde_json::from_value(a["question"].clone()).ok())
                    .collect();
                self.new_session(questions, false)?;
            }
            "resume" => {
                let r = self
                    .store
                    .get(arg)?
                    .ok_or_else(|| anyhow::anyhow!("练习不存在"))?;
                let mut s: Session = serde_json::from_value(r.payload)?;
                s.current()?;
                s.active_since = now();
                self.view = if s.submitted { "summary" } else { "question" }.into();
                self.session = Some(s);
                self.session_base = Some(r.version);
            }
            "next" | "previous" => {
                let s = self
                    .session
                    .as_mut()
                    .ok_or_else(|| anyhow::anyhow!("练习已关闭"))?;
                let index = if verb == "next" {
                    s.index + 1
                } else {
                    s.index.saturating_sub(1)
                };
                s.navigate(index, now())?;
                self.save_session()?;
            }
            "answer-option" => {
                let s = self
                    .session
                    .as_mut()
                    .ok_or_else(|| anyhow::anyhow!("练习已关闭"))?;
                ensure!(!s.submitted, "本轮已提交，请开启新的练习");
                let q = s.current()?.clone();
                ensure!(q.options.iter().any(|o| o.id == arg), "选项无效");
                let a = s.answers.entry(q.id).or_default();
                if q.kind == "single" {
                    a.selected = vec![arg.into()];
                } else if a.selected.iter().any(|s| s == arg) {
                    a.selected.retain(|s| s != arg);
                } else {
                    a.selected.push(arg.into());
                }
                self.save_session()?;
            }
            "answer-verdict" => {
                let s = self
                    .session
                    .as_mut()
                    .ok_or_else(|| anyhow::anyhow!("练习已关闭"))?;
                ensure!(!s.submitted, "本轮已提交，请开启新的练习");
                let id = s.current()?.id.clone();
                s.answers.entry(id).or_default().verdict = Some(arg == "true");
                self.save_session()?;
            }
            "answer-image" => {
                self.image_for = self
                    .session
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("练习已关闭"))?
                    .current()?
                    .id
                    .clone();
                self.bridge.call("pick", "3")?;
            }
            "remove-image" => {
                if let Some(s) = self.session.as_mut() {
                    ensure!(!s.submitted, "本轮已提交，请开启新的练习");
                    let id = s.current()?.id.clone();
                    s.answers.entry(id).or_default().image.clear();
                }
                self.save_session()?;
            }
            "submit-one" => {
                self.save_session()?;
                let s = self
                    .session
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("练习已关闭"))?;
                ensure!(!s.exam, "模拟考试请整卷提交");
                let q = s.current()?;
                ensure!(
                    s.answers.get(&q.id).is_some_and(Answer::answered),
                    "请先选择或填写答案"
                );
                let a = s.attempt_payload(q, now())?;
                self.store.put_many(vec![("attempt".into(), a.clone())])?;
                self.result = Some(a);
                self.view = "result".into();
            }
            "submit-exam" => self.confirm = "exam".into(),
            "finish-practice" => self.finish_exam()?,
            "result" => {
                self.result = Some(
                    self.store
                        .get(arg)?
                        .ok_or_else(|| anyhow::anyhow!("作答记录不存在"))?
                        .payload,
                );
                self.view = "result".into();
            }
            "continue" => {
                self.view = if self.session.as_ref().is_some_and(|s| s.submitted) {
                    "summary"
                } else {
                    "question"
                }
                .into()
            }
            "note" => {
                let q = self
                    .session
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("练习已关闭"))?
                    .current()?;
                self.edit_id = format!("note:{}", q.id);
                let r = self.store.get(&self.edit_id)?;
                self.editor = r
                    .as_ref()
                    .and_then(|r| r.payload["text"].as_str())
                    .unwrap_or("")
                    .into();
                self.edit_base = r.map(|r| r.version);
                self.draft_kind = "note".into();
                self.view = "editor".into();
            }
            "grade" => {
                let store = self.store.clone();
                let config = self.config.clone();
                let mut attempt = self
                    .result
                    .clone()
                    .ok_or_else(|| anyhow::anyhow!("没有作答记录"))?;
                self.job("正在对照题目与教材批改…", move || {
                    let q: Question = serde_json::from_value(attempt["question"].clone())?;
                    let grade = ai::grade(&store, &config, &q, &attempt["answer"])?;
                    for (k, v) in grade
                        .as_object()
                        .ok_or_else(|| anyhow::anyhow!("评分结构无效"))?
                    {
                        attempt[k] = v.clone();
                    }
                    attempt["pending"] = json!(false);
                    store.put_many(vec![("attempt".into(), attempt.clone())])?;
                    Ok(Output::Grade(attempt))
                });
            }
            "links" => {
                let bank = learning::banks(&self.store)?
                    .into_iter()
                    .find(|b| b.id == self.selected_bank)
                    .ok_or_else(|| anyhow::anyhow!("请先选择题库"))?;
                self.link_bank = bank.id;
                self.link_materials = bank.material_ids.into_iter().collect();
                self.view = "bank-links".into();
            }
            "link" => {
                if !self.link_materials.remove(arg) {
                    self.link_materials.insert(arg.into());
                }
            }
            "links-save" => {
                learning::link_bank(
                    &self.store,
                    &self.link_bank,
                    self.link_materials.iter().cloned().collect(),
                )?;
                self.status = "题库与资料关联已保存".into();
                self.view.clear();
            }
            "role" => self.role = arg.parse::<usize>()?.min(2),
            "material-select" => {
                if !self.selected.remove(arg) {
                    self.selected.insert(arg.into());
                }
            }
            "material" => {
                self.material = self.store.get(arg)?;
                self.filter.clear();
                self.view = "material".into();
            }
            "strict-edit" => {
                let r = self
                    .material
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("未选择资料"))?;
                self.editor = r.payload["strictRules"].as_str().unwrap_or("").into();
                self.draft_kind = "strict".into();
                self.view = "editor".into();
            }
            "ocr" => self.confirm = "ocr".into(),
            "assistant" => {
                self.assistant_context = domain::clip(&self.contextual_data().to_string(), 12000);
                self.assistant_open = true;
            }
            "assistant-close" => self.assistant_open = false,
            "web" => self.allow_web = !self.allow_web,
            "ask" => {
                ensure!(!self.query.trim().is_empty(), "请先输入问题");
                let context = serde_json::from_str(&self.assistant_context).unwrap_or(Value::Null);
                self.select_context_materials(&context);
                let store = self.store.clone();
                let config = self.config.clone();
                let selected = self.selected.clone();
                let query = format!("{}\n当前学习上下文：{}", self.query, self.assistant_context);
                let session = self.agent_session.clone();
                let web = self.allow_web;
                let cancel = self.cancel.clone();
                self.job("助教正在查资料与分析…", move || {
                    Ok(Output::Chat(agent::run(
                        &store, &config, &session, &query, &selected, web, &cancel,
                    )?))
                });
            }
            "generate-page" => {
                if let Some(bank) = learning::banks(&self.store)?
                    .iter()
                    .find(|b| b.id == self.selected_bank)
                {
                    self.selected = bank.material_ids.iter().cloned().collect();
                }
                self.assistant_open = false;
                self.view = "generate".into();
            }
            "wrong-reference" => self.wrong = !self.wrong,
            "generate" => {
                let mut spec: Value = serde_json::from_str(&self.spec)?;
                domain::blueprint(&mut spec)?;
                let store = self.store.clone();
                let config = self.config.clone();
                let selected = self.selected.clone();
                let wrong = self.wrong;
                self.job("正在参考教材、考纲与错题出题…", move || {
                    Ok(Output::Bank(ai::generate(
                        &store, &config, spec, &selected, wrong,
                    )?))
                });
            }
            "draft" => {
                let r = self
                    .store
                    .get(arg)?
                    .ok_or_else(|| anyhow::anyhow!("草稿不存在"))?;
                if r.payload["type"] == "bankDraft" {
                    self.editor = serde_json::to_string_pretty(&r.payload["bank"])?;
                    self.draft_kind = "bank".into();
                } else {
                    self.editor = r.payload["text"].as_str().unwrap_or("").into();
                    self.edit_id =
                        format!("note:{}", r.payload["questionId"].as_str().unwrap_or(""));
                    self.edit_base = self.store.get(&self.edit_id)?.map(|r| r.version);
                    self.draft_kind = "note".into();
                }
                self.assistant_open = false;
                self.view = "editor".into();
            }
            "save-editor" => {
                if self.draft_kind == "bank" {
                    serde_json::from_str::<Bank>(&self.editor)?.validate()?;
                }
                self.confirm = "save".into();
            }
            "edit-question" => {
                let r = self
                    .store
                    .get(arg)?
                    .ok_or_else(|| anyhow::anyhow!("题目不存在"))?;
                self.edit_question = Some(serde_json::from_value(r.payload)?);
                self.edit_base = Some(r.version);
                self.edit_parent = false;
                self.view = "question-editor".into();
            }
            "new-question" => {
                self.edit_question = Some(serde_json::from_value(
                    json!({"id":id(),"type":"short","stem":"","maxScore":5,"origin":"original"}),
                )?);
                self.edit_base = None;
                self.edit_parent = false;
                self.view = "question-editor".into();
            }
            "draft-question" => {
                let bank: Bank = serde_json::from_str(&self.editor)?;
                self.edit_question = bank.all_questions().into_iter().find(|q| q.id == arg);
                self.edit_parent = true;
                self.view = "question-editor".into();
            }
            "q-type" => {
                ensure!(domain::TYPES.contains(&arg), "题型无效");
                if let Some(q) = self.edit_question.as_mut() {
                    q.kind = arg.into();
                    q.answer = if arg == "boolean" {
                        json!(true)
                    } else {
                        json!([])
                    };
                    if matches!(arg, "single" | "multiple") && q.options.len() < 2 {
                        q.options = vec![
                            domain::OptionItem {
                                id: "A".into(),
                                text: String::new(),
                            },
                            domain::OptionItem {
                                id: "B".into(),
                                text: String::new(),
                            },
                        ];
                    }
                }
            }
            "q-correct" => {
                if let Some(q) = self.edit_question.as_mut() {
                    if q.kind == "boolean" {
                        q.answer = json!(arg == "true");
                    } else {
                        let mut ids: Vec<String> =
                            serde_json::from_value(q.answer.clone()).unwrap_or_default();
                        if q.kind == "single" {
                            ids = vec![arg.into()];
                        } else if ids.iter().any(|s| s == arg) {
                            ids.retain(|s| s != arg);
                        } else {
                            ids.push(arg.into());
                        }
                        q.answer = json!(ids);
                    }
                }
            }
            "q-add-option" => {
                if let Some(q) = self.edit_question.as_mut() {
                    ensure!(q.options.len() < 12, "最多12个选项");
                    q.options.push(domain::OptionItem {
                        id: ((b'A' + q.options.len() as u8) as char).to_string(),
                        text: String::new(),
                    });
                }
            }
            "q-save" => {
                let q = self
                    .edit_question
                    .clone()
                    .ok_or_else(|| anyhow::anyhow!("编辑已关闭"))?;
                q.validate()?;
                if self.edit_parent {
                    let bank: Bank = serde_json::from_str(&self.editor)?;
                    let mut bank = bank.normalized();
                    let existing = bank
                        .questions
                        .iter_mut()
                        .find(|item| item.id == q.id)
                        .ok_or_else(|| anyhow::anyhow!("原题不存在"))?;
                    *existing = q;
                    bank.validate()?;
                    self.editor = serde_json::to_string_pretty(&bank)?;
                    self.view = "editor".into();
                } else {
                    self.store.save(
                        "question",
                        &q.id,
                        serde_json::to_value(&q)?,
                        self.edit_base.as_deref(),
                        false,
                        false,
                    )?;
                    if !self.selected_bank.is_empty() {
                        let r = self
                            .store
                            .get(&self.selected_bank)?
                            .ok_or_else(|| anyhow::anyhow!("题库不存在"))?;
                        let mut b: learning::BankGroup = serde_json::from_value(r.payload)?;
                        if !b.question_ids.contains(&q.id) {
                            b.question_ids.push(q.id);
                            self.store.save(
                                "bank",
                                &b.id,
                                serde_json::to_value(&b)?,
                                Some(&r.version),
                                false,
                                false,
                            )?;
                        }
                    }
                    self.view.clear();
                }
                self.edit_question = None;
                self.status = "题目已保存".into();
            }
            "save-config" => {
                for url in [
                    &self.config.ai_base,
                    &self.config.search_base,
                    &self.config.dav_url,
                ] {
                    if !url.is_empty() {
                        crate::network::https(url)?;
                    }
                }
                self.bridge
                    .call("saveConfig", &serde_json::to_string(&self.config)?)?;
                self.status = "连接配置已加密保存在本机".into();
            }
            "paste" => {
                let text = self.bridge.call("clipboard", "")?;
                ensure!(
                    !text.trim().is_empty(),
                    "剪贴板为空，请尝试系统输入长按粘贴"
                );
                self.set_field(arg, text.trim().into())?;
                self.status = "已粘贴，请保存连接配置".into();
            }
            "native" => {
                let value = match arg {
                    "aiKey" => &self.config.ai_key,
                    "searchKey" => &self.config.search_key,
                    _ => &self.config.dav_password,
                };
                self.bridge.call(
                    "editSecret",
                    &json!({"field":arg,"label":"输入或长按粘贴","value":value}).to_string(),
                )?;
            }
            "sync" => {
                self.save_session()?;
                let store = self.store.clone();
                let config = self.config.clone();
                self.job("正在同步，冲突版本会同时保留…", move || {
                    Ok(Output::Synced(sync::sync(&store, &config)?))
                });
            }
            "test-search" => {
                let config = self.config.clone();
                self.job("正在测试DeepSeek搜索…", move || {
                    let result = crate::network::search(&config, "学习方法")?;
                    Ok(Output::Info(format!(
                        "搜索成功：{}条结果",
                        result.as_array().map_or(0, |r| r.len())
                    )))
                });
            }
            "backup" => {
                self.save_session()?;
                let store = self.store.clone();
                self.job("正在准备完整备份…", move || {
                    let mut value = json!({"backupVersion":1,"revisions":store.revisions(None)?});
                    add_assets(&store, &mut value)?;
                    Ok(Output::Export(value, "EasyGot-backup.json".into()))
                });
            }
            "export-bank" => {
                let store = self.store.clone();
                let chosen = self.selected_bank.clone();
                let directory = self.selected_directory.clone();
                let filtered = !self.filter.is_empty();
                let questions = self.filtered_questions();
                self.job("正在导出题库…", move || {
                    let mut value = if let Some(bank) = learning::banks(&store)?
                        .into_iter()
                        .find(|b| b.id == chosen)
                    {
                        serde_json::to_value(learning::export_bank(
                            &store, &bank, &directory, questions, filtered,
                        ))?
                    } else {
                        json!({"schemaVersion":1,"title":"EasyGot题库","questions":questions})
                    };
                    add_assets(&store, &mut value)?;
                    Ok(Output::Export(value, "EasyGot-bank.json".into()))
                });
            }
            "history" | "conflicts" => self.view = verb.into(),
            "restore" | "resolve" => {
                let r = self
                    .store
                    .revisions(None)?
                    .into_iter()
                    .find(|r| r.version == arg)
                    .ok_or_else(|| anyhow::anyhow!("版本不存在"))?;
                self.editor = r.payload.to_string();
                self.edit_id = r.entity_id;
                self.draft_kind = r.kind;
                self.confirm = format!("{}{}", verb, if r.deleted { "_deleted" } else { "" });
            }
            "confirm-no" => self.confirm.clear(),
            "confirm-yes" => self.accept()?,
            "papers" => self.view = "papers".into(),
            "paper-new" => self.create_paper(self.filtered_questions())?,
            "paper" => {
                self.paper = self
                    .store
                    .get(arg)?
                    .map(|r| serde_json::from_value(r.payload))
                    .transpose()?;
                self.pdf_path.clear();
                self.view = "paper".into();
            }
            "paper-answers" => {
                self.show_answers = !self.show_answers;
                self.pdf_path.clear();
            }
            "pdf" => {
                let spec = self
                    .paper
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("试卷已关闭"))?
                    .print_spec(self.show_answers);
                let bridge = self.bridge.clone();
                self.job("正在排版试卷PDF…", move || {
                    Ok(Output::Pdf(bridge.call("printPaper", &spec.to_string())?))
                });
            }
            "pdf-save" => {
                self.bridge.call("export",&json!({"path":self.pdf_path,"name":if self.show_answers{"EasyGot-答案解析.pdf"}else{"EasyGot-作答试卷.pdf"}}).to_string())?;
            }
            "pdf-print" => {
                self.bridge.call("openPdf", &self.pdf_path)?;
            }
            "paper-timer" => {
                let p = self
                    .paper
                    .as_mut()
                    .ok_or_else(|| anyhow::anyhow!("试卷已关闭"))?;
                if p.started_at.is_some() {
                    p.stop(now());
                } else {
                    p.start(now());
                }
                learning::save_paper(&self.store, p)?;
            }
            "paper-upload" => {
                let p = self
                    .paper
                    .as_mut()
                    .ok_or_else(|| anyhow::anyhow!("试卷已关闭"))?;
                p.stop(now());
                learning::save_paper(&self.store, p)?;
                self.bridge.call("pick", "6")?;
            }
            "paper-grade" => {
                let id = self
                    .paper
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("试卷已关闭"))?
                    .id
                    .clone();
                self.grade_paper_pending(&id);
            }
            "review-confirm" => self.paper_review_confirmed = !self.paper_review_confirmed,
            "review-save" => {
                ensure!(self.paper_review_confirmed, "请逐题核对并勾选确认");
                let answers: Vec<RecognizedAnswer> = serde_json::from_str(&self.paper_review)?;
                let p = self
                    .paper
                    .as_mut()
                    .ok_or_else(|| anyhow::anyhow!("试卷已关闭"))?;
                p.stop(now());
                let n = learning::record_paper_answers(&self.store, p, &answers)?;
                self.status = format!("已记录{n}题，客观题已判分");
                self.view = "paper".into();
            }
            "review-check" | "review-option" | "review-verdict" => {
                let (index, value) = arg.split_once(':').unwrap_or((arg, ""));
                let mut answers: Vec<RecognizedAnswer> = serde_json::from_str(&self.paper_review)?;
                let a = answers
                    .get_mut(index.parse::<usize>()?)
                    .ok_or_else(|| anyhow::anyhow!("识别记录不存在"))?;
                match verb {
                    "review-check" => a.uncertain = !a.uncertain,
                    "review-verdict" => {
                        a.verdict = Some(value == "true");
                        a.uncertain = true;
                    }
                    _ => {
                        let q = self
                            .paper
                            .as_ref()
                            .and_then(|p| p.questions.iter().find(|q| q.number == a.number))
                            .ok_or_else(|| anyhow::anyhow!("请先修正题号"))?;
                        if q.question.kind == "single" {
                            a.selected = vec![value.into()];
                        } else if a.selected.iter().any(|s| s == value) {
                            a.selected.retain(|s| s != value);
                        } else {
                            a.selected.push(value.into());
                        }
                        a.uncertain = true;
                    }
                }
                self.paper_review_confirmed = false;
                self.paper_review = serde_json::to_string(&answers)?;
            }
            _ => anyhow::bail!("未知操作：{verb}"),
        }
        Ok(())
    }
    pub fn confirmation(&self) -> &str {
        if self.confirm.starts_with("leave:") {
            return "离开编辑？尚未保存的修改将丢弃。";
        }
        match self.confirm.as_str(){"exam"=>"确认提交整卷？未作答题记为0分，主观题提交后可联网批改。", "ocr"=>"将文档图片或PDF页面发送给你配置的视觉模型，可能产生多次调用费用。已识别页面会保存。", "save"=>"确认保存？同id内容会更新，历史版本保留，可从设置中恢复。",_=>"确认恢复或合并该版本？历史记录会保留。"}
    }
    pub(super) fn back(&mut self) {
        if let Some((tab, view)) = self.navigation.pop() {
            self.tab = tab;
            self.view = view;
        } else if self.view == "question-editor" && self.edit_parent {
            self.view = "editor".into();
        } else {
            self.view.clear();
        }
        if self.view == "question" && self.session.as_ref().is_some_and(|s| s.submitted) {
            self.view = "summary".into();
        }
        self.filter.clear();
    }
    pub fn resumed(&mut self) {
        if let Some(s) = self.session.as_mut() {
            if !s.submitted {
                s.active_since = now();
            }
        }
    }
    pub(super) fn accept(&mut self) -> Result<()> {
        let action = std::mem::take(&mut self.confirm);
        if let Some(target) = action.strip_prefix("leave:") {
            if target == "back" {
                self.back();
            } else {
                self.go(target.parse()?);
            }
            return Ok(());
        }
        match action.as_str() {
            "save" => self.save_editor()?,
            "exam" => self.finish_exam()?,
            "ocr" => self.start_ocr(),
            _ => {
                let payload = serde_json::from_str(&self.editor)?;
                self.store.save(
                    &self.draft_kind,
                    &self.edit_id,
                    payload,
                    None,
                    action.ends_with("deleted"),
                    action.starts_with("resolve"),
                )?;
                self.session = None;
            }
        }
        Ok(())
    }
}
