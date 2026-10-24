//! Slint adapter. The controller has no dependency on Slint or Android rendering.
use std::{cell::RefCell, rc::Rc, time::Duration};
use slint::{ComponentHandle, Model, ModelRc, VecModel};
use crate::{controller, platform::Bridge};
slint::include_modules!();

fn row(entry: controller::Entry) -> Entry {
    let image = if entry.image.is_empty() { slint::Image::default() } else {
        slint::Image::load_from_path(std::path::Path::new(&entry.image)).unwrap_or_default()
    };
    Entry { key:entry.key.into(),kind:entry.kind,title:entry.title.into(),detail:entry.detail.into(),
        value:entry.value.into(),action:entry.action.into(),action2:entry.action2.into(),label2:entry.label2.into(),
        checked:entry.checked,enabled:entry.enabled,image }
}
fn refresh(window:&AppWindow,app:&mut controller::EasyGot,model:&Rc<VecModel<Entry>>) {
    window.set_page_title(app.title().into());window.set_route(app.view.clone().into());window.set_tab(app.tab as i32);
    window.set_home(app.tab==0&&app.view.is_empty());window.set_status(app.status.clone().into());window.set_busy(app.busy);
    window.set_assistant_open(app.assistant_open);window.set_query(app.query.clone().into());
    window.set_assistant_answer(app.chat_answer.clone().into());window.set_allow_web(app.allow_web);
    window.set_confirmation(if app.confirm.is_empty(){"".into()}else{app.confirmation().into()});
    window.set_stats(ModelRc::new(VecModel::from(app.stats().into_iter().map(Into::into).collect::<Vec<_>>())));
    let entries=app.rows();
    // Keep existing repeater instances and input focus while updating search results.
    for(index,entry)in entries.iter().cloned().enumerate(){let mut item=row(entry);item.enabled &= !app.busy;
        if index<model.row_count(){if model.row_data(index).as_ref()!=Some(&item){model.set_row_data(index,item);}}
        else{model.push(item);}}
    while model.row_count()>entries.len(){model.remove(model.row_count()-1);}
    app.dirty=false;
}
pub fn run(app:android_activity::AndroidApp)->anyhow::Result<()> {
    slint::android::init(app.clone()).map_err(|e|anyhow::anyhow!("{e}"))?;
    let window=AppWindow::new()?;
    let state=Rc::new(RefCell::new(controller::EasyGot::new(Bridge{app})?));
    let model=Rc::new(VecModel::from(Vec::<Entry>::new()));window.set_rows(model.clone().into());
    refresh(&window,&mut state.borrow_mut(),&model);
    let weak=window.as_weak();let state_action=state.clone();let model_action=model.clone();
    window.on_action(move|action|{let Some(window)=weak.upgrade()else{return;};let mut app=state_action.borrow_mut();app.perform(&action);refresh(&window,&mut app,&model_action);});
    let weak=window.as_weak();let state_field=state.clone();let model_field=model.clone();
    window.on_field(move|key,value|{let Some(window)=weak.upgrade()else{return;};let mut app=state_field.borrow_mut();if let Err(e)=app.set_field(&key,value.to_string()){app.status=e.to_string();app.dirty=true;}if app.dirty{refresh(&window,&mut app,&model_field);}});
    let state_ball=state.clone();window.on_ball_moved(move|x,y|{state_ball.borrow_mut().assistant_pos=(x,y);});
    let timer=slint::Timer::default();let weak=window.as_weak();let state_timer=state.clone();
    timer.start(slint::TimerMode::Repeated,Duration::from_millis(200),move||{
        let Some(window)=weak.upgrade()else{return;};let mut app=state_timer.borrow_mut();
        app.assistant_pos=(window.get_ball_x(),window.get_ball_y());app.tick();window.set_clock(app.clock().into());
        if let Ok(value)=app.bridge.call("insets",""){if let Ok(value)=serde_json::from_str::<serde_json::Value>(&value){
            window.set_safe_top(value["top"].as_f64().unwrap_or(28.)as f32);
            window.set_safe_bottom(value["bottom"].as_f64().unwrap_or(20.)as f32);
            window.set_keyboard_height(value["ime"].as_f64().unwrap_or(0.)as f32);
            window.global::<Theme>().set_scale(value["fontScale"].as_f64().unwrap_or(1.).clamp(0.85,2.)as f32);
        }}
        if app.dirty{refresh(&window,&mut app,&model);}
    });
    window.run()?;state.borrow_mut().save_session()?;Ok(())
}
