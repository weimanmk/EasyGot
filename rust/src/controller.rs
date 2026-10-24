//! Application controller: business actions and state, independent of the rendering framework.
use crate::{
    agent, ai,
    domain::{self, Bank, DirectoryEntry, Question},
    learning::{self, Paper, RecognizedAnswer},
    materials,
    network::Config,
    platform::Bridge,
    practice::{Answer, Session},
    store::{asset_refs, id, now, sha, Revision, Store},
    sync, vision,
};
use anyhow::{ensure, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
};
enum Output {
    Info(String),
    Bank(Bank),
    Material(Value),
    Grade(Value),
    Export(Value, String),
    Synced(String),
    Chat(String),
    PaperRecognized(Vec<RecognizedAnswer>),
    Pdf(String),
    PaperGraded(String),
}
#[derive(Clone, Default, PartialEq)]
pub struct Entry {
    pub key: String,
    pub kind: i32,
    pub title: String,
    pub detail: String,
    pub value: String,
    pub action: String,
    pub action2: String,
    pub label2: String,
    pub checked: bool,
    pub enabled: bool,
    pub image: String,
}
impl Entry {
    fn text(key: &str, title: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            title: title.into(),
            detail: detail.into(),
            enabled: true,
            ..Self::default()
        }
    }
    fn button(key: &str, title: impl Into<String>, primary: bool) -> Self {
        Self {
            kind: if primary { 1 } else { 2 },
            action: key.into(),
            ..Self::text(key, title, "")
        }
    }
    fn field(key: &str, title: &str, value: impl Into<String>, multi: bool) -> Self {
        Self {
            kind: if multi { 5 } else { 4 },
            value: value.into(),
            ..Self::text(key, title, "")
        }
    }
    fn check(key: &str, title: impl Into<String>, checked: bool) -> Self {
        Self {
            kind: 6,
            action: key.into(),
            checked,
            ..Self::text(key, title, "")
        }
    }
    fn choice(
        key: &str,
        title: impl Into<String>,
        detail: impl Into<String>,
        checked: bool,
    ) -> Self {
        Self {
            kind: 3,
            action: key.into(),
            checked,
            ..Self::text(key, title, detail)
        }
    }
    fn pair(key: &str, label: &str, other: &str, label2: &str) -> Self {
        Self {
            kind: 12,
            action: key.into(),
            action2: other.into(),
            label2: label2.into(),
            ..Self::text(key, label, "")
        }
    }
}
pub struct EasyGot {
    pub store: Store,
    pub bridge: Bridge,
    pub config: Config,
    cancel: Arc<AtomicBool>,
    agent_session: String,
    pub allow_web: bool,
    pub chat_answer: String,
    pub tab: usize,
    pub view: String,
    pub status: String,
    pub busy: bool,
    rx: mpsc::Receiver<Result<Output, String>>,
    tx: mpsc::Sender<Result<Output, String>>,
    filter: String,
    selected: HashSet<String>,
    role: usize,
    strict: String,
    material: Option<Revision>,
    session: Option<Session>,
    session_base: Option<String>,
    result: Option<Value>,
    image_for: String,
    editor: String,
    draft_kind: String,
    edit_id: String,
    edit_base: Option<String>,
    pub confirm: String,
    pub query: String,
    spec: String,
    minutes: u32,
    wrong: bool,
    selected_bank: String,
    selected_directory: String,
    link_bank: String,
    link_materials: HashSet<String>,
    pub assistant_open: bool,
    assistant_context: String,
    pub paper: Option<Paper>,
    paper_review: String,
    paper_review_confirmed: bool,
    pub assistant_pos: (f32, f32),
    pdf_path: String,
    show_answers: bool,
    pub dirty: bool,
    pending_save: bool,
    last_edit: u64,
    pub visible_count: usize,
    edit_question: Option<Question>,
    edit_parent: bool,
    navigation: Vec<(usize, String)>,
}
impl EasyGot {
    pub fn new(bridge: Bridge) -> Result<Self> {
        let store = Store::open(bridge.root())?;
        learning::migrate_legacy(&store)?;
        let config = serde_json::from_str(&bridge.call("loadConfig", "")?)?;
        let (tx, rx) = mpsc::channel();
        Ok(Self {
            store,
            bridge,
            config,
            cancel: Arc::new(AtomicBool::new(false)),
            agent_session: "learning-assistant".into(),
            allow_web: false,
            chat_answer: String::new(),
            tab: 0,
            view: String::new(),
            status: String::new(),
            busy: false,
            rx,
            tx,
            filter: String::new(),
            selected: HashSet::new(),
            role: 0,
            strict: String::new(),
            material: None,
            session: None,
            session_base: None,
            result: None,
            image_for: String::new(),
            editor: String::new(),
            draft_kind: String::new(),
            edit_id: String::new(),
            edit_base: None,
            confirm: String::new(),
            query: String::new(),
            spec: serde_json::to_string_pretty(
                &json!({"title":"专项练习","chapter":"","knowledgePoints":"","count":5,"difficulty":3,"typeCounts":{"single":2,"multiple":1,"boolean":1,"short":1},"weights":{}}),
            )?,
            minutes: 30,
            wrong: true,
            selected_bank: String::new(),
            selected_directory: String::new(),
            link_bank: String::new(),
            link_materials: HashSet::new(),
            assistant_open: false,
            assistant_context: String::new(),
            paper: None,
            paper_review: String::new(),
            paper_review_confirmed: false,
            assistant_pos: (274., 90.),
            pdf_path: String::new(),
            show_answers: false,
            dirty: true,
            pending_save: false,
            last_edit: 0,
            visible_count: 40,
            edit_question: None,
            edit_parent: false,
            navigation: Vec::new(),
        })
    }
    fn job(&mut self, title: &str, work: impl FnOnce() -> Result<Output> + Send + 'static) {
        if self.busy {
            self.status = "正在处理上一项任务，请稍候".into();
            return;
        }
        self.dirty = true;
        self.cancel.store(false, Ordering::Relaxed);
        self.busy = true;
        self.status = title.into();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(work))
                .map_err(|_| "任务意外终止，原数据保留".to_owned())
                .and_then(|r| r.map_err(|e| e.to_string()));
            let _ = tx.send(result);
        });
    }
    fn info(&mut self, result: Result<()>) {
        match result {
            Ok(()) => self.status = "已保存".into(),
            Err(e) => self.status = e.to_string(),
        }
    }
    fn list(&self, kind: &str) -> Vec<Revision> {
        self.store.list(kind).unwrap_or_default()
    }
    fn bank_import(&mut self, mut bank: Bank) -> Result<()> {
        bank.validate()?;
        for (hash, value) in &bank.assets {
            let bytes = STANDARD.decode(value)?;
            ensure!(sha(&bytes) == *hash, "素材校验失败");
            self.store.asset(&bytes)?;
        }
        let refs = asset_refs(&serde_json::to_value(bank.all_questions())?);
        for hash in refs {
            ensure!(self.store.asset_path(&hash)?.exists(), "题目缺少图片素材");
        }
        self.selected_bank = learning::import_bank(&self.store, &bank)?;
        self.selected_directory.clear();
        Ok(())
    }
    pub fn poll(&mut self) {
        while let Ok(result) = self.rx.try_recv() {
            self.dirty = true;
            self.busy = false;
            match result {
                Err(error) => self.status = error,
                Ok(Output::Info(s)) => self.status = s,
                Ok(Output::PaperRecognized(answers)) => {
                    if let Some(paper) = self.paper.as_ref() {
                        if let Ok(Some(record)) = self.store.get(&paper.id) {
                            self.paper = serde_json::from_value(record.payload).ok();
                        }
                    }
                    self.paper_review = serde_json::to_string_pretty(&answers).unwrap_or_default();
                    self.paper_review_confirmed = false;
                    self.view = "paper-review".into();
                    self.status = "识别完成，请逐题核对后再批改".into();
                }
                Ok(Output::Pdf(path)) => {
                    self.pdf_path = path;
                    self.status = "PDF试卷已生成，可保存或打印".into();
                }
                Ok(Output::PaperGraded(message)) => {
                    self.status = message;
                    self.view = "paper".into();
                }
                Ok(Output::Chat(s)) => {
                    self.chat_answer = s;
                    self.status = "分析完成".into();
                }
                Ok(Output::Synced(s)) => {
                    self.status = s;
                    self.session = None;
                    self.session_base = None;
                }
                Ok(Output::Bank(bank)) => {
                    self.editor = serde_json::to_string_pretty(&bank).unwrap_or_default();
                    self.draft_kind = "bank".into();
                    self.view = "editor".into();
                    self.status = "请核对题目、答案和来源，再确认入库".into();
                }
                Ok(Output::Material(material)) => {
                    self.material = self
                        .store
                        .get(material["id"].as_str().unwrap_or(""))
                        .ok()
                        .flatten();
                    self.view = "material".into();
                    self.status = "资料已保存，可离线阅读检索".into();
                }
                Ok(Output::Grade(value)) => {
                    self.result = Some(value);
                    self.view = "result".into();
                    self.status = "评分已保存，请结合参考答案核对".into();
                }
                Ok(Output::Export(value, name)) => {
                    let result = (|| -> Result<()> {
                        let path = self.store.root.join("export.json");
                        std::fs::write(&path, serde_json::to_vec_pretty(&value)?)?;
                        self.bridge
                            .call("export", &json!({"path":path,"name":name}).to_string())?;
                        Ok(())
                    })();
                    self.info(result);
                }
            }
        }
        if let Ok(event) = self.bridge.call("poll", "") {
            if !event.is_empty() {
                self.dirty = true;
                if let Ok(v) = serde_json::from_str::<Value>(&event) {
                    self.import_event(v);
                } else {
                    self.status = event;
                }
            }
        }
    }
    fn import_event(&mut self, event: Value) {
        let kind = event["kind"].as_u64().unwrap_or(0);
        if kind == 90 {
            self.assistant_open = false;
            match event["screen"].as_str().unwrap_or("") {
                "home" => self.go(0),
                "settings" => self.go(4),
                "assistant" => {
                    self.assistant_context = self.contextual_data().to_string();
                    self.assistant_open = true;
                }
                "directory" => {
                    if let Ok(bank) = serde_json::from_str::<Bank>(include_str!(
                        "../../examples/multi-paper-bank.json"
                    )) {
                        let _ = self.bank_import(bank);
                        self.selected_directory = "paper-01".into();
                        self.go(1);
                    }
                }
                "question" => {
                    if let Ok(bank) = serde_json::from_str::<Bank>(include_str!(
                        "../../examples/sample-bank.json"
                    )) {
                        let _ = self.bank_import(bank.clone());
                        let _ = self.new_session(bank.questions, false);
                    }
                }
                "sample" => {
                    if let Ok(bank) = serde_json::from_str::<Bank>(include_str!(
                        "../../examples/sample-bank.json"
                    )) {
                        let _ = self.bank_import(bank);
                    }
                    self.go(0);
                }
                "paper" => {
                    if let Ok(bank) = serde_json::from_str::<Bank>(include_str!(
                        "../../examples/sample-bank.json"
                    )) {
                        let _ = self.bank_import(bank.clone());
                        if self.create_paper(bank.questions).is_ok() {
                            if let Some(paper) = self.paper.as_ref() {
                                let spec = paper.print_spec(false);
                                if let Ok(path) = self.bridge.call("printPaper", &spec.to_string())
                                {
                                    self.pdf_path = path.clone();
                                    let _ = std::fs::write(
                                        self.store.root.join("qa-paper-path.txt"),
                                        path,
                                    );
                                }
                                let _ = std::fs::write(
                                    self.store.root.join("qa-print-spec.json"),
                                    spec.to_string(),
                                );
                            }
                        }
                    }
                }
                "check-ball" => {
                    let pos = self.assistant_pos;
                    let _ = std::fs::write(
                        self.store.root.join("qa-ball.json"),
                        json!({"x":pos.0,"y":pos.1,"sampledAt":now()}).to_string(),
                    );
                }
                "check-paste" => {
                    let _ = std::fs::write(
                        self.store.root.join("qa-paste-result.json"),
                        json!({"passed":self.config.ai_key=="easygot-test-key"}).to_string(),
                    );
                }
                _ => {}
            }
            return;
        }
        if kind == 20 {
            self.status = "文件已导出".into();
            return;
        }
        if kind == 21 {
            let value = event["value"].as_str().unwrap_or("").to_owned();
            match event["field"].as_str().unwrap_or("") {
                "aiKey" => self.config.ai_key = value,
                "searchKey" => self.config.search_key = value,
                "davPassword" => self.config.dav_password = value,
                _ => {}
            }
            self.status = "已填入，请保存连接配置".into();
            return;
        }
        let path = event["path"].as_str().unwrap_or("").to_owned();
        let name = event["name"].as_str().unwrap_or("资料").to_owned();
        let store = self.store.clone();
        if kind == 5 || kind == 6 {
            let files = event["files"].as_array().cloned().unwrap_or_default();
            let config = self.config.clone();
            let paper = self.paper.clone();
            self.job("正在识别整页照片…", move || {
                let mut images = vec![];
                for file in files {
                    let data = std::fs::read(file["path"].as_str().unwrap_or(""))?;
                    image::guess_format(&data)?;
                    images.push(store.asset(&data)?);
                }
                if kind == 5 {
                    Ok(Output::Bank(vision::questions(&store, &config, &images)?))
                } else {
                    let mut paper = paper.ok_or_else(|| anyhow::anyhow!("请先选择要上传的试卷"))?;
                    paper.stop(now());
                    for hash in &images {
                        if !paper.pages.contains(hash) {
                            paper.pages.push(hash.clone());
                        }
                    }
                    learning::save_paper(&store, &paper)?;
                    Ok(Output::PaperRecognized(vision::paper_answers(
                        &store, &config, &paper, &images,
                    )?))
                }
            });
            return;
        }
        if kind == 1 {
            self.job("读取JSON题库…", move || {
                let data = std::fs::read(path)?;
                let bank: Bank = serde_json::from_slice(&data)?;
                bank.validate()?;
                Ok(Output::Bank(bank))
            });
        } else if kind == 2 {
            let role = ["textbook", "syllabus", "past_paper"][self.role].to_owned();
            let strict = self.strict.clone();
            self.job("提取教材原文…", move || {
                let data = std::fs::read(path)?;
                let material = materials::import(&store, &name, &data, &role, &strict)?;
                Ok(Output::Material(material))
            });
        } else if kind == 3 {
            let result = (|| -> Result<()> {
                let bytes = std::fs::read(path)?;
                image::guess_format(&bytes)?;
                let hash = store.asset(&bytes)?;
                if let Some(s) = self.session.as_mut() {
                    s.answers.entry(self.image_for.clone()).or_default().image = hash;
                }
                self.save_session()?;
                Ok(())
            })();
            self.info(result);
        } else if kind == 4 {
            self.job("合并完整备份…", move || {
                let value: Value = serde_json::from_slice(&std::fs::read(path)?)?;
                ensure!(value["backupVersion"] == 1, "备份版本不兼容");
                import_assets(&store, &value)?;
                let revisions: Vec<Revision> = serde_json::from_value(value["revisions"].clone())?;
                for hash in asset_refs(&value["revisions"]) {
                    ensure!(store.asset_path(&hash)?.exists(), "备份缺少素材");
                }
                store.ingest(&revisions)?;
                Ok(Output::Synced("备份已合并，请检查同步冲突".into()))
            });
        }
    }
    fn filtered_questions(&self) -> Vec<Question> {
        let questions = if let Some(bank) = learning::banks(&self.store)
            .unwrap_or_default()
            .into_iter()
            .find(|b| b.id == self.selected_bank)
        {
            learning::bank_questions(&self.store, &bank, &self.selected_directory)
                .unwrap_or_default()
        } else {
            self.list("question")
                .into_iter()
                .filter_map(|r| serde_json::from_value::<Question>(r.payload).ok())
                .collect()
        };
        questions
            .into_iter()
            .filter(|q| {
                format!("{} {} {}", q.stem, q.chapter, q.knowledge_points.join(" "))
                    .contains(&self.filter)
            })
            .collect()
    }
    fn scope_title(&self) -> String {
        if let Some(bank) = learning::banks(&self.store)
            .unwrap_or_default()
            .into_iter()
            .find(|b| b.id == self.selected_bank)
        {
            if let Some(entry) = DirectoryEntry::find(&bank.directory, &self.selected_directory) {
                return format!("{} · {}", bank.title, entry.title);
            }
            return bank.title;
        }
        "EasyGot专项试卷".into()
    }

    fn save_editor(&mut self) -> Result<()> {
        match self.draft_kind.as_str() {
            "bank" => {
                let bank: Bank = serde_json::from_str(&self.editor)?;
                self.bank_import(bank)?;
            }
            "question" => {
                let q: Question = serde_json::from_str(&self.editor)?;
                q.validate()?;
                self.store.save(
                    "question",
                    &q.id,
                    serde_json::to_value(&q)?,
                    self.edit_base.as_deref(),
                    false,
                    false,
                )?;
            }
            "note" => {
                self.store.save(
                    "note",
                    &self.edit_id,
                    json!({"id":self.edit_id,"text":self.editor}),
                    self.edit_base.as_deref(),
                    false,
                    false,
                )?;
            }
            "strict" => {
                let r = self
                    .material
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("未选择资料"))?;
                let mut m = r.payload.clone();
                m["strictRules"] = json!(self.editor);
                self.store
                    .save("material", &r.entity_id, m, Some(&r.version), false, false)?;
            }
            _ => {}
        }
        self.view.clear();
        Ok(())
    }
    fn start_ocr(&mut self) {
        let Some(revision) = self.material.clone() else {
            return;
        };
        let store = self.store.clone();
        let config = self.config.clone();
        let bridge = self.bridge.clone();
        self.job("逐页识别资料图片…", move || {
            let mut m = revision.payload;
            let mut base = revision.version;
            if m["format"] == "pdf" && m["pagesRendered"] != true {
                let path = store.asset_path(m["asset"].as_str().unwrap_or(""))?;
                let pages: Vec<Value> =
                    serde_json::from_str(&bridge.call("renderPdf", path.to_str().unwrap_or(""))?)?;
                for page in pages {
                    let path = page["path"].as_str().unwrap();
                    let hash = store.asset(&std::fs::read(path)?)?;
                    m["images"]
                        .as_array_mut()
                        .unwrap()
                        .push(json!({"asset":hash,"locator":page["locator"],"recognized":false}));
                    let _ = std::fs::remove_file(path);
                }
                m["pagesRendered"] = json!(true);
                base = store
                    .save(
                        "material",
                        m["id"].as_str().unwrap(),
                        m.clone(),
                        Some(&base),
                        false,
                        false,
                    )?
                    .version;
            }
            let length = m["images"].as_array().map_or(0, |v| v.len());
            for i in 0..length {
                if m["images"][i]["recognized"] == true {
                    continue;
                }
                let hash = m["images"][i]["asset"].as_str().unwrap_or("");
                let text = ai::recognize(&store, &config, hash)?;
                let locator = m["images"][i]["locator"].clone();
                m["sections"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"locator":locator,"text":text}));
                m["images"][i]["recognized"] = json!(true);
                base = store
                    .save(
                        "material",
                        m["id"].as_str().unwrap(),
                        m.clone(),
                        Some(&base),
                        false,
                        false,
                    )?
                    .version;
            }
            Ok(Output::Material(m))
        });
    }
    fn open_assistant_for(&mut self, prompt: &str, mut context: Value) {
        if let Ok(q) = serde_json::from_value::<Question>(context.clone()) {
            if let Ok(q) = learning::effective_question(&self.store, &q) {
                context = serde_json::to_value(q).unwrap_or(context);
            }
        }
        self.select_context_materials(&context);
        self.assistant_open = true;
        self.query = prompt.into();
        self.assistant_context = domain::clip(&context.to_string(), 12000);
    }
    fn select_context_materials(&mut self, context: &Value) {
        let mut refs = HashSet::new();
        if let Some(ids) = context["materialIds"]
            .as_array()
            .or_else(|| context["question"]["materialIds"].as_array())
            .or_else(|| context["bank"]["materialIds"].as_array())
        {
            refs.extend(ids.iter().filter_map(|v| v.as_str()).map(str::to_owned));
        }
        if let Some(id) = context["materialId"].as_str() {
            refs.insert(id.to_owned());
        }
        if let Some(questions) = context["paper"]["questions"].as_array() {
            for q in questions {
                if let Some(ids) = q["question"]["materialIds"].as_array() {
                    refs.extend(ids.iter().filter_map(|v| v.as_str()).map(str::to_owned));
                }
            }
        }
        if !refs.is_empty() {
            self.selected = refs;
        }
    }
    fn contextual_data(&self) -> Value {
        if self.view == "question" {
            if let Some(session) = &self.session {
                let q = &session.questions[session.index];
                return json!({"question":q,"studentAnswer":session.answers.get(&q.id),"materialIds":q.material_ids});
            }
        }
        if self.view == "result" {
            return self.result.clone().unwrap_or(Value::Null);
        }
        if self.view == "paper" || self.view == "paper-review" {
            return json!({"paper":self.paper,"recognizedAnswers":self.paper_review});
        }
        if self.view == "material" {
            return self
                .material
                .as_ref()
                .map(|r| json!({"materialId":r.entity_id,"title":r.payload["title"]}))
                .unwrap_or(Value::Null);
        }
        if !self.selected_bank.is_empty() {
            return json!({"bankId":self.selected_bank,"directoryId":self.selected_directory,"bank":learning::banks(&self.store).unwrap_or_default().into_iter().find(|b|b.id==self.selected_bank)});
        }
        json!({"screen":match self.tab{0=>"学习概览",1=>"题库",2=>"资料",_=>"连接设置"}})
    }
    fn create_paper(&mut self, questions: Vec<Question>) -> Result<()> {
        let questions = questions
            .iter()
            .map(|q| learning::effective_question(&self.store, q))
            .collect::<Result<Vec<_>>>()?;
        let title = self.scope_title();
        let paper = Paper::new(&title, questions)?;
        learning::save_paper(&self.store, &paper)?;
        self.paper = Some(paper);
        self.pdf_path.clear();
        self.view = "paper".into();
        Ok(())
    }
    fn grade_paper_pending(&mut self, paper_id: &str) {
        let paper_id = paper_id.to_owned();
        let store = self.store.clone();
        let config = self.config.clone();
        self.job(
            "正在对照参考答案和关联教材批改…",
            move || {
                let mut count = 0;
                for revision in store.list("attempt")? {
                    let mut attempt = revision.payload;
                    if attempt["paperId"].as_str() != Some(&paper_id) || attempt["pending"] != true
                    {
                        continue;
                    }
                    let question: Question = serde_json::from_value(attempt["question"].clone())?;
                    let grade = ai::grade(&store, &config, &question, &attempt["answer"])?;
                    for (key, value) in grade
                        .as_object()
                        .ok_or_else(|| anyhow::anyhow!("评分结构无效"))?
                    {
                        attempt[key] = value.clone();
                    }
                    attempt["pending"] = json!(false);
                    store.save(
                        "attempt",
                        attempt["id"].as_str().unwrap(),
                        attempt.clone(),
                        Some(&revision.version),
                        false,
                        false,
                    )?;
                    count += 1;
                }
                Ok(Output::PaperGraded(format!(
                    "本卷已完成{count}道主观题批改，错题已归入复习记录"
                )))
            },
        );
    }
}
impl EasyGot {
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
    fn new_session(&mut self, questions: Vec<Question>, exam: bool) -> Result<()> {
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
    fn finish_exam(&mut self) -> Result<()> {
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
    fn dispatch(&mut self, action: &str) -> Result<()> {
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
                    "../../examples/sample-bank.json"
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
            return "离开编辑？尚未确认的修改会留在本次打开的草稿中，返回编辑前请确认已经保存。";
        }
        match self.confirm.as_str(){"exam"=>"确认提交整卷？未作答题记为0分，主观题提交后可联网批改。", "ocr"=>"将文档图片或PDF页面发送给你配置的视觉模型，可能产生多次调用费用。已识别页面会保存。", "save"=>"确认保存？同id内容会更新，历史版本保留，可从设置中恢复。",_=>"确认恢复或合并该版本？历史记录会保留。"}
    }
    fn back(&mut self) {
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
    fn accept(&mut self) -> Result<()> {
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
    pub fn rows(&self) -> Vec<Entry> {
        let mut rows = vec![];
        match self.view.as_str() {
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
        rows.push(Entry::pair("pick:1", "导入JSON", "pick:5", "拍照识题"));
        let banks = learning::banks(&self.store).unwrap_or_default();
        rows.push(Entry::field(
            "filter",
            "搜索题干、章节或知识点",
            &self.filter,
            false,
        ));
        if banks.is_empty() {
            rows.push(Entry::text(
                "no-bank",
                "还没有题库",
                "支持一个题库包含多层目录、10套或更多试卷，每套题保留独立题目顺序。",
            ));
            rows.push(Entry::button("sample", "载入示例题库", true));
            return;
        }
        if let Some(bank) = banks.iter().find(|b| b.id == self.selected_bank) {
            rows.push(Entry::text(
                "selected-bank",
                &bank.title,
                format!(
                    "{}道题 · 关联{}份资料",
                    bank.question_ids.len(),
                    bank.material_ids.len()
                ),
            ));
            rows.push(Entry::pair("bank:", "切换题库", "links", "关联资料"));
            if !bank.directory.is_empty() {
                rows.push(Entry::button("choose-directory", "选择套卷目录", false));
            }
        } else {
            rows.push(Entry::text(
                "bank-list",
                "我的题库",
                "选择题库可按套卷练习、考试或关联教材。",
            ));
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
        }
        let questions = self.filtered_questions();
        rows.push(Entry::text(
            "scope",
            self.scope_title(),
            format!("当前范围共 {} 道题", questions.len()),
        ));
        rows.push(Entry::pair("practice", "逐题练习", "exam", "计时考试"));
        rows.push(Entry::field(
            "minutes",
            "考试时长（分钟，1–360）",
            self.minutes.to_string(),
            false,
        ));
        rows.push(Entry::pair(
            "paper-new",
            "导出纸上试卷",
            "generate-page",
            "AI出新题",
        ));
        rows.push(Entry::pair(
            "export-bank",
            "导出标准JSON",
            "new-question",
            "新增题目",
        ));
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
            for kind in domain::TYPES {
                rows.push(Entry::choice(
                    &format!("q-type:{kind}"),
                    domain::label(kind),
                    "",
                    q.kind == kind,
                ));
            }
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
fn duration(milliseconds: u64) -> String {
    let seconds = milliseconds / 1000;
    format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600,
        seconds / 60 % 60,
        seconds % 60
    )
}
fn import_assets(store: &Store, value: &Value) -> Result<()> {
    let mut total = 0;
    if let Some(assets) = value["assets"].as_object() {
        for (hash, b64) in assets {
            let bytes = STANDARD.decode(b64.as_str().unwrap_or(""))?;
            total += bytes.len();
            ensure!(total <= 60 * 1024 * 1024, "备份素材总量超过60MB");
            ensure!(sha(&bytes) == *hash, "备份素材校验失败");
            store.asset(&bytes)?;
        }
    }
    Ok(())
}
fn add_assets(store: &Store, value: &mut Value) -> Result<()> {
    let mut assets = serde_json::Map::new();
    let mut total = 0;
    for hash in asset_refs(value) {
        let bytes = std::fs::read(store.asset_path(&hash)?)?;
        total += bytes.len();
        ensure!(
            total <= 60 * 1024 * 1024,
            "JSON备份素材超过60MB，请使用WebDAV"
        );
        assets.insert(hash, json!(STANDARD.encode(bytes)));
    }
    value["assets"] = json!(assets);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn app() -> EasyGot {
        EasyGot::new(Bridge {
            root: std::env::temp_dir().join(id()),
        })
        .unwrap()
    }
    fn cleanup(app: EasyGot) {
        std::fs::remove_dir_all(app.store.root).unwrap();
    }
    #[test]
    fn all_primary_pages_have_actions_and_settings_keep_password_fields() {
        let mut app = app();
        for tab in [0, 1, 2, 4] {
            app.go(tab);
            let rows = app.rows();
            assert!(!rows.is_empty());
            assert!(rows
                .iter()
                .filter(|r| matches!(r.kind, 1 | 2 | 3 | 6 | 12))
                .all(|r| !r.action.is_empty()));
        }
        assert!(app.rows().iter().any(|r| r.key == "aiKey" && r.kind == 9));
        cleanup(app);
    }
    #[test]
    fn practice_navigation_and_restart_preserve_answers() {
        let mut app = app();
        app.dispatch("sample").unwrap();
        app.dispatch("practice").unwrap();
        let q = app.session.as_ref().unwrap().current().unwrap().clone();
        app.dispatch(&format!("answer-option:{}", q.options[0].id))
            .unwrap();
        app.dispatch("submit-one").unwrap();
        assert_eq!(app.store.list("attempt").unwrap().len(), 1);
        let session_id = app.session.as_ref().unwrap().id.clone();
        app.dispatch("continue").unwrap();
        app.dispatch("next").unwrap();
        app.save_session().unwrap();
        let bridge = app.bridge.clone();
        drop(app);
        let mut app = EasyGot::new(bridge).unwrap();
        app.dispatch(&format!("resume:{session_id}")).unwrap();
        assert_eq!(app.session.as_ref().unwrap().index, 1);
        assert_eq!(
            app.session.as_ref().unwrap().answers[&q.id].selected,
            vec![q.options[0].id.clone()]
        );
        cleanup(app);
    }
    #[test]
    fn ten_paper_scope_and_links_survive_controller_restart() {
        let mut app = app();
        app.bank_import(
            serde_json::from_str(include_str!("../../examples/multi-paper-bank.json")).unwrap(),
        )
        .unwrap();
        app.dispatch("directory:paper-10").unwrap();
        assert_eq!(app.filtered_questions().len(), 2);
        assert!(app.scope_title().contains("10"));
        app.dispatch("exam").unwrap();
        assert_eq!(app.session.as_ref().unwrap().questions.len(), 2);
        app.dispatch("submit-exam").unwrap();
        assert_eq!(app.store.list("attempt").unwrap().len(), 0);
        app.dispatch("confirm-yes").unwrap();
        assert_eq!(app.store.list("attempt").unwrap().len(), 2);
        assert!(app.session.as_ref().unwrap().submitted);
        cleanup(app);
    }
    #[test]
    fn photo_review_requires_reconfirmation_after_edits() {
        let mut app = app();
        app.paper_review = serde_json::to_string(&vec![RecognizedAnswer {
            number: 1,
            text: "a".into(),
            selected: vec![],
            verdict: None,
            confidence: 0.9,
            uncertain: false,
        }])
        .unwrap();
        app.paper_review_confirmed = true;
        app.set_field("review:0:text", "edited".into()).unwrap();
        let answers: Vec<RecognizedAnswer> = serde_json::from_str(&app.paper_review).unwrap();
        assert!(answers[0].uncertain);
        assert!(!app.paper_review_confirmed);
        cleanup(app);
    }
    #[test]
    fn generation_is_a_form_and_import_is_not_committed_before_confirmation() {
        let mut app = app();
        app.dispatch("generate-page").unwrap();
        assert!(app.rows().iter().any(|r| r.key == "spec:count"));
        assert!(!app.rows().iter().any(|r| r.key == "editor"));
        app.editor = include_str!("../../examples/multi-paper-bank.json").into();
        app.draft_kind = "bank".into();
        app.view = "editor".into();
        assert!(app
            .rows()
            .iter()
            .any(|r| r.action.starts_with("draft-question:")));
        app.dispatch("save-editor").unwrap();
        assert!(app.store.list("question").unwrap().is_empty());
        app.dispatch("confirm-yes").unwrap();
        assert_eq!(app.store.list("question").unwrap().len(), 20);
        cleanup(app);
    }
}
