//! Slint adapter. The controller has no dependency on Slint or Android rendering.
use crate::{controller, platform::Bridge};
use slint::{ComponentHandle, Model, ModelRc, VecModel};
use std::{cell::RefCell, rc::Rc, time::Duration};
slint::include_modules!();

fn row(mut entry: controller::Entry) -> Entry {
    let image = if entry.image.is_empty() {
        slint::Image::default()
    } else {
        match crate::media::preview_path(std::path::Path::new(&entry.image))
            .and_then(|path| Ok(slint::Image::load_from_path(&path)?))
        {
            Ok(image) => image,
            Err(error) => {
                entry.kind = 0;
                entry.title = "暂时无法显示图片".into();
                entry.detail = error.to_string();
                slint::Image::default()
            }
        }
    };
    Entry {
        key: entry.key.into(),
        kind: entry.kind,
        title: entry.title.into(),
        detail: entry.detail.into(),
        value: entry.value.into(),
        action: entry.action.into(),
        action2: entry.action2.into(),
        label2: entry.label2.into(),
        checked: entry.checked,
        enabled: entry.enabled,
        image,
    }
}
fn refresh(window: &AppWindow, app: &mut controller::EasyGot, model: &Rc<VecModel<Entry>>) {
    window.set_page_title(app.title().into());
    window.set_route(app.view.clone().into());
    window.set_tab(app.tab as i32);
    window.set_home(app.tab == 0 && app.view.is_empty());
    window.set_status(app.status.clone().into());
    window.set_busy(app.busy);
    window.set_assistant_open(app.assistant_open);
    window.set_query(app.query.clone().into());
    window.set_assistant_answer(app.chat_answer.clone().into());
    window.set_allow_web(app.allow_web);
    window.set_confirmation(if app.confirm.is_empty() {
        "".into()
    } else {
        app.confirmation().into()
    });
    window.set_stats(ModelRc::new(VecModel::from(
        app.stats().into_iter().map(Into::into).collect::<Vec<_>>(),
    )));
    let entries = app.rows();
    // Keep existing repeater instances and input focus while updating search results.
    for (index, entry) in entries.iter().cloned().enumerate() {
        let mut item = row(entry);
        item.enabled &= !app.busy;
        if index < model.row_count() {
            if model.row_data(index).as_ref() != Some(&item) {
                model.set_row_data(index, item);
            }
        } else {
            model.push(item);
        }
    }
    while model.row_count() > entries.len() {
        model.remove(model.row_count() - 1);
    }
    app.dirty = false;
}
#[cfg(target_os = "android")]
pub fn run(app: android_activity::AndroidApp) -> anyhow::Result<()> {
    let state = Rc::new(RefCell::new(controller::EasyGot::new(Bridge {
        app: app.clone(),
    })?));
    let lifecycle = state.clone();
    slint::android::init_with_event_listener(app, move |event| {
        use android_activity::{MainEvent, PollEvent};
        match event {
            PollEvent::Main(MainEvent::Pause) => {
                let mut app = lifecycle.borrow_mut();
                if let Err(e) = app.save_session() {
                    app.status = e.to_string();
                    app.dirty = true;
                }
            }
            PollEvent::Main(MainEvent::Resume { .. }) => lifecycle.borrow_mut().resumed(),
            _ => {}
        }
    })
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    let window = AppWindow::new()?;
    let model = Rc::new(VecModel::from(Vec::<Entry>::new()));
    window.set_rows(model.clone().into());
    refresh(&window, &mut state.borrow_mut(), &model);
    let weak = window.as_weak();
    let state_action = state.clone();
    let model_action = model.clone();
    window.on_action(move |action| {
        let Some(window) = weak.upgrade() else {
            return;
        };
        let mut app = state_action.borrow_mut();
        app.perform(&action);
        refresh(&window, &mut app, &model_action);
    });
    let weak = window.as_weak();
    let state_field = state.clone();
    let model_field = model.clone();
    window.on_field(move |key, value| {
        let Some(window) = weak.upgrade() else {
            return;
        };
        let mut app = state_field.borrow_mut();
        if let Err(e) = app.set_field(&key, value.to_string()) {
            app.status = e.to_string();
            app.dirty = true;
        }
        if app.dirty {
            refresh(&window, &mut app, &model_field);
        }
    });
    let state_ball = state.clone();
    window.on_ball_moved(move |x, y| {
        state_ball.borrow_mut().assistant_pos = (x, y);
    });
    let bounds = Rc::new(RefCell::new(std::collections::BTreeMap::<
        String,
        serde_json::Value,
    >::new()));
    let measured = bounds.clone();
    window.on_measured(move |key, x, y, width, height| {
        measured.borrow_mut().insert(
            key.to_string(),
            serde_json::json!({"x":x,"y":y,"width":width,"height":height}),
        );
    });
    let mut snapshot_pending = false;
    let timer = slint::Timer::default();
    let weak = window.as_weak();
    let state_timer = state.clone();
    timer.start(slint::TimerMode::Repeated,Duration::from_millis(200),move||{
        let Some(window)=weak.upgrade()else{return;};let mut app=state_timer.borrow_mut();
        app.assistant_pos=(window.get_ball_x(),window.get_ball_y());app.tick();window.set_clock(app.clock().into());
        if let Ok(value)=app.bridge.call("insets",""){if let Ok(value)=serde_json::from_str::<serde_json::Value>(&value){
            window.set_safe_top(value["top"].as_f64().unwrap_or(28.)as f32);
            window.set_safe_bottom(value["bottom"].as_f64().unwrap_or(20.)as f32);

            window.global::<Theme>().set_scale(value["fontScale"].as_f64().unwrap_or(1.).clamp(0.85,2.)as f32);
        }}
        if app.dirty{refresh(&window,&mut app,&model);}
        if snapshot_pending{
            let mut value=app.qa_state();value["viewport"]=serde_json::json!({"top":window.get_content_top(),"bottom":window.get_content_bottom()});value["scale"]=serde_json::json!(window.window().scale_factor());value["bounds"]=serde_json::json!(*bounds.borrow());value["window"]=serde_json::json!({"width":window.window().size().width,"height":window.window().size().height});
            let _=std::fs::write(app.store.root.join("qa-state.json"),value.to_string());snapshot_pending=false;
        }
        if app.qa_snapshot_requested{app.qa_snapshot_requested=false;bounds.borrow_mut().clear();window.set_measure_token(window.get_measure_token()+1);snapshot_pending=true;}

    });
    window.run()?;
    state.borrow_mut().save_session()?;
    Ok(())
}

