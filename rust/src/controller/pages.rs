use super::*;

impl EasyGot {
    pub fn rows(&self) -> Vec<Entry> {
        let mut rows = vec![];
        match self.view.as_str() {
            "exam-setup" => {
                let questions = self.filtered_questions();
                rows.push(Entry::text(
                    "exam-scope",
                    self.scope_title(),
                    format!(
                        "{}道题 · 总分{}分",
                        questions.len(),
                        questions.iter().map(|q| q.max_score).sum::<f64>()
                    ),
                ));
                rows.push(Entry::field(
                    "minutes",
                    "考试时长（分钟，1–360）",
                    self.minutes.to_string(),
                    false,
                ));
                rows.push(Entry::text("exam-rules","整卷提交后评分","计时开始后切换应用仍继续倒计时；到时自动交卷。主观题作答先保存，联网后可AI批改。"));
                rows.push(Entry::button("exam-start", "开始计时考试", true));
            }
            "question" => {
                if let Some(s) = &self.session {
                    if let Ok(q) = s.current() {
                        rows.push(Entry::text(
                            "question",
                            format!(
                                "{} / {} · {} · {}分",
                                s.index + 1,
                                s.questions.len(),
                                domain::label(&q.kind),
                                q.max_score
                            ),
                            &q.stem,
                        ));
                        self.image_row(&mut rows, "question-image", &q.image);
                        let a = s.answers.get(&q.id).cloned().unwrap_or_default();
                        if q.kind == "single" || q.kind == "multiple" {
                            for o in &q.options {
                                rows.push(Entry::choice(
                                    &format!("answer-option:{}", o.id),
                                    format!("{}. {}", o.id, o.text),
                                    "",
                                    a.selected.contains(&o.id),
                                ));
                            }
                        } else if q.kind == "boolean" {
                            for (value, label) in [(true, "正确"), (false, "错误")] {
                                rows.push(Entry::choice(
                                    &format!("answer-verdict:{value}"),
                                    label,
                                    "",
                                    a.verdict == Some(value),
                                ));
                            }
                        } else {
                            rows.push(Entry::field("answer", "你的作答", &a.text, true));
                            rows.push(Entry::button("answer-image", "上传手写答案图片", false));
                            self.image_row(&mut rows, "answer-photo", &a.image);
                            if !a.image.is_empty() {
                                rows.push(Entry::button("remove-image", "移除作答图片", false));
                            }
                        }
                        if s.index > 0 {
                            rows.push(Entry::button("previous", "上一题", false));
                        }
                        if s.exam {
                            rows.push(Entry::button("submit-exam", "提交整卷", true));
                        } else {
                            rows.push(Entry::button("submit-one", "提交本题", true));
                        }
                        if s.index + 1 < s.questions.len() {
                            rows.push(Entry::button("next", "下一题", false));
                        }
                        rows.push(Entry::pair("note", "本题笔记", "assistant", "问助教"));
                        if !s.exam {
                            rows.push(Entry::button(
                                "finish-practice",
                                "结束并查看本轮结果",
                                false,
                            ));
                        }
                    }
                }
            }
            "result" => {
                if let Some(a) = &self.result {
                    rows.push(Entry::text(
                        "grade",
                        if a["pending"] == true {
                            "已保存 · 等待AI批改".into()
                        } else {
                            format!("{} / {} 分", a["score"], a["maxScore"])
                        },
                        a["title"].as_str().unwrap_or(""),
                    ));
                    let answer = &a["answer"];
                    let text = if let Some(v) = answer["verdict"].as_bool() {
                        if v {
                            "正确".into()
                        } else {
                            "错误".into()
                        }
                    } else if answer["selected"].as_array().is_some_and(|v| !v.is_empty()) {
                        answer["selected"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .filter_map(Value::as_str)
                            .collect::<Vec<_>>()
                            .join("、")
                    } else {
                        answer["text"].as_str().unwrap_or("未作答").into()
                    };
                    rows.push(Entry::text("your-answer", "你的作答", text));
                    self.image_row(
                        &mut rows,
                        "submitted-image",
                        answer["image"].as_str().unwrap_or(""),
                    );
                    rows.push(Entry::text(
                        "feedback",
                        "判分与解析",
                        format!(
                            "{}\n{}",
                            a["feedback"].as_str().unwrap_or(""),
                            a["explanation"].as_str().unwrap_or("")
                        ),
                    ));
                    rows.push(Entry::text(
                        "reference",
                        "参考答案",
                        if a["question"]["referenceAnswer"]
                            .as_str()
                            .is_some_and(|v| !v.is_empty())
                        {
                            a["question"]["referenceAnswer"].as_str().unwrap().into()
                        } else {
                            a["question"]["answer"].to_string()
                        },
                    ));
                    if let Some(citations) = a["citations"].as_array() {
                        for (i, c) in citations.iter().enumerate() {
                            rows.push(Entry::text(
                                &format!("citation-{i}"),
                                c["title"].as_str().unwrap_or("教材引用"),
                                format!(
                                    "{}\n{}",
                                    c["locator"].as_str().unwrap_or(""),
                                    c["quote"].as_str().unwrap_or("")
                                ),
                            ));
                        }
                    }
                    rows.push(Entry::button("grade", "AI批改与详细讲解", true));
                    rows.push(Entry::text(
                        "grade-hint",
                        "使用已配置的视觉模型",
                        "会发送本题、作答和相关教材片段。",
                    ));
                    rows.push(Entry::button("continue", "返回练习", false));
                }
            }
            "summary" => {
                if let Some(s) = &self.session {
                    let attempts: Vec<_> = self
                        .list("attempt")
                        .into_iter()
                        .filter(|r| r.payload["sessionId"].as_str() == Some(&s.id))
                        .collect();
                    let score: f64 = attempts
                        .iter()
                        .filter(|a| a.payload["pending"] != true)
                        .map(|a| a.payload["score"].as_f64().unwrap_or(0.))
                        .sum();
                    let max: f64 = s.questions.iter().map(|q| q.max_score).sum();
                    let pending = attempts
                        .iter()
                        .filter(|a| a.payload["pending"] == true)
                        .count();
                    rows.push(Entry::text(
                        "summary",
                        format!("{score} / {max} 分"),
                        format!(
                            "已答 {}/{}题 · {}题待评分 · 用时 {}",
                            s.answered(),
                            s.questions.len(),
                            pending,
                            duration(s.elapsed_ms)
                        ),
                    ));
                    for a in attempts {
                        rows.push(Entry::choice(
                            &format!("result:{}", a.entity_id),
                            a.payload["title"].as_str().unwrap_or(""),
                            if a.payload["pending"] == true {
                                "待AI评分".into()
                            } else {
                                format!("{} / {} 分", a.payload["score"], a.payload["maxScore"])
                            },
                            false,
                        ));
                    }
                }
            }
            "bank-links" => {
                rows.push(Entry::text(
                    "link-help",
                    "为题库选择资料",
                    "讲解、评分与出题优先依据关联的教材、考纲和往年试题。",
                ));
                for m in self.list("material") {
                    rows.push(Entry::check(
                        &format!("link:{}", m.entity_id),
                        m.payload["title"].as_str().unwrap_or("资料"),
                        self.link_materials.contains(&m.entity_id),
                    ));
                }
                if self.list("material").is_empty() {
                    rows.push(Entry::button("nav:2", "先导入教材或考纲", true));
                } else {
                    rows.push(Entry::button("links-save", "保存题库关联", true));
                }
            }
            "directory" => {
                if let Some(bank) = learning::banks(&self.store)
                    .unwrap_or_default()
                    .iter()
                    .find(|b| b.id == self.selected_bank)
                {
                    rows.push(Entry::choice(
                        "directory:",
                        "全部题目",
                        "",
                        self.selected_directory.is_empty(),
                    ));
                    self.directory_rows(&mut rows, &bank.directory, 0);
                }
            }
            "material-import" => self.material_import_rows(&mut rows),
            "material" => {
                if let Some(m) = &self.material {
                    rows.push(Entry::text(
                        "material-title",
                        m.payload["title"].as_str().unwrap_or("资料"),
                        m.payload["strictRules"]
                            .as_str()
                            .filter(|s| !s.is_empty())
                            .unwrap_or("未设置额外严格要求"),
                    ));
                    rows.push(Entry::field(
                        "filter",
                        "在本文件内搜索",
                        &self.filter,
                        false,
                    ));
                    rows.push(Entry::pair(
                        "ocr",
                        "识别图片与页面",
                        "strict-edit",
                        "修改严格要求",
                    ));
                    if let Some(sections) = m.payload["sections"].as_array() {
                        let sections: Vec<_> = sections
                            .iter()
                            .filter(|s| {
                                self.filter.is_empty()
                                    || s["text"].as_str().unwrap_or("").contains(&self.filter)
                            })
                            .collect();
                        for (i, s) in sections.iter().take(self.visible_count).enumerate() {
                            rows.push(Entry::text(
                                &format!("section-{i}"),
                                s["locator"].as_str().unwrap_or("段落"),
                                s["text"]
                                    .as_str()
                                    .filter(|s| !s.is_empty())
                                    .unwrap_or("此处没有可提取文字，可使用图片识别。"),
                            ));
                        }
                        if sections.len() > self.visible_count {
                            rows.push(Entry::button("more", "继续阅读", false));
                        }
                    }
                }
            }
            "generate" => {
                rows.push(Entry::text(
                    "generate-help",
                    "先生成草稿，再确认入库",
                    format!(
                        "已选择{}份资料；可到资料页勾选教材、考纲、往年试题。",
                        self.selected.len()
                    ),
                ));
                let spec: Value = serde_json::from_str(&self.spec).unwrap_or(Value::Null);
                for (key, label) in [
                    ("title", "新题库名称"),
                    ("chapter", "指定章节"),
                    ("knowledgePoints", "知识点"),
                    ("count", "总题量（1–15）"),
                    ("difficulty", "难度（1–5）"),
                ] {
                    rows.push(Entry::field(
                        &format!("spec:{key}"),
                        label,
                        if let Some(s) = spec[key].as_str() {
                            s.into()
                        } else {
                            spec[key].to_string()
                        },
                        false,
                    ));
                }
                for kind in domain::TYPES {
                    rows.push(Entry::field(
                        &format!("spec:type-{kind}"),
                        &format!("{}题量", domain::label(kind)),
                        spec["typeCounts"][kind].as_u64().unwrap_or(0).to_string(),
                        false,
                    ));
                }
                rows.push(Entry::field(
                    "spec:weights",
                    "考纲知识点权重，例如 {\"导数\":2,\"积分\":1}",
                    spec["weights"].to_string(),
                    true,
                ));
                rows.push(Entry::check(
                    "wrong-reference",
                    "参考我的错题生成变式",
                    self.wrong,
                ));
                rows.push(Entry::button("generate", "生成并预览", true));
            }
            "editor" => {
                if self.draft_kind == "bank" {
                    match serde_json::from_str::<Bank>(&self.editor) {
                        Ok(bank) => {
                            rows.push(Entry::text(
                                "import-preview",
                                &bank.title,
                                format!(
                                    "{}道题 · {}个顶层目录 · 修改和确认后才入库",
                                    bank.all_questions().len(),
                                    bank.directory.len()
                                ),
                            ));
                            rows.push(Entry::button("save-editor", "校验并确认入库", true));
                            let questions = bank.all_questions();
                            for q in questions.iter().take(self.visible_count) {
                                rows.push(Entry::choice(
                                    &format!("draft-question:{}", q.id),
                                    &q.stem,
                                    format!(
                                        "{} · {}分 · 点击编辑",
                                        domain::label(&q.kind),
                                        q.max_score
                                    ),
                                    false,
                                ));
                            }
                            if questions.len() > self.visible_count {
                                rows.push(Entry::button("more", "显示更多题目", false));
                            }
                        }
                        Err(e) => {
                            rows.push(Entry::text("invalid-draft", "草稿需要修正", e.to_string()))
                        }
                    }
                } else {
                    rows.push(Entry::field(
                        "editor",
                        if self.draft_kind == "note" {
                            "笔记内容"
                        } else {
                            "严格要求"
                        },
                        &self.editor,
                        true,
                    ));
                    rows.push(Entry::button("save-editor", "保存修改", true));
                }
            }
            "question-editor" => self.question_editor_rows(&mut rows),
            "papers" => {
                rows.push(Entry::button("nav:1", "从题库选题组卷", true));
                for r in self.list("paper") {
                    if let Ok(p) = serde_json::from_value::<Paper>(r.payload) {
                        rows.push(Entry::choice(
                            &format!("paper:{}", p.id),
                            &p.title,
                            format!(
                                "{}题 · {} · 已上传{}页",
                                p.questions.len(),
                                duration(p.elapsed(now())),
                                p.pages.len()
                            ),
                            false,
                        ));
                    }
                }
                if rows.len() == 1 {
                    rows.push(Entry::text(
                        "no-papers",
                        "还没有纸上试卷",
                        "在题库选择套卷或筛选题目后，导出为PDF试卷。",
                    ));
                }
            }
            "paper" => {
                if let Some(p) = &self.paper {
                    rows.push(Entry::text(
                        "paper-summary",
                        &p.title,
                        format!("{}道题 · 已上传{}页答卷", p.questions.len(), p.pages.len()),
                    ));
                    rows.push(Entry::check(
                        "paper-answers",
                        "导出答案与解析卷",
                        self.show_answers,
                    ));
                    rows.push(Entry::button(
                        "pdf",
                        if self.show_answers {
                            "生成答案解析PDF"
                        } else {
                            "生成作答试卷PDF"
                        },
                        true,
                    ));
                    if !self.pdf_path.is_empty() {
                        rows.push(Entry::pair(
                            "pdf-save",
                            "保存PDF",
                            "pdf-print",
                            "打印 / 预览",
                        ));
                    }
                    rows.push(Entry::button(
                        "paper-timer",
                        if p.started_at.is_some() {
                            "暂停 / 完成计时"
                        } else {
                            "开始 / 继续纸上作答"
                        },
                        true,
                    ));
                    rows.push(Entry::button(
                        "paper-upload",
                        "上传整页或多页答卷照片",
                        false,
                    ));
                    let mut seen = HashSet::new();
                    let attempts: Vec<_> = self
                        .list("attempt")
                        .into_iter()
                        .filter(|a| {
                            a.payload["paperId"].as_str() == Some(&p.id)
                                && seen.insert(
                                    a.payload["questionId"].as_str().unwrap_or("").to_owned(),
                                )
                        })
                        .collect();
                    rows.push(Entry::text(
                        "paper-progress",
                        format!("已核对 {} / {} 题", attempts.len(), p.questions.len()),
                        "识别完成后，先核对题号和答案，再记录成绩。",
                    ));
                    if attempts.iter().any(|r| r.payload["pending"] == true) {
                        rows.push(Entry::button("paper-grade", "AI批改本卷主观题", true));
                    }
                    for a in attempts {
                        rows.push(Entry::choice(
                            &format!("result:{}", a.entity_id),
                            format!(
                                "第{}题 · {}",
                                a.payload["paperNumber"],
                                a.payload["title"].as_str().unwrap_or("")
                            ),
                            if a.payload["pending"] == true {
                                "待评分".into()
                            } else {
                                format!("{} / {} 分", a.payload["score"], a.payload["maxScore"])
                            },
                            false,
                        ));
                    }
                }
            }
            "paper-review" => self.review_rows(&mut rows),
            "history" => {
                if let Ok(mut records) = self.store.revisions(None) {
                    records.reverse();
                    for r in records.into_iter().take(self.visible_count) {
                        rows.push(Entry::choice(
                            &format!("restore:{}", r.version),
                            format!("{} · {}", r.kind, domain::clip(&r.entity_id, 24)),
                            domain::clip(&r.payload.to_string(), 180),
                            false,
                        ));
                    }
                    rows.push(Entry::button("more", "显示更多历史", false));
                }
            }
            "conflicts" => match self.store.conflicts() {
                Ok(groups) => {
                    if groups.is_empty() {
                        rows.push(Entry::text(
                            "no-conflicts",
                            "没有待处理冲突",
                            "多个设备的冲突版本会同时保留，可在这里选择。",
                        ));
                    }
                    for group in groups {
                        for r in group {
                            rows.push(Entry::choice(
                                &format!("resolve:{}", r.version),
                                format!(
                                    "{} · {}{}",
                                    r.kind,
                                    domain::clip(&r.entity_id, 24),
                                    if r.deleted { " · 删除版本" } else { "" }
                                ),
                                domain::clip(&r.payload.to_string(), 1000),
                                false,
                            ));
                        }
                    }
                }
                Err(e) => rows.push(Entry::text("conflict-error", "无法读取冲突", e.to_string())),
            },
            _ => match self.tab {
                0 => self.home_rows(&mut rows),
                1 => self.bank_rows(&mut rows),
                2 => self.material_rows(&mut rows),
                _ => self.settings_rows(&mut rows),
            },
        }
        rows
    }
    fn image_row(&self, rows: &mut Vec<Entry>, key: &str, hash: &str) {
        if hash.is_empty() {
            return;
        }
        if let Ok(path) = self.store.asset_path(hash) {
            rows.push(Entry {
                kind: 7,
                image: path.to_string_lossy().into(),
                ..Entry::text(key, "图片", "")
            });
        }
    }
    fn home_rows(&self, rows: &mut Vec<Entry>) {
        if self.list("question").is_empty() {
            rows.push(Entry::text(
                "onboarding",
                "从一份题库开始",
                "导入JSON题库，也可拍照识别整页试题。教材可在资料页关联到题库。",
            ));
            rows.push(Entry::pair("pick:1", "导入题库", "sample", "体验示例"));
        }
        let sessions: Vec<_> = self
            .list("session")
            .into_iter()
            .filter(|r| r.payload["submitted"] != true)
            .collect();
        if !sessions.is_empty() {
            rows.push(Entry {
                kind: 8,
                ..Entry::text("resume-heading", "继续学习", "")
            });
            for r in sessions.iter().take(3) {
                rows.push(Entry::choice(
                    &format!("resume:{}", r.entity_id),
                    r.payload["title"]
                        .as_str()
                        .filter(|s| !s.is_empty())
                        .unwrap_or("未完成练习"),
                    format!(
                        "第{}题 · 点击恢复",
                        r.payload["index"].as_u64().unwrap_or(0) + 1
                    ),
                    false,
                ));
            }
        }
        let attempts = self.list("attempt");
        if !attempts.is_empty() {
            rows.push(Entry {
                kind: 8,
                ..Entry::text("recent-heading", "最近作答", "")
            });
            for r in attempts.into_iter().take(5) {
                rows.push(Entry::choice(
                    &format!("result:{}", r.entity_id),
                    r.payload["title"].as_str().unwrap_or("作答记录"),
                    if r.payload["pending"] == true {
                        "等待AI批改".into()
                    } else {
                        format!("{} / {}分", r.payload["score"], r.payload["maxScore"])
                    },
                    false,
                ));
            }
        }
        rows.push(Entry::button("wrong", "复习我的错题", false));
        for r in self.list("plan").into_iter().take(10) {
            rows.push(Entry::choice(
                &format!("draft:{}", r.entity_id),
                r.payload["title"].as_str().unwrap_or("学习草稿"),
                "查看、编辑并确认",
                false,
            ));
        }
    }
    fn bank_rows(&self, rows: &mut Vec<Entry>) {
        let banks = learning::banks(&self.store).unwrap_or_default();
        let Some(bank) = banks.iter().find(|bank| bank.id == self.selected_bank) else {
            rows.push(Entry::pair("pick:1", "导入JSON", "pick:5", "拍照识题"));
            if banks.is_empty() {
                rows.push(Entry::text(
                    "no-bank",
                    "从第一份题库开始",
                    "支持一个题库包含多套试卷。导入后可分别按套卷、章节或知识点练习。",
                ));
                rows.push(Entry::pair(
                    "sample",
                    "体验示例",
                    "new-question",
                    "手动新增题目",
                ));
            }
            for bank in &banks {
                rows.push(Entry::choice(
                    &format!("bank:{}", bank.id),
                    &bank.title,
                    format!(
                        "{}题 · 关联{}份资料",
                        bank.question_ids.len(),
                        bank.material_ids.len()
                    ),
                    false,
                ));
            }
            if !banks.is_empty() {
                rows.push(Entry::button("all-practice", "混合练习所有题库", false));
            }
            return;
        };
        let questions = self.filtered_questions();
        rows.push(Entry::text(
            "scope",
            self.scope_title(),
            format!(
                "当前范围 {} 题 · 关联 {} 份资料",
                questions.len(),
                bank.material_ids.len()
            ),
        ));
        rows.push(Entry::pair("practice", "逐题练习", "exam", "计时考试"));
        if bank.directory.is_empty() {
            rows.push(Entry::pair("bank:", "切换题库", "links", "关联资料"));
        } else {
            rows.push(Entry::pair(
                "choose-directory",
                "选择套卷目录",
                "links",
                "关联资料",
            ));
            rows.push(Entry::button("bank:", "切换题库", false));
        }
        rows.push(Entry::pair(
            "paper-new",
            "导出纸上试卷",
            "generate-page",
            "AI出新题",
        ));
        rows.push(Entry::field(
            "filter",
            "搜索题干、章节或知识点",
            &self.filter,
            false,
        ));
        rows.push(Entry::pair(
            "export-bank",
            "导出标准JSON",
            "new-question",
            "新增题目",
        ));
        rows.push(Entry::pair("pick:1", "导入JSON", "pick:5", "拍照识题"));
        for q in questions.iter().take(self.visible_count) {
            rows.push(Entry::choice(
                &format!("edit-question:{}", q.id),
                &q.stem,
                format!(
                    "{} · {}分 · {}",
                    domain::label(&q.kind),
                    q.max_score,
                    q.chapter
                ),
                false,
            ));
        }
        if questions.len() > self.visible_count {
            rows.push(Entry::button(
                "more",
                format!("继续显示 · 还有{}题", questions.len() - self.visible_count),
                false,
            ));
        }
        if questions.is_empty() {
            rows.push(Entry::text(
                "no-matches",
                "没有匹配的题目",
                "尝试清除搜索条件或切换套卷。",
            ));
        }
    }
    fn directory_rows(&self, rows: &mut Vec<Entry>, entries: &[DirectoryEntry], depth: usize) {
        for entry in entries {
            let title = format!("{}{}", "　".repeat(depth), entry.title);
            if entry.children.is_empty() {
                rows.push(Entry::choice(
                    &format!("directory:{}", entry.id),
                    title,
                    format!(
                        "{}题{}",
                        entry.ids().len(),
                        entry
                            .time_limit_minutes
                            .map_or(String::new(), |n| format!(" · {n}分钟"))
                    ),
                    self.selected_directory == entry.id,
                ));
            } else {
                rows.push(Entry {
                    kind: 8,
                    ..Entry::text(&entry.id, title, "")
                });
                self.directory_rows(rows, &entry.children, depth + 1);
            }
        }
    }
    fn material_rows(&self, rows: &mut Vec<Entry>) {
        rows.push(Entry::text(
            "material-help",
            "教材、考纲和往年试题",
            "导入后可离线阅读与搜索。勾选资料用于AI出题；题库可另外保存资料关联。",
        ));
        rows.push(Entry::button(
            "material-import",
            "导入教材、考纲或往年试题",
            true,
        ));
        rows.push(Entry::field("filter", "离线搜索原文", &self.filter, false));
        if !self.filter.is_empty() {
            if let Ok(hits) = materials::search(&self.store, &self.filter, &self.selected) {
                if let Some(hits) = hits.as_array() {
                    if hits.is_empty() {
                        rows.push(Entry::text("no-hits", "没有找到原文", "尝试更短的关键词。"));
                    }
                    for (i, h) in hits.iter().enumerate() {
                        rows.push(Entry::text(
                            &format!("hit-{i}"),
                            format!(
                                "{} · {}",
                                h["title"].as_str().unwrap_or(""),
                                h["locator"].as_str().unwrap_or("")
                            ),
                            h["text"].as_str().unwrap_or(""),
                        ));
                    }
                }
            }
        }
        let materials = self.list("material");
        if materials.is_empty() {
            rows.push(Entry::text(
                "no-materials",
                "还没有资料",
                "文字版文档可离线提取；图片和扫描页面可使用已配置的视觉模型识别。",
            ));
        }
        for m in materials {
            rows.push(Entry::check(
                &format!("material-select:{}", m.entity_id),
                m.payload["title"].as_str().unwrap_or("资料"),
                self.selected.contains(&m.entity_id),
            ));
            rows.push(Entry::button(
                &format!("material:{}", m.entity_id),
                "阅读与图片识别",
                false,
            ));
        }
    }
    fn material_import_rows(&self, rows: &mut Vec<Entry>) {
        for (i, label) in ["教材", "考纲", "往年试题"].iter().enumerate() {
            rows.push(Entry::choice(
                &format!("role:{i}"),
                *label,
                "导入资料类型",
                self.role == i,
            ));
        }
        rows.push(Entry::field(
            "strict",
            "本资料的严格要求（可选）",
            &self.strict,
            false,
        ));
        rows.push(Entry::button(
            "pick:2",
            "导入PDF、Word、TXT或Markdown",
            true,
        ));
    }
    fn settings_rows(&self, rows: &mut Vec<Entry>) {
        rows.push(Entry::button("save-config", "保存连接配置", true));
        rows.push(Entry {
            kind: 8,
            ..Entry::text("model-heading", "模型与搜索", "")
        });
        rows.push(Entry::field(
            "aiBase",
            "AI服务地址（OpenAI兼容）",
            &self.config.ai_base,
            false,
        ));
        self.secret_rows(rows, "aiKey", "API Key", &self.config.ai_key);
        rows.push(Entry::field("model", "模型名称", &self.config.model, false));
        rows.push(Entry::text(
            "model-hint",
            "统一模型配置",
            "助教、批改、识图和出题共用。请使用支持图片的模型。",
        ));
        rows.push(Entry::field(
            "searchBase",
            "DeepSeek搜索服务地址",
            &self.config.search_base,
            false,
        ));
        self.secret_rows(
            rows,
            "searchKey",
            "搜索Key（可留空复用DeepSeek官方Key）",
            &self.config.search_key,
        );
        rows.push(Entry::button("test-search", "测试DeepSeek搜索", false));
        rows.push(Entry {
            kind: 8,
            ..Entry::text("dav-heading", "WebDAV同步", "")
        });
        rows.push(Entry::field(
            "davUrl",
            "WebDAV根目录HTTPS地址",
            &self.config.dav_url,
            false,
        ));
        rows.push(Entry::field(
            "davUser",
            "用户名",
            &self.config.dav_user,
            false,
        ));
        self.secret_rows(
            rows,
            "davPassword",
            "应用专用密码",
            &self.config.dav_password,
        );
        rows.push(Entry::text(
            "dav-hint",
            "同步范围",
            "题库、资料、图片、答题、错题与笔记。云端文件可直接读取，冲突保留双方；凭据只存本机。",
        ));
        rows.push(Entry::button("save-config", "保存连接配置", true));
        rows.push(Entry::button("sync", "立即双向同步", false));
        rows.push(Entry {
            kind: 8,
            ..Entry::text("data-heading", "数据与恢复", "")
        });
        rows.push(Entry::pair(
            "backup",
            "导出完整备份",
            "pick:4",
            "合并导入备份",
        ));
        rows.push(Entry::pair(
            "conflicts",
            "处理同步冲突",
            "history",
            "历史版本",
        ));
        rows.push(Entry::text(
            "about",
            "EasyGot 0.3 · Slint重构预览",
            "Rust业务与界面 · 内置学习Agent\nGPL-3.0-only\nhttps://github.com/weimanmk/EasyGot",
        ));
    }
    fn secret_rows(&self, rows: &mut Vec<Entry>, key: &str, title: &str, value: &str) {
        rows.push(Entry {
            kind: 9,
            ..Entry::field(key, title, value, false)
        });
        rows.push(Entry::pair(
            &format!("paste:{key}"),
            "粘贴",
            &format!("native:{key}"),
            "系统输入 / 长按粘贴",
        ));
    }
    fn question_editor_rows(&self, rows: &mut Vec<Entry>) {
        if let Some(q) = &self.edit_question {
            rows.push(Entry {
                kind: 13,
                ..Entry::field("q-type", "题型", &q.kind, false)
            });
            rows.push(Entry::field("q:stem", "题干", &q.stem, true));
            rows.push(Entry::field(
                "q:score",
                "分值",
                q.max_score.to_string(),
                false,
            ));
            for (i, o) in q.options.iter().enumerate() {
                rows.push(Entry::field(
                    &format!("q:option-{i}"),
                    &format!("选项{}", o.id),
                    &o.text,
                    true,
                ));
                if matches!(q.kind.as_str(), "single" | "multiple") {
                    rows.push(Entry::check(
                        &format!("q-correct:{}", o.id),
                        format!("{}是正确选项", o.id),
                        q.answer
                            .as_array()
                            .is_some_and(|a| a.contains(&json!(o.id))),
                    ));
                }
            }
            if matches!(q.kind.as_str(), "single" | "multiple") {
                rows.push(Entry::button("q-add-option", "增加选项", false));
            } else if q.kind == "boolean" {
                for (v, t) in [(true, "正确"), (false, "错误")] {
                    rows.push(Entry::choice(
                        &format!("q-correct:{v}"),
                        t,
                        "标准答案",
                        q.answer == json!(v),
                    ));
                }
            } else {
                rows.push(Entry::field(
                    "q:reference",
                    "参考答案",
                    &q.reference_answer,
                    true,
                ));
            }
            rows.push(Entry::field("q:explanation", "解析", &q.explanation, true));
            rows.push(Entry::field("q:chapter", "章节", &q.chapter, false));
            rows.push(Entry::field(
                "q:knowledge",
                "知识点（逗号分隔）",
                q.knowledge_points.join("，"),
                false,
            ));
            rows.push(Entry::field(
                "q:strict",
                "严格评分要求",
                &q.strict_requirements,
                true,
            ));
            rows.push(Entry::button("q-save", "校验并保存题目", true));
        }
    }
    fn review_rows(&self, rows: &mut Vec<Entry>) {
        rows.push(Entry::text(
            "review-help",
            "逐题核对后记录成绩",
            "低置信度、模糊或涂改内容需重点核对。修改题号或答案后，必须重新勾选本题确认。",
        ));
        if let Some(p) = &self.paper {
            for (i, h) in p.pages.iter().enumerate() {
                self.image_row(rows, &format!("original-{i}"), h);
            }
        }
        if let Ok(answers) = serde_json::from_str::<Vec<RecognizedAnswer>>(&self.paper_review) {
            for (i, a) in answers.iter().enumerate() {
                rows.push(Entry::field(
                    &format!("review:{i}:number"),
                    "识别题号（可修正）",
                    a.number.to_string(),
                    false,
                ));
                if let Some(q) = self
                    .paper
                    .as_ref()
                    .and_then(|p| p.questions.iter().find(|q| q.number == a.number))
                {
                    rows.push(Entry::text(
                        &format!("review-stem-{i}"),
                        format!("第{}题 · 置信度{:.0}%", a.number, a.confidence * 100.),
                        &q.question.stem,
                    ));
                    match q.question.kind.as_str() {
                        "single" | "multiple" => {
                            for o in &q.question.options {
                                rows.push(Entry::choice(
                                    &format!("review-option:{i}:{}", o.id),
                                    format!("{}. {}", o.id, o.text),
                                    "",
                                    a.selected.contains(&o.id),
                                ));
                            }
                        }
                        "boolean" => {
                            for (v, t) in [(true, "正确"), (false, "错误")] {
                                rows.push(Entry::choice(
                                    &format!("review-verdict:{i}:{v}"),
                                    t,
                                    "",
                                    a.verdict == Some(v),
                                ));
                            }
                        }
                        _ => rows.push(Entry::field(
                            &format!("review:{i}:text"),
                            "识别出的作答原文",
                            &a.text,
                            true,
                        )),
                    }
                }
                rows.push(Entry::check(
                    &format!("review-check:{i}"),
                    if a.confidence < 0.85 {
                        "已重点核对这道低置信度试题"
                    } else {
                        "已核对本题识别结果"
                    },
                    !a.uncertain,
                ));
            }
        }
        rows.push(Entry::check(
            "review-confirm",
            "我已核对全部题号和作答",
            self.paper_review_confirmed,
        ));
        rows.push(Entry::button("review-save", "确认作答并记录成绩", true));
    }
}
