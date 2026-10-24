pub mod domain;
pub mod store;
pub mod network;
pub mod materials;
pub mod ai;
pub mod sync;
pub mod agent;
pub mod learning;
pub mod vision;
pub mod practice;
mod platform;
mod controller;
#[cfg(target_os = "android")]
mod ui;


#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: android_activity::AndroidApp) {
    android_logger::init_once(android_logger::Config::default().with_max_level(log::LevelFilter::Info));
    log::info!("EasyGot native runtime starting");
    if let Err(error)=ui::run(app) { log::error!("UI startup failed: {error}"); }
}