/// Render the actual Slint component without an OS window for layout regression review.
/// This supplements Android captures; it does not verify Android IME or services.
#[cfg(all(feature = "ui-check", not(target_os = "android")))]
pub fn render_samples(directory: &std::path::Path, font: &std::path::Path) -> anyhow::Result<()> {
    use slint::platform::{
        software_renderer::{MinimalSoftwareWindow, RepaintBufferType},
        Platform, PlatformError, WindowAdapter,
    };
    struct Headless(Rc<MinimalSoftwareWindow>);
    impl Platform for Headless {
        fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
            Ok(self.0.clone())
        }
    }
    std::fs::create_dir_all(directory)?;
    let adapter = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
    slint::platform::set_platform(Box::new(Headless(adapter.clone())))
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    if font.exists() {
        use slint::fontique_011::fontique;
        let mut collection = slint::fontique_011::shared_collection();
        let fonts = collection.register_fonts(
            fontique::Blob::new(std::sync::Arc::new(std::fs::read(font)?)),
            None,
        );
        collection.append_fallbacks(
            fontique::FallbackKey::new(fontique::Script::from_str_unchecked("Hani"), None),
            fonts.iter().map(|f| f.0),
        );
    }
    let window = AppWindow::new()?;
    window
        .window()
        .dispatch_event(slint::platform::WindowEvent::ScaleFactorChanged { scale_factor: 2. });
    adapter.set_size(slint::PhysicalSize::new(700, 1536));
    window.show()?;
    let model = Rc::new(VecModel::from(Vec::<Entry>::new()));
    window.set_rows(model.clone().into());
    let root = directory.join("sample-data");
    let mut app = controller::EasyGot::new(Bridge { root })?;
    for screen in [
        "home",
        "question",
        "directory",
        "settings",
        "assistant",
        "paper",
        "large-home",
    ] {
        app.assistant_open = false;
        app.perform("nav:0");
        window
            .global::<Theme>()
            .set_scale(if screen == "large-home" { 1.3 } else { 1. });
        match screen {
            "question" => {
                app.perform("sample");
                app.perform("practice");
            }
            "directory" => {
                let bank =
                    serde_json::from_str(include_str!("../../examples/multi-paper-bank.json"))?;
                let id = crate::learning::import_bank(&app.store, &bank)?;
                app.perform("nav:1");
                app.perform(&format!("bank:{id}"));
                app.perform("choose-directory");
            }
            "settings" => app.perform("nav:4"),
            "assistant" => app.perform("assistant"),
            "paper" => {
                app.perform("sample");
                app.perform("paper-new");
            }
            _ => {}
        }
        refresh(&window, &mut app, &model);
        slint::platform::update_timers_and_animations();
        window.window().request_redraw();
        let mut buffer = slint::SharedPixelBuffer::<slint::Rgb8Pixel>::new(700, 1536);
        assert!(adapter.draw_if_needed(|renderer| {
            renderer.render(buffer.make_mut_slice(), 700);
        }));
        image::save_buffer(
            directory.join(format!("{screen}.png")),
            buffer.as_bytes(),
            700,
            1536,
            image::ColorType::Rgb8,
        )?;
    }
    window.hide()?;
    Ok(())
}
