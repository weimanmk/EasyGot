use anyhow::{bail,ensure,Result};
use serde::{Deserialize,Serialize};
use serde_json::{Value,json};
use std::collections::{HashSet,BTreeMap};

pub const TYPES:[&str;6]=["single","multiple","boolean","short","essay","calculation"];
#[derive(Debug,Clone,Serialize,Deserialize,Default)]
#[serde(rename_all="camelCase")]
pub struct Source {#[serde(default)]pub material_id:String,#[serde(default)]pub url:String,pub title:String,pub locator:String}
#[derive(Debug,Clone,Serialize,Deserialize)]
#[serde(rename_all="camelCase")]
pub struct Question {
    pub id:String,#[serde(rename="type")]pub kind:String,pub stem:String,pub max_score:f64,
    #[serde(default)]pub options:Vec<OptionItem>,#[serde(default)]pub answer:Value,
    #[serde(default)]pub reference_answer:String,#[serde(default)]pub explanation:String,
    #[serde(default)]pub chapter:String,#[serde(default)]pub knowledge_points:Vec<String>,
    #[serde(default)]pub difficulty:u8,#[serde(default)]pub strict_requirements:String,
    #[serde(default)]pub material_ids:Vec<String>,#[serde(default)]pub sources:Vec<Source>,
    #[serde(default)]pub origin:String,#[serde(default,skip_serializing_if="String::is_empty")]pub image:String,
    #[serde(flatten)]pub extra:BTreeMap<String,Value>,
}
#[derive(Debug,Clone,Serialize,Deserialize)]pub struct OptionItem{pub id:String,pub text:String}
#[derive(Debug,Clone,Serialize,Deserialize)]
#[serde(rename_all="camelCase")]
pub struct Bank{#[serde(default)]pub id:String,#[serde(default)]pub material_ids:Vec<String>,pub schema_version:u32,pub title:String,#[serde(default)]pub questions:Vec<Question>,#[serde(default)]pub directory:Vec<DirectoryEntry>,#[serde(default)]pub assets:BTreeMap<String,String>}
/// A folder has children; a paper has inline questions and/or references to the bank pool.
#[derive(Debug,Clone,Serialize,Deserialize,Default)]
#[serde(rename_all="camelCase")]
pub struct DirectoryEntry {
    pub id:String,pub title:String,
    #[serde(default,skip_serializing_if="Vec::is_empty")]pub questions:Vec<Question>,
    #[serde(default,skip_serializing_if="Vec::is_empty")]pub question_ids:Vec<String>,
    #[serde(default,skip_serializing_if="Vec::is_empty")]pub children:Vec<DirectoryEntry>,
    #[serde(default,skip_serializing_if="Option::is_none")]pub time_limit_minutes:Option<u32>,
}
impl DirectoryEntry {
    pub fn find<'a>(entries:&'a [Self],id:&str)->Option<&'a Self>{for entry in entries{if entry.id==id{return Some(entry);}if let Some(found)=Self::find(&entry.children,id){return Some(found);}}None}
    pub fn ids(&self)->Vec<String>{let mut ids=self.question_ids.clone();ids.extend(self.questions.iter().map(|q|q.id.clone()));for child in &self.children{ids.extend(child.ids());}let mut seen=HashSet::new();ids.retain(|id|seen.insert(id.clone()));ids}
    fn collect_questions(&self,all:&mut Vec<Question>){all.extend(self.questions.clone());for child in &self.children{child.collect_questions(all);}}
    fn normalize(&mut self){self.question_ids.extend(self.questions.drain(..).map(|q|q.id));for child in &mut self.children{child.normalize();}}
    pub fn pruned(&self,allowed:&HashSet<String>)->Option<Self>{let mut entry=self.clone();entry.questions.clear();entry.question_ids.retain(|id|allowed.contains(id));entry.children=self.children.iter().filter_map(|child|child.pruned(allowed)).collect();if entry.question_ids.is_empty()&&entry.children.is_empty(){None}else{Some(entry)}}
    fn validate(&self,known:&HashSet<String>,nodes:&mut HashSet<String>,depth:usize)->Result<()> {
        ensure!(depth<=8&&nodes.len()<1000,"题库目录最多8层、1000项");
        ensure!(!self.id.trim().is_empty()&&!self.title.trim().is_empty()&&nodes.insert(self.id.clone()),"目录id/标题为空或目录id重复");
        ensure!(self.time_limit_minutes.is_none_or(|n|(1..=360).contains(&n)),"套卷时长须为1–360分钟");
        ensure!(self.children.is_empty()||(self.questions.is_empty()&&self.question_ids.is_empty()),"文件夹只能包含子目录，不能同时包含题目");
        ensure!(!self.children.is_empty()||!self.questions.is_empty()||!self.question_ids.is_empty(),"套卷不能为空");
        let mut seen=HashSet::new();for id in self.question_ids.iter().chain(self.questions.iter().map(|q|&q.id)){ensure!(known.contains(id)&&seen.insert(id),"套卷引用不存在或重复的题目");}
        for child in &self.children{child.validate(known,nodes,depth+1)?;}Ok(())
    }
}
impl Question {
    pub fn subjective(&self)->bool{matches!(self.kind.as_str(),"short"|"essay"|"calculation")}
    pub fn validate(&self)->Result<()> {
        ensure!(!self.id.trim().is_empty()&&!self.stem.trim().is_empty(),"id和题干不能为空");
        ensure!(TYPES.contains(&self.kind.as_str()),"未知题型");
        ensure!(self.max_score.is_finite()&&self.max_score>0.&&self.max_score<=10000.,"分值超出范围");
        if self.kind=="single"||self.kind=="multiple" {
            ensure!((2..=12).contains(&self.options.len()),"选择题需要2–12个选项");let mut keys=HashSet::new();
            for o in &self.options{ensure!(!o.id.is_empty()&&!o.text.trim().is_empty()&&keys.insert(&o.id),"选项空白或重复");}
            let answers=self.answer.as_array().ok_or_else(||anyhow::anyhow!("选择题答案应为数组"))?;
            ensure!(!answers.is_empty()&&(self.kind!="single"||answers.len()==1),"答案数量不正确");let mut seen=HashSet::new();
            for answer in answers{let a=answer.as_str().ok_or_else(||anyhow::anyhow!("答案必须为选项id"))?;ensure!(keys.iter().any(|k|k.as_str()==a)&&seen.insert(a),"答案无效或重复");}
        }else if self.kind=="boolean"{ensure!(self.answer.is_boolean(),"判断题答案必须为布尔值");}
        else{ensure!(!self.reference_answer.trim().is_empty(),"主观题必须有参考答案");}
        for s in &self.sources{ensure!(!s.title.is_empty()&&!s.locator.is_empty()&&(!s.material_id.is_empty()||!s.url.is_empty()),"引用信息不完整");}
        Ok(())
    }
    pub fn score(&self,answer:&Value)->Result<f64>{
        ensure!(!self.subjective(),"主观题需要评分模型");let correct=if self.kind=="boolean"{answer.is_boolean()&&answer==&self.answer}else if let Some(actual)=answer.as_array(){let expected=self.answer.as_array().ok_or_else(||anyhow::anyhow!("标准答案错误"))?;actual.len()==expected.len()&&actual.iter().all(|x|expected.contains(x))&&actual.iter().collect::<HashSet<_>>().len()==actual.len()}else{false};Ok(if correct{self.max_score}else{0.})
    }
}
impl Bank{
    pub fn all_questions(&self)->Vec<Question>{let mut all=self.questions.clone();for entry in &self.directory{entry.collect_questions(&mut all);}all}
    pub fn normalized(&self)->Self{let mut bank=self.clone();bank.questions=self.all_questions();for entry in &mut bank.directory{entry.normalize();}bank}
    pub fn validate(&self)->Result<()>{ensure!(self.schema_version==1&&!self.title.trim().is_empty(),"题库版本或名称无效");let questions=self.all_questions();ensure!(questions.len()<=20000,"最多导入20000题");let mut ids=HashSet::new();for q in &questions{q.validate()?;ensure!(ids.insert(q.id.clone()),"题目id重复");}let mut nodes=HashSet::new();for entry in &self.directory{entry.validate(&ids,&mut nodes,1)?;}Ok(())}
}
pub fn grade_valid(grade:&Value,max:f64)->Result<()> {let score=grade["score"].as_f64().ok_or_else(||anyhow::anyhow!("AI分数不是数字"))?;ensure!(score.is_finite()&&(0.0..=max).contains(&score),"AI分数越界，未保存");for k in ["feedback","explanation"]{ensure!(grade[k].as_str().is_some_and(|s|!s.trim().is_empty()),"AI缺少反馈或解析");}Ok(())}
pub fn parse_json(text:&str)->Result<Value>{let s=text.trim();let s=if s.starts_with("```"){let begin=s.find('\n').ok_or_else(||anyhow::anyhow!("JSON输出不完整"))?;let end=s.rfind("```").unwrap_or(s.len());ensure!(end>begin,"JSON输出不完整");&s[begin+1..end]}else{s};Ok(serde_json::from_str(s)?)}
pub fn clip(s:&str,max:usize)->String{s.chars().take(max).collect()}
pub fn label(t:&str)->&str{match t{"single"=>"单选","multiple"=>"多选","boolean"=>"判断","short"=>"简答","essay"=>"论述","calculation"=>"计算",_=>t}}
pub fn blueprint(spec:&mut Value)->Result<()> {
    let count=spec["count"].as_u64().unwrap_or(0);ensure!((1..=15).contains(&count),"题量应为1–15");
    ensure!((1..=5).contains(&spec["difficulty"].as_u64().unwrap_or(0)),"难度应为1–5");
    let types=spec["typeCounts"].as_object().ok_or_else(||anyhow::anyhow!("缺少题型配比"))?;let mut sum=0;
    for (t,n) in types{ensure!(TYPES.contains(&t.as_str()),"未知题型");sum+=n.as_u64().ok_or_else(||anyhow::anyhow!("题数必须为非负整数"))?;}
    ensure!(sum==count,"题型题数总和须等于题量");
    let weights=spec["weights"].as_object().ok_or_else(||anyhow::anyhow!("权重应为JSON对象"))?;let mut total=0.;
    for (k,w) in weights{let n=w.as_f64().unwrap_or(0.);ensure!(!k.is_empty()&&n.is_finite()&&n>0.,"权重必须为正数");total+=n;}
    ensure!(total.is_finite(),"权重总和过大");let mut alloc=BTreeMap::new();let mut remainders=vec![];let mut assigned=0;
    for (k,w) in weights{let exact=count as f64*w.as_f64().unwrap()/total;let n=exact.floor()as u64;alloc.insert(k.clone(),n);assigned+=n;remainders.push((k.clone(),exact-n as f64));}
    if !remainders.is_empty(){remainders.sort_by(|a,b|b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));for i in 0..(count-assigned)as usize{*alloc.get_mut(&remainders[i].0).unwrap()+=1;}}
    spec["knowledgePointCounts"]=json!(alloc);Ok(())
}
pub fn verify_blueprint(spec:&Value,questions:&[Question])->Result<()> {
    ensure!(questions.len()==spec["count"].as_u64().unwrap_or(0)as usize,"AI生成题量不符合要求");let mut types=BTreeMap::<String,u64>::new();let mut points=BTreeMap::<String,u64>::new();
    for q in questions{ensure!(q.difficulty==spec["difficulty"].as_u64().unwrap_or(0)as u8,"AI题目难度不符合要求");if let Some(ch)=spec["chapter"].as_str().filter(|s|!s.is_empty()){ensure!(q.chapter==ch,"AI题目章节不符合要求");}*types.entry(q.kind.clone()).or_default()+=1;if let Some(p)=q.knowledge_points.first(){*points.entry(p.clone()).or_default()+=1;}}
    for t in TYPES{ensure!(types.get(t).copied().unwrap_or(0)==spec["typeCounts"][t].as_u64().unwrap_or(0),"AI题型配比不符合要求");}
    if let Some(targets)=spec["knowledgePointCounts"].as_object(){for(k,n)in targets{ensure!(points.get(k).copied().unwrap_or(0)==n.as_u64().unwrap_or(0),"AI知识点配比不符合考纲权重");}}Ok(())
}
#[cfg(test)]mod tests{use super::*;#[test]fn imported_bank_and_scoring(){let bank:Bank=serde_json::from_str(include_str!("../../examples/sample-bank.json")).unwrap();bank.validate().unwrap();let q=&bank.questions[1];assert_eq!(q.score(&json!(["C","B","A"])).unwrap(),3.);assert_eq!(q.score(&json!(["A","A","C"])).unwrap(),0.);assert_eq!(bank.questions[2].score(&json!("false")).unwrap(),0.);}#[test]fn score_limits(){assert!(grade_valid(&json!({"score":11,"feedback":"x","explanation":"y"}),10.).is_err());assert!(grade_valid(&json!({"score":4.5,"feedback":"x","explanation":"y"}),10.).is_ok());}#[test]fn weights(){let mut s=json!({"count":5,"difficulty":3,"typeCounts":{"single":5},"weights":{"甲":3,"乙":2}});blueprint(&mut s).unwrap();assert_eq!(s["knowledgePointCounts"]["甲"],3);}}
