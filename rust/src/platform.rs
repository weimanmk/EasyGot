use anyhow::{ensure,Result};use jni::{JavaVM,objects::{JObject,JString,JValue}};
#[derive(Clone)]pub struct Bridge{pub app:android_activity::AndroidApp}
impl Bridge{pub fn call(&self,command:&str,payload:&str)->Result<String>{
    // AndroidApp owns the global Activity reference; JObject does not delete it on drop.
    let vm=unsafe{JavaVM::from_raw(self.app.vm_as_ptr().cast())}?;let mut env=vm.attach_current_thread()?;let activity=unsafe{JObject::from_raw(self.app.activity_as_ptr().cast())};
    let command=env.new_string(command)?;let payload=env.new_string(payload)?;
    let value=match env.call_method(&activity,"bridge","(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;",&[JValue::Object(command.as_ref()),JValue::Object(payload.as_ref())]){Ok(v)=>v,Err(e)=>{let _=env.exception_clear();return Err(e.into());}};
    let string=JString::from(value.l()?);let result:String=env.get_string(&string)?.into();ensure!(!result.starts_with("ERROR:"),"{result}");Ok(result)
}}
