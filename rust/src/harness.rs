//! Full upstream DSH runs in the embedded proot/Node sidecar. Rust exposes learning tools over MCP.
use anyhow::{ensure,Result};use serde_json::{json,Value};use std::{sync::{Arc,RwLock,OnceLock},io::Read,collections::HashSet};use crate::{store::{Store,id},network::{self,Config},domain::{self,Bank,Question},materials,ai};
pub const PORT:u16=38641;
static STARTED:OnceLock<()>=OnceLock::new();
pub type SharedConfig=Arc<RwLock<Config>>;
pub fn tools()->Value{let query=json!({"type":"object","properties":{"query":{"type":"string"},"materialIds":{"type":"array","items":{"type":"string"}}},"required":["query"]});json!([
    {"name":"search_materials","description":"搜索本地教材、考纲、往年试题，返回带文件和页码/段落的原文","inputSchema":query},
    {"name":"get_wrong_questions","description":"读取最近失分题及作答记录","inputSchema":{"type":"object","properties":{}}},
    {"name":"deepseek_search","description":"使用配置的DeepSeek Search接口联网搜索，查询词不超过128字符","inputSchema":{"type":"object","properties":{"query":{"type":"string"}},"required":["query"]}},
    {"name":"deepseek_fetch","description":"通过DeepSeek Fetch读取指定网页正文","inputSchema":{"type":"object","properties":{"url":{"type":"string"}},"required":["url"]}},
    {"name":"grade_answer","description":"对照题库参考答案和教材严格要求，为文字或手写图片评分；返回评分供核对","inputSchema":{"type":"object","properties":{"questionId":{"type":"string"},"text":{"type":"string"},"image":{"type":"string"}},"required":["questionId"]}},
    {"name":"propose_question_bank","description":"提交生成题草稿，用户在EasyGot预览确认前不会加入正式题库。JSON遵循EasyGot题库schemaVersion1","inputSchema":{"type":"object","properties":{"bank":{"type":"object"}},"required":["bank"]}},
    {"name":"save_study_plan","description":"创建新的复习计划，可在EasyGot版本记录中撤销；不覆盖已有笔记","inputSchema":{"type":"object","properties":{"title":{"type":"string"},"text":{"type":"string"}},"required":["title","text"]}}
])}
pub fn call(store:&Store,c:&Config,name:&str,args:&Value)->Result<Value>{match name{
    "search_materials"=>{let selected:HashSet<String>=args["materialIds"].as_array().map(|a|a.iter().filter_map(|v|v.as_str().map(String::from)).collect()).unwrap_or_default();materials::search(store,args["query"].as_str().unwrap_or(""),&selected)},
    "get_wrong_questions"=>Ok(json!(store.wrong()?)),
    "deepseek_search"=>network::search(c,args["query"].as_str().unwrap_or("")),
    "deepseek_fetch"=>network::fetch(c,args["url"].as_str().unwrap_or("")),
    "grade_answer"=>{let r=store.get(args["questionId"].as_str().unwrap_or(""))?.ok_or_else(||anyhow::anyhow!("题目不存在"))?;let q:Question=serde_json::from_value(r.payload)?;ai::grade(store,c,&q,args)},
    "propose_question_bank"=>{let mut bank:Bank=serde_json::from_value(args["bank"].clone())?;bank.validate()?;for q in &mut bank.questions{q.id=id();q.origin="ai".into();}let draft_id=id();store.save("plan",&draft_id,json!({"id":draft_id,"title":bank.title,"type":"bankDraft","bank":bank}),None,false,false)?;Ok(json!({"draftId":draft_id,"status":"awaiting_user_confirmation","message":"草稿已生成，请回EasyGot助教页预览，尚未导入正式题库"}))},
    "save_study_plan"=>{let title=args["title"].as_str().unwrap_or("");let text=args["text"].as_str().unwrap_or("");ensure!(!title.is_empty()&&!text.is_empty(),"计划内容不能为空");let plan_id=id();store.save("plan",&plan_id,json!({"id":plan_id,"title":title,"text":text}),None,false,false)?;Ok(json!({"saved":true,"id":plan_id,"undo":"可在EasyGot版本记录撤销"}))},_=>anyhow::bail!("未知学习工具")}}
