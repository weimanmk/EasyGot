mod actions;
mod pages;
// Application controller: business actions and state, independent of the rendering framework.
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
    pub qa_snapshot_requested: bool,
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
            qa_snapshot_requested: false,
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
                "snapshot" => self.qa_snapshot_requested = true,
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
                        self.view = "directory".into();
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
