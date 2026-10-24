//! In-process Rust agent kernel: durable turns, typed learning tools, bounded context and repeat guards.
//! Architecture reference: deepseek-harness-mobile / DeepSeek Harness (MIT); no DSH product runtime.
use anyhow::{ensure,Result};use serde_json::{Value,json};use std::{collections::HashSet,sync::{Arc,atomic::{AtomicBool,Ordering}}};
use crate::{store::{Store,id,now},network::{self,Config},domain,materials,ai};
pub fn tools()->Value{json!([
 {"name":"search_materials","description":"检索选定教材、考纲、往年试题，返回真实出处","parameters":{"type":"object","properties":{"query":{"type":"string"}},"required":["query"]}},
 {"name":"get_wrong_questions","description":"读取最近失分题和作答，分析薄弱点","parameters":{"type":"object","properties":{}}},
 {"name":"search_web","description":"使用DeepSeek联网补充知识点；外部来源必须和教材区分","parameters":{"type":"object","properties":{"query":{"type":"string"}},"required":["query"]}},
 {"name":"fetch_web","description":"读取本轮搜索返回的网页正文","parameters":{"type":"object","properties":{"url":{"type":"string"}},"required":["url"]}},
 {"name":"generate_questions","description":"按章节、知识点、题型、题量、难度、考纲权重和错题生成新题草稿，用户确认前不入正式题库","parameters":{"type":"object","properties":{"specification":{"type":"object"}},"required":["specification"]}},
 {"name":"propose_note","description":"为题目生成笔记修改草稿，由用户确认后保存","parameters":{"type":"object","properties":{"questionId":{"type":"string"},"text":{"type":"string"}},"required":["questionId","text"]}},
 {"name":"save_study_plan","description":"创建新的可撤销复习计划","parameters":{"type":"object","properties":{"title":{"type":"string"},"text":{"type":"string"}},"required":["title","text"]}}
])}
fn event(store:&Store,session_id:&str,kind:&str,value:Value)->Result<()>{let event_id=id();store.save("agent",&event_id,json!({"id":event_id,"sessionId":session_id,"event":kind,"value":value,"time":now()}),None,false,false)?;Ok(())}
pub fn run(store:&Store,c:&Config,session_id:&str,query:&str,selected:&HashSet<String>,web:bool,cancel:&Arc<AtomicBool>)->Result<String>{
 ensure!(!query.trim().is_empty(),"请先输入问题");let mut messages=vec![json!({"role":"system","content":format!("{} 你是软件内部Agent，可多步检索、分析和调用学习工具。不要提及DSH、插件安装或容器。先查教材再回答，无法查证时明确说明。生成题和修改笔记必须通过草稿确认。",ai::SYSTEM)})];
 let mut history=store.list("agent")?;history.sort_by_key(|r|r.modified_at);let recent:Vec<_>=history.into_iter().filter(|r|r.payload["sessionId"]==session_id&&["user","assistant"].contains(&r.payload["event"].as_str().unwrap_or(""))).collect();
 // Keep complete durable history on disk, and bounded verbatim recent turns in model context.
 for r in recent.iter().rev().take(12).collect::<Vec<_>>().into_iter().rev(){messages.push(json!({"role":r.payload["event"],"content":domain::clip(r.payload["value"].as_str().unwrap_or(""),5000)}));}
 messages.push(json!({"role":"user","content":query}));event(store,session_id,"user",json!(query))?;
 let tools:Vec<Value>=tools().as_array().unwrap().iter().filter(|t|web||!["search_web","fetch_web"].contains(&t["name"].as_str().unwrap_or(""))).map(|t|json!({"type":"function","function":t})).collect();
 let mut used=std::collections::HashMap::<String,u32>::new();let mut urls=HashSet::new();
 for step in 0..24{ensure!(!cancel.load(Ordering::Relaxed),"任务已停止，已有作答和草稿保留");event(store,session_id,"step",json!(step))?;
  let base=c.ai_base.trim_end_matches('/');let endpoint=if base.ends_with("/chat/completions"){base.to_owned()}else{format!("{base}/chat/completions")};
  let response=network::post(&endpoint,&c.ai_key,&json!({"model":c.model,"messages":messages,"tools":tools,"tool_choice":"auto","stream":false,"max_tokens":4096}))?;
  ensure!(!cancel.load(Ordering::Relaxed),"任务已停止，历史已保留");let choice=&response["choices"][0];ensure!(choice["finish_reason"]!="length","输出被截断，请缩小任务");let message=choice["message"].clone();let calls=message["tool_calls"].as_array().cloned().unwrap_or_default();
  if calls.is_empty(){let text=message["content"].as_str().ok_or_else(||anyhow::anyhow!("模型没有返回有效文本"))?.to_owned();event(store,session_id,"assistant",json!(text))?;return Ok(text);}
  ensure!(calls.len()<=8,"单步工具调用过多");messages.push(message);
  for call in calls{ensure!(!cancel.load(Ordering::Relaxed),"任务已停止");let name=call["function"]["name"].as_str().unwrap_or("");let arguments:Value=serde_json::from_str(call["function"]["arguments"].as_str().unwrap_or("{}"))?;let signature=format!("{name}:{arguments}");let repetitions=used.entry(signature).or_default();*repetitions+=1;
   event(store,session_id,"tool_start",json!({"name":name,"arguments":arguments}))?;
   let result=(||->Result<Value>{ensure!(*repetitions<=2,"重复相同工具调用，请更换检索方式或总结结果");match name{
    "search_materials"=>materials::search(store,arguments["query"].as_str().unwrap_or(""),selected),
    "get_wrong_questions"=>Ok(json!(store.wrong()?)),
    "search_web"=>{ensure!(web,"本轮未允许联网");let found=network::search(c,arguments["query"].as_str().unwrap_or(""))?;if let Some(results)=found.as_array(){for r in results{if let Some(url)=r["url"].as_str(){urls.insert(url.to_owned());}}}Ok(found)},
    "fetch_web"=>{let url=arguments["url"].as_str().unwrap_or("");ensure!(web&&urls.contains(url),"只能读取本轮搜索返回的网页");network::fetch(c,url)},
    "generate_questions"=>{let bank=ai::generate(store,c,arguments["specification"].clone(),selected,true)?;let draft_id=id();store.save("plan",&draft_id,json!({"id":draft_id,"type":"bankDraft","title":bank.title,"bank":bank}),None,false,false)?;Ok(json!({"draftId":draft_id,"status":"awaiting_confirmation","message":"已生成新题草稿，在助教页确认后才入库"}))},
    "propose_note"=>{let qid=arguments["questionId"].as_str().unwrap_or("");ensure!(store.get(qid)?.is_some(),"题目不存在");let draft_id=id();store.save("plan",&draft_id,json!({"id":draft_id,"type":"noteDraft","questionId":qid,"title":"笔记修改草稿","text":arguments["text"]}),None,false,false)?;Ok(json!({"draftId":draft_id,"status":"awaiting_confirmation"}))},
    "save_study_plan"=>{let plan_id=id();store.save("plan",&plan_id,json!({"id":plan_id,"title":arguments["title"],"text":arguments["text"]}),None,false,false)?;Ok(json!({"id":plan_id,"saved":true,"undo":"版本记录可撤销"}))},_=>anyhow::bail!("未知工具，未执行")}})().unwrap_or_else(|e|json!({"error":e.to_string()}));
   event(store,session_id,"tool_result",result.clone())?;messages.push(json!({"role":"tool","tool_call_id":call["id"],"content":domain::clip(&result.to_string(),14000)}));
  }
 }anyhow::bail!("已达到本轮24步上限，已保留历史和草稿，请拆分任务继续")
}
#[cfg(test)]mod tests{use super::*;#[test]fn tools_are_learning_scoped(){let definitions=tools();let names:Vec<_>=definitions.as_array().unwrap().iter().filter_map(|v|v["name"].as_str()).collect();assert!(names.contains(&"generate_questions"));assert!(!names.contains(&"bash"));assert!(!names.contains(&"write_file"));}}
