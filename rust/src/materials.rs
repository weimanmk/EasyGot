use anyhow::{ensure,Result};use serde_json::{json,Value};use std::{io::{Read,Cursor},collections::HashSet};use crate::{store::{Store,id,now},domain::clip};
pub fn import(store:&Store,name:&str,data:&[u8],role:&str,strict:&str)->Result<Value>{
    ensure!(data.len()<=30*1024*1024,"文件超过30MB");let asset=store.asset(data)?;let lower=name.to_lowercase();let mut sections=vec![];let mut images=vec![];
    if lower.ends_with(".pdf"){
        let pages=pdf_extract::extract_text_from_mem_by_pages(data)?;ensure!(pages.len()<=500,"PDF最多500页，请拆分");
        for(i,text)in pages.into_iter().enumerate(){sections.push(json!({"locator":format!("第 {} 页",i+1),"text":text}));}
    }else if lower.ends_with(".docx"){
        let mut archive=zip::ZipArchive::new(Cursor::new(data))?;let mut xml=String::new();archive.by_name("word/document.xml")?.take(20*1024*1024).read_to_string(&mut xml)?;
        let mut reader=quick_xml::Reader::from_str(&xml);let mut p=String::new();let mut in_text=false;let mut n=0;
        loop{use quick_xml::events::Event;match reader.read_event()?{Event::Start(e)=>{if e.local_name().as_ref()==b"t"{in_text=true;}},Event::Text(t)if in_text=>p.push_str(&t.unescape()?),Event::Empty(e)=>if e.local_name().as_ref()==b"tab"{p.push('\t');}else if e.local_name().as_ref()==b"br"{p.push('\n');},Event::End(e)=>{if e.local_name().as_ref()==b"t"{in_text=false;}if e.local_name().as_ref()==b"p"{n+=1;sections.push(json!({"locator":format!("第 {n} 段"),"text":p}));p.clear();}},Event::Eof=>break,_=>{}}}
        let mut total=0;for i in 0..archive.len(){let mut file=archive.by_index(i)?;if !file.name().starts_with("word/media/"){continue;}ensure!(file.size()<=25*1024*1024,"Word图片过大");let mut bytes=vec![];file.by_ref().take(25*1024*1024+1).read_to_end(&mut bytes)?;total+=bytes.len();ensure!(total<=60*1024*1024,"Word内嵌图片总量超过60MB");if image::guess_format(&bytes).is_ok(){images.push(json!({"asset":store.asset(&bytes)?,"locator":format!("内嵌图片 {}",file.name()),"recognized":false}));}else{sections.push(json!({"locator":"未支持图片","text":format!("{} 暂不支持，需转成PNG/JPEG后识别",file.name())}));}}
    }else if lower.ends_with(".txt")||lower.ends_with(".md"){
        let text=String::from_utf8(data.to_vec())?;for(i,paragraph)in text.split("\n\n").enumerate(){sections.push(json!({"locator":format!("第 {} 段",i+1),"text":paragraph}));}
    }else{anyhow::bail!("支持PDF、DOCX、UTF-8 TXT/Markdown；旧DOC请先转换");}
    let material=json!({"id":id(),"title":name,"role":role,"strictRules":strict,"asset":asset,"format":if lower.ends_with(".pdf"){"pdf"}else if lower.ends_with(".docx"){"docx"}else{"text"},"sections":sections,"images":images,"importedAt":now()});store.save("material",material["id"].as_str().unwrap(),material.clone(),None,false,false)?;Ok(material)
}
pub fn search(store:&Store,query:&str,selected:&HashSet<String>)->Result<Value>{
    let clean=query.trim().to_lowercase();let mut terms:Vec<String>=clean.split_whitespace().map(String::from).collect();let chars:Vec<_>=clean.chars().collect();for window in chars.windows(2).take(100){terms.push(window.iter().collect());}
    let mut hits=vec![];for r in store.list("material")?{let m=r.payload;if !selected.is_empty()&&!selected.contains(m["id"].as_str().unwrap_or("")){continue;}if let Some(sections)=m["sections"].as_array(){for s in sections{let text=s["text"].as_str().unwrap_or("");let lower=text.to_lowercase();let mut score=if !clean.is_empty()&&lower.contains(&clean){20}else{0};score+=terms.iter().filter(|t|!t.is_empty()&&lower.contains(t.as_str())).count();if score==0{continue;}let offset=lower.find(&clean).unwrap_or(0);let at=text.char_indices().map(|(i,_)|i).take_while(|i|*i<=offset.saturating_sub(120)).last().unwrap_or(0);hits.push((score,json!({"materialId":m["id"],"title":m["title"],"role":m["role"],"locator":s["locator"],"text":clip(&text[at..],2400),"strictRules":m["strictRules"]})));}}}
    hits.sort_by(|a,b|b.0.cmp(&a.0));Ok(Value::Array(hits.into_iter().take(8).map(|(_,v)|v).collect()))
}
