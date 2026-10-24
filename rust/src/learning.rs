//! Persistent question-bank/reference links and paper practice records.
use anyhow::{ensure,Result};
use serde::{Deserialize,Serialize};
use serde_json::{json,Value};
use std::collections::HashSet;
use crate::{domain::{Bank,Question,DirectoryEntry},store::{Store,id,now}};

#[derive(Debug,Clone,Serialize,Deserialize)]
#[serde(rename_all="camelCase")]
pub struct BankGroup {pub id:String,pub title:String,pub question_ids:Vec<String>,#[serde(default)]pub material_ids:Vec<String>,#[serde(default)]pub directory:Vec<DirectoryEntry>}
pub fn banks(store:&Store)->Result<Vec<BankGroup>> {store.list("bank")?.into_iter().map(|r|Ok(serde_json::from_value(r.payload)?)).collect()}
pub fn migrate_legacy(store:&Store)->Result<()> {if banks(store)?.is_empty(){let questions=store.list("question")?;if !questions.is_empty(){let group=BankGroup{id:"legacy-imported-bank".into(),title:"历史导入题库".into(),question_ids:questions.into_iter().map(|r|r.entity_id).collect(),material_ids:vec![],directory:vec![]};store.put_many(vec![("bank".into(),serde_json::to_value(group)?)])?;}}Ok(())}
pub fn effective_question(store:&Store,q:&Question)->Result<Question>{let mut q=q.clone();let mut ids:HashSet<String>=q.material_ids.iter().cloned().collect();for b in banks(store)?{if b.question_ids.contains(&q.id){ids.extend(b.material_ids);}}q.material_ids=ids.into_iter().collect();q.material_ids.sort();Ok(q)}
pub fn bank_questions(store:&Store,bank:&BankGroup,directory:&str)->Result<Vec<Question>>{
 let ids=if directory.is_empty(){bank.question_ids.clone()}else{DirectoryEntry::find(&bank.directory,directory).ok_or_else(||anyhow::anyhow!("套卷目录不存在"))?.ids()};
 let mut questions=vec![];for id in ids{if let Some(record)=store.get(&id)?{if !record.deleted&&record.kind=="question"{questions.push(serde_json::from_value(record.payload)?);}}}Ok(questions)
}
pub fn export_bank(store:&Store,bank:&BankGroup,directory:&str,questions:Vec<Question>,filtered:bool)->Bank{
 let allowed=questions.iter().map(|q|q.id.clone()).collect();let entries=if directory.is_empty(){bank.directory.clone()}else{DirectoryEntry::find(&bank.directory,directory).cloned().into_iter().collect()};
 let directory_entries=entries.iter().filter_map(|e|e.pruned(&allowed)).collect();
 let title=if let Some(entry)=DirectoryEntry::find(&bank.directory,directory){format!("{} · {}",bank.title,entry.title)}else{bank.title.clone()};
 let title=if filtered{format!("{title} · 筛选题目")}else{title};let id=if filtered{String::new()}else if directory.is_empty(){bank.id.clone()}else{format!("{}:{}",bank.id,directory)};
 let _=store;Bank{id,title,schema_version:1,material_ids:bank.material_ids.clone(),questions,directory:directory_entries,assets:Default::default()}
}
pub fn import_bank(store:&Store,bank:&Bank)->Result<String>{bank.validate()?;let bank=bank.normalized();let existing=banks(store)?.into_iter().find(|b|(!bank.id.is_empty()&&b.id==bank.id)||(bank.id.is_empty()&&b.title==bank.title));let group=BankGroup{id:existing.as_ref().map(|b|b.id.clone()).unwrap_or_else(||if bank.id.is_empty(){id()}else{bank.id.clone()}),title:bank.title.clone(),directory:bank.directory.clone(),question_ids:bank.questions.iter().map(|q|q.id.clone()).collect(),material_ids:if bank.material_ids.is_empty(){existing.map(|b|b.material_ids).unwrap_or_default()}else{bank.material_ids.clone()}};let mut writes=vec![];for q in &bank.questions{writes.push(("question".to_owned(),serde_json::to_value(q)?));}writes.push(("bank".into(),serde_json::to_value(&group)?));store.put_many(writes)?;Ok(group.id)}
pub fn link_bank(store:&Store,bank_id:&str,materials:Vec<String>)->Result<()> {let r=store.get(bank_id)?.ok_or_else(||anyhow::anyhow!("题库不存在"))?;ensure!(r.kind=="bank","记录不是题库");for material in &materials{let m=store.get(material)?.ok_or_else(||anyhow::anyhow!("关联资料不存在"))?;ensure!(m.kind=="material"&&!m.deleted,"资料已删除或类型不符");}let mut bank:BankGroup=serde_json::from_value(r.payload)?;bank.material_ids=materials;store.save("bank",bank_id,serde_json::to_value(bank)?,Some(&r.version),false,false)?;Ok(())}

/// Create a question and its bank/paper membership together. Editing an existing
/// question uses Store::save with its base revision instead of this operation.
pub fn add_question(store:&Store,question:&Question,bank_id:&str,directory:&str)->Result<String>{
 question.validate()?;ensure!(store.get(&question.id)?.is_none(),"此题id已经存在，请使用编辑入口");
 let target=if bank_id.is_empty(){"personal-question-bank"}else{bank_id};
 let mut bank=if let Some(record)=store.get(target)?{ensure!(record.kind=="bank"&&!record.deleted,"题库不可用");serde_json::from_value::<BankGroup>(record.payload)?}else{ensure!(bank_id.is_empty(),"题库不存在");BankGroup{id:target.into(),title:"我的题库".into(),question_ids:vec![],material_ids:vec![],directory:vec![]}};
 if !directory.is_empty(){
  fn attach(entries:&mut [DirectoryEntry],target:&str,question:&str)->bool{for entry in entries{if entry.id==target&&entry.children.is_empty(){entry.question_ids.push(question.into());return true;}if attach(&mut entry.children,target,question){return true;}}false}
  ensure!(attach(&mut bank.directory,directory,&question.id),"当前套卷不存在，请重新选择");
 }
 bank.question_ids.push(question.id.clone());
 store.put_many(vec![("question".into(),serde_json::to_value(question)?),("bank".into(),serde_json::to_value(&bank)?)])?;Ok(bank.id)
}

#[derive(Debug,Clone,Serialize,Deserialize)]
#[serde(rename_all="camelCase")]
pub struct PaperQuestion{pub number:u32,pub question:Question}
#[derive(Debug,Clone,Serialize,Deserialize)]
#[serde(rename_all="camelCase")]
pub struct Paper{pub id:String,pub title:String,pub created_at:u64,pub questions:Vec<PaperQuestion>,#[serde(default)]pub started_at:Option<u64>,#[serde(default)]pub stopped_at:Option<u64>,#[serde(default)]pub elapsed_ms:u64,#[serde(default)]pub pages:Vec<String>,#[serde(default)]pub submissions:Vec<String>}
impl Paper {
 pub fn new(title:&str,questions:Vec<Question>)->Result<Self>{ensure!(!questions.is_empty(),"请先选择试题");ensure!(questions.len()<=200,"一次最多导出200道题");Ok(Self{id:id(),title:title.into(),created_at:now(),questions:questions.into_iter().enumerate().map(|(i,q)|PaperQuestion{number:(i+1)as u32,question:q}).collect(),started_at:None,stopped_at:None,elapsed_ms:0,pages:vec![],submissions:vec![]})}
 pub fn elapsed(&self,time:u64)->u64{self.elapsed_ms+self.started_at.map_or(0,|s|time.saturating_sub(s))}
 pub fn start(&mut self,time:u64){if self.started_at.is_none(){self.started_at=Some(time);self.stopped_at=None;}}
 pub fn stop(&mut self,time:u64){self.elapsed_ms=self.elapsed(time);self.started_at=None;self.stopped_at=Some(time);}
 pub fn print_spec(&self,answers:bool)->Value{let questions:Vec<_>=self.questions.iter().map(|entry|{let q=&entry.question;let mut text=format!("{}. 【{} · {}分】{}",entry.number,crate::domain::label(&q.kind),q.max_score,q.stem);for option in &q.options{text.push_str(&format!("\n{}. {}",option.id,option.text));}if answers{text.push_str(&format!("\n参考答案：{}\n解析：{}",if q.subjective(){q.reference_answer.clone()}else{q.answer.to_string()},q.explanation));}json!({"number":entry.number,"id":q.id,"text":text,"image":q.image,"answerLines":if answers{0}else if q.subjective(){8}else{2}})}).collect();json!({"paperId":self.id,"title":self.title,"answers":answers,"questions":questions})}
}
#[derive(Debug,Clone,Serialize,Deserialize)]
#[serde(rename_all="camelCase")]
pub struct RecognizedAnswer{pub number:u32,#[serde(default)]pub text:String,#[serde(default)]pub selected:Vec<String>,#[serde(default)]pub verdict:Option<bool>,#[serde(default)]pub confidence:f64,#[serde(default)]pub uncertain:bool}
pub fn check_answers(paper:&Paper,answers:&[RecognizedAnswer])->Result<()> {let mut seen=HashSet::new();for answer in answers{ensure!(seen.insert(answer.number),"同一题号重复，请先核对合并");let question=paper.questions.iter().find(|q|q.number==answer.number).ok_or_else(||anyhow::anyhow!("照片中出现不属于本卷的题号{}",answer.number))?;ensure!(answer.confidence.is_finite()&&(0.0..=1.0).contains(&answer.confidence),"识别置信度非法");let mut selected=HashSet::new();for option in &answer.selected{ensure!(question.question.options.iter().any(|o|o.id==*option)&&selected.insert(option),"识别选项无效或重复");}}Ok(())}
pub fn save_paper(store:&Store,paper:&Paper)->Result<()> {store.save("paper",&paper.id,serde_json::to_value(paper)?,None,false,false)?;Ok(())}
pub fn record_paper_answers(store:&Store,paper:&mut Paper,answers:&[RecognizedAnswer])->Result<usize>{check_answers(paper,answers)?;let submission=id();let mut writes=vec![];for entry in &paper.questions{let recognized=answers.iter().find(|a|a.number==entry.number);let Some(answer)=recognized else{continue;};ensure!(!answer.uncertain,"第{}题尚未确认识别结果",entry.number);let q=&entry.question;let pending=q.subjective()&&!answer.text.trim().is_empty();let value=if q.kind=="boolean"{json!(answer.verdict)}else{json!(answer.selected)};let attempt_id=format!("{}:{}:{}",paper.id,submission,q.id);writes.push(("attempt".into(),json!({"id":attempt_id,"questionId":q.id,"question":q,"title":crate::domain::clip(&q.stem,100),"answer":{"text":answer.text,"selected":answer.selected,"verdict":answer.verdict},"score":if q.subjective(){0.0}else{q.score(&value)?},"maxScore":q.max_score,"pending":pending,"paperId":paper.id,"paperNumber":entry.number,"submissionId":submission,"elapsedMs":paper.elapsed_ms,"answeredAt":now(),"source":"paper","explanation":q.explanation,"feedback":if pending{"待AI批改"}else{"按已核对的纸上作答判分"}})));}paper.submissions.push(submission);writes.push(("paper".into(),serde_json::to_value(&paper)?));let count=writes.len()-1;store.put_many(writes)?;Ok(count)}
#[cfg(test)]mod tests{
 use super::*;fn questions()->Vec<Question>{serde_json::from_str::<Bank>(include_str!("../../examples/sample-bank.json")).unwrap().questions}
 #[test]fn new_question_is_added_to_the_selected_paper_atomically(){
  let root=std::env::temp_dir().join(id());let store=Store::open(&root).unwrap();let bank:Bank=serde_json::from_str(include_str!("../../examples/multi-paper-bank.json")).unwrap();let bank_id=import_bank(&store,&bank).unwrap();let mut q=questions().remove(0);q.id="new-paper-ten-question".into();add_question(&store,&q,&bank_id,"paper-10").unwrap();let bank=banks(&store).unwrap().remove(0);let selected=bank_questions(&store,&bank,"paper-10").unwrap();assert_eq!(selected.len(),3);assert_eq!(selected[2].id,q.id);
  let mut bad=q.clone();bad.id="not-inserted".into();assert!(add_question(&store,&bad,&bank_id,"missing-paper").is_err());assert!(store.get(&bad.id).unwrap().is_none());std::fs::remove_dir_all(root).unwrap();
 }
 #[test]fn ten_papers_import_in_order_and_round_trip(){let root=std::env::temp_dir().join(id());let store=Store::open(&root).unwrap();let bank:Bank=serde_json::from_str(include_str!("../../examples/multi-paper-bank.json")).unwrap();bank.validate().unwrap();assert_eq!(bank.all_questions().len(),20);let bank_id=import_bank(&store,&bank).unwrap();let group=banks(&store).unwrap().into_iter().find(|b|b.id==bank_id).unwrap();assert_eq!(group.directory.len(),2);for n in 1..=10{let directory=format!("paper-{n:02}");let questions=bank_questions(&store,&group,&directory).unwrap();assert_eq!(questions.len(),2);assert_eq!(questions[0].id,format!("paper-{n:02}-q1"));let export=export_bank(&store,&group,&directory,questions,false);export.validate().unwrap();assert_eq!(export.directory.len(),1);assert_eq!(export.normalized().all_questions().len(),2);}let export=export_bank(&store,&group,"",bank_questions(&store,&group,"").unwrap(),false);export.validate().unwrap();assert_eq!(export.directory[0].children.len(),5);assert!(export.directory[0].children[0].questions.is_empty());import_bank(&store,&export).unwrap();assert_eq!(store.list("question").unwrap().len(),20);std::fs::remove_dir_all(root).unwrap();}
 #[test]fn invalid_directory_references_and_duplicate_nodes_are_rejected(){let mut bank:Bank=serde_json::from_str(include_str!("../../examples/multi-paper-bank.json")).unwrap();bank.directory[0].children[0].question_ids.push("missing".into());assert!(bank.validate().is_err());bank.directory[0].children[0].question_ids.clear();bank.directory[1].id=bank.directory[0].id.clone();assert!(bank.validate().is_err());}
 #[test]fn print_hides_answers_and_stable_numbers(){let paper=Paper::new("试卷",questions()).unwrap();let spec=paper.print_spec(false);assert_eq!(spec["questions"][0]["number"],1);assert!(!spec.to_string().contains("referenceAnswer"));assert!(!spec["questions"][0]["text"].as_str().unwrap().contains("参考答案"));assert!(paper.print_spec(true)["questions"][0]["text"].as_str().unwrap().contains("参考答案"));}
 #[test]fn paper_timer_survives_pause(){let mut paper=Paper::new("p",questions()).unwrap();paper.start(1000);paper.stop(7000);paper.start(10000);paper.stop(14000);assert_eq!(paper.elapsed_ms,10000);}
 #[test]fn unmatched_or_duplicate_answers_rejected(){let paper=Paper::new("p",questions()).unwrap();let a=RecognizedAnswer{number:99,text:String::new(),selected:vec![],verdict:None,confidence:1.,uncertain:false};assert!(check_answers(&paper,&[a]).is_err());}
 #[test]fn bank_links_affect_scoring_context(){let root=std::env::temp_dir().join(id());let store=Store::open(&root).unwrap();let bank:Bank=serde_json::from_str(include_str!("../../examples/sample-bank.json")).unwrap();let bank_id=import_bank(&store,&bank).unwrap();store.save("material","m",json!({"id":"m","title":"教材"}),None,false,false).unwrap();link_bank(&store,&bank_id,vec!["m".into()]).unwrap();assert_eq!(effective_question(&store,&bank.questions[0]).unwrap().material_ids,vec!["m"]);link_bank(&store,&bank_id,vec![]).unwrap();assert!(effective_question(&store,&bank.questions[0]).unwrap().material_ids.is_empty());std::fs::remove_dir_all(root).unwrap();}
 #[test]fn paper_answers_save_grades_and_reject_uncertain(){let root=std::env::temp_dir().join(id());let store=Store::open(&root).unwrap();let mut paper=Paper::new("纸卷",questions()).unwrap();paper.start(1000);paper.stop(7000);let a=RecognizedAnswer{number:1,text:String::new(),selected:vec!["B".into()],verdict:None,confidence:0.6,uncertain:true};assert!(record_paper_answers(&store,&mut paper,&[a.clone()]).is_err());assert!(store.list("attempt").unwrap().is_empty());let mut a=a;a.uncertain=false;a.selected=paper.questions[0].question.answer.as_array().unwrap().iter().map(|v|v.as_str().unwrap().into()).collect();assert_eq!(record_paper_answers(&store,&mut paper,&[a]).unwrap(),1);let attempts=store.list("attempt").unwrap();assert_eq!(attempts[0].payload["score"].as_f64().unwrap(),paper.questions[0].question.max_score);assert_eq!(attempts[0].payload["elapsedMs"],6000);assert_eq!(attempts[0].payload["pending"],false);std::fs::remove_dir_all(root).unwrap();}

}