pub fn dispatch(store:&Store,c:&Config,message:&Value)->Value{
    let request_id=message.get("id").cloned().unwrap_or(Value::Null);let result:Result<Value>=(||{match message["method"].as_str().unwrap_or(""){
        "initialize"=>Ok(json!({"protocolVersion":"2024-11-05","capabilities":{"tools":{"listChanged":false}},"serverInfo":{"name":"EasyGot Learning","version":"0.1.0"},"instructions":format!("{} 学习写入仅通过草稿确认，题型规则见search_materials结果。propose_question_bank不会直接导题。",ai::SYSTEM)})),
        "tools/list"=>Ok(json!({"tools":tools()})),"ping"=>Ok(json!({})),
        "tools/call"=>{let params=&message["params"];match call(store,c,params["name"].as_str().unwrap_or(""),&params["arguments"]){Ok(v)=>Ok(json!({"content":[{"type":"text","text":domain::clip(&v.to_string(),60000)}]})),Err(e)=>Ok(json!({"isError":true,"content":[{"type":"text","text":e.to_string()}]}))}},_=>anyhow::bail!("不支持的MCP方法")}})();
    match result{Ok(v)=>json!({"jsonrpc":"2.0","id":request_id,"result":v}),Err(e)=>json!({"jsonrpc":"2.0","id":request_id,"error":{"code":-32601,"message":e.to_string()}})}
}
pub fn start(store:Store,config:SharedConfig)->Result<()> {
    let token_path=store.root.join("learning-bridge.token");let token=if token_path.exists(){std::fs::read_to_string(&token_path)?}else{let token=id()+&id();std::fs::write(&token_path,&token)?;token};prepare_bundle(&store,&token)?;
    if STARTED.get().is_some(){return Ok(());}let server=tiny_http::Server::http(("127.0.0.1",PORT)).map_err(|e|anyhow::anyhow!(e.to_string()))?;let _=STARTED.set(());
    std::thread::spawn(move||{for mut request in server.incoming_requests(){let valid=request.url()=="/mcp"&&request.headers().iter().any(|h|h.field.equiv("Authorization")&&h.value.as_str()==format!("Bearer {token}"));if !valid{let _=request.respond(tiny_http::Response::from_string("Unauthorized").with_status_code(401));continue;}
        if request.method()!=&tiny_http::Method::Post{let _=request.respond(tiny_http::Response::empty(405));continue;}
        let mut bytes=vec![];let read=request.as_reader().take(2*1024*1024+1).read_to_end(&mut bytes);if read.is_err()||bytes.len()>2*1024*1024{let _=request.respond(tiny_http::Response::empty(413));continue;}
        let message:Value=match serde_json::from_slice(&bytes){Ok(v)=>v,Err(_)=>{let _=request.respond(tiny_http::Response::empty(400));continue;}};
        if message.get("id").is_none(){let _=request.respond(tiny_http::Response::empty(202));continue;}
        let c=match config.read(){Ok(c)=>c.clone(),Err(_)=>{let _=request.respond(tiny_http::Response::empty(503));continue;}};
        let result=dispatch(&store,&c,&message);let mut response=tiny_http::Response::from_string(result.to_string());if let Ok(header)=tiny_http::Header::from_bytes(b"Content-Type",b"application/json"){response=response.with_header(header);}let _=request.respond(response);
    }});Ok(())
}
fn prepare_bundle(store:&Store,token:&str)->Result<()> {
    let dir=store.root.join("bootstrap/rootfs/home/dsh/easygot-learning-tools");std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("package.json"),include_str!("../../harness-bundle/package.json"))?;std::fs::write(dir.join("index.js"),"export {}\n")?;
    let patch=format!("- insert:\n    - id: easygot-learning\n      name: '@deepseek-ai/dsh-mcp-client'\n      config:\n        serverName: easygot\n        transport: streamable-http\n        url: 'http://127.0.0.1:{PORT}/mcp'\n        headers:\n          Authorization: 'Bearer {token}'\n        toolCallTimeoutMs: 180000\n");std::fs::write(dir.join("cordis.patch.yml"),patch)?;Ok(())
}
#[cfg(test)]mod tests{use super::*;#[test]fn malformed_proposal_does_not_write_question(){let root=std::env::temp_dir().join(id());let s=Store::open(&root).unwrap();assert!(call(&s,&Config::default(),"propose_question_bank",&json!({"bank":{}})).is_err());assert!(s.list("question").unwrap().is_empty());let bank:Value=serde_json::from_str(include_str!("../../examples/sample-bank.json")).unwrap();call(&s,&Config::default(),"propose_question_bank",&json!({"bank":bank})).unwrap();assert!(s.list("question").unwrap().is_empty());assert_eq!(s.list("plan").unwrap().len(),1);std::fs::remove_dir_all(root).unwrap();}}
