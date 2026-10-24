pub mod domain;
pub mod store;
pub mod network;
pub mod materials;
pub mod ai;
pub mod sync;
pub mod agent;
pub mod learning;
pub mod vision;
#[cfg(target_os = "android")]
mod platform;
#[cfg(target_os = "android")]
mod ui;
#[cfg(target_os = "android")]
mod design;

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: android_activity::AndroidApp) {
    android_logger::init_once(android_logger::Config::default().with_max_level(log::LevelFilter::Info));
    let options = eframe::NativeOptions { android_app: Some(app.clone()), ..Default::default() };
    let result=eframe::run_native("EasyGot",options,Box::new(move|cc|Ok(Box::new(ui::EasyGot::new(cc,app)))));
    if let Err(error)=result { log::error!("UI startup failed: {error}"); }
}
