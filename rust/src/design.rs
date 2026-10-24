//! Visual tokens adapted from the user's mobile reference; Material Symbols are from Google's icon font.
use eframe::egui::{self,Color32,RichText};
pub const BG:Color32=Color32::from_rgb(239,237,240);
pub const CARD:Color32=Color32::from_rgb(253,251,253);
pub const BLUE:Color32=Color32::from_rgb(0,122,255);
pub const INK:Color32=Color32::from_rgb(26,30,38);
pub const MUTED:Color32=Color32::from_rgb(112,116,128);
pub const INSET:Color32=Color32::from_rgb(240,238,242);
pub fn icon(code:char,size:f32)->RichText{RichText::new(code.to_string()).font(egui::FontId::new(size,egui::FontFamily::Name("icons".into())))}
pub fn card()->egui::Frame{egui::Frame::new().fill(CARD).corner_radius(28).inner_margin(20).shadow(egui::Shadow{offset:[0,5],blur:18,spread:0,color:Color32::from_black_alpha(12)})}
pub fn inset()->egui::Frame{egui::Frame::new().fill(INSET).corner_radius(22).inner_margin(16)}
pub fn primary(ui:&mut egui::Ui,label:&str)->egui::Response{ui.add_sized([ui.available_width(),48.],egui::Button::new(RichText::new(label).color(Color32::WHITE).strong()).fill(BLUE).corner_radius(24))}
pub fn secondary(ui:&mut egui::Ui,label:&str)->egui::Response{ui.add_sized([ui.available_width(),44.],egui::Button::new(RichText::new(label).color(INK)).fill(INSET).corner_radius(22))}
pub fn section(ui:&mut egui::Ui,icon_code:char,title:&str){ui.horizontal(|ui|{ui.label(icon(icon_code,22.).color(INK));ui.label(RichText::new(title).strong().size(19.).color(INK));});ui.add_space(8.);}
pub fn metric(ui:&mut egui::Ui,label:&str,value:&str,detail:&str){inset().show(ui,|ui|{ui.set_min_width((ui.available_width()-12.).max(80.));ui.label(RichText::new(label).size(13.).color(MUTED));ui.add_space(9.);ui.label(RichText::new(value).size(27.).strong().color(INK));ui.label(RichText::new(detail).size(12.).color(MUTED));});}
pub fn apply(ctx:&egui::Context){let mut style=(*ctx.style()).clone();style.visuals=egui::Visuals::light();style.visuals.panel_fill=BG;style.visuals.window_fill=CARD;style.visuals.extreme_bg_color=INSET;style.visuals.faint_bg_color=INSET;style.visuals.override_text_color=Some(INK);style.visuals.selection.bg_fill=BLUE;style.visuals.selection.stroke=egui::Stroke::new(1.,Color32::WHITE);style.visuals.widgets.noninteractive.bg_stroke=egui::Stroke::NONE;style.visuals.widgets.inactive.bg_fill=INSET;style.visuals.widgets.inactive.weak_bg_fill=INSET;style.visuals.widgets.inactive.bg_stroke=egui::Stroke::NONE;style.visuals.widgets.inactive.corner_radius=egui::CornerRadius::same(14);style.visuals.widgets.hovered.corner_radius=egui::CornerRadius::same(14);style.visuals.widgets.active.corner_radius=egui::CornerRadius::same(14);style.visuals.widgets.hovered.bg_fill=Color32::from_rgb(228,235,249);style.visuals.widgets.active.bg_fill=Color32::from_rgb(208,227,255);style.spacing.item_spacing=egui::vec2(10.,12.);style.spacing.button_padding=egui::vec2(14.,12.);style.spacing.interact_size=egui::vec2(44.,44.);style.text_styles.insert(egui::TextStyle::Body,egui::FontId::proportional(16.));style.text_styles.insert(egui::TextStyle::Button,egui::FontId::proportional(15.));style.text_styles.insert(egui::TextStyle::Heading,egui::FontId::proportional(27.));ctx.set_style(style);}
