use egui::{
    color_picker::{color_picker_color32, Alpha},
    panel::Side,
    Button, Color32, DragValue, Stroke, Ui,
};

use crate::{Dimensions, Scene, Strip};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct StripApp {
    scene: Scene,
    color_counter: usize,
}

impl Default for StripApp {
    fn default() -> Self {
        Self {
            color_counter: 0,
            scene: Scene::default(),
        }
    }
}

impl StripApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customized the look at feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }
}

impl eframe::App for StripApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::new(Side::Left, "Strips").show(ctx, |ui| {
            strip_ui(ui, &mut self.scene.strips, &mut self.color_counter);
        });
    }
}

fn strip_ui(ui: &mut Ui, strips: &mut Vec<Strip>, color_counter: &mut usize) {
    ui.horizontal(|ui| {
        if ui.button("+").clicked() {
            let color = COLOR_TABLE[*color_counter % COLOR_TABLE.len()];
            *color_counter += 1;
            strips.push(Strip {
                position: [0.5; 2],
                size: [4.8, 50.],
                rotation: 0.,
                color,
            })
        }

        if ui.button("Clear").clicked() {
            strips.clear();
        }
    });

    let mut do_remove = None;
    let mut do_dup = None;

    egui::containers::ScrollArea::vertical().show(ui, |ui| {
        for (idx, strip) in strips.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                // Change color
                let butt = Button::new("Color").fill(strip.color);
                if ui.add(butt).clicked() {
                    strip.color = COLOR_TABLE[*color_counter % COLOR_TABLE.len()];
                    *color_counter += 1;
                }

                // XY
                let speed = 0.003;
                ui.add(
                    DragValue::new(&mut strip.position[0])
                        .prefix("X: ")
                        .speed(speed),
                );
                ui.add(
                    DragValue::new(&mut strip.position[1])
                        .prefix("Y: ")
                        .speed(speed),
                );

                // Height
                ui.add(
                    DragValue::new(&mut strip.size[1])
                        .prefix("Height: ")
                        .suffix(" cm"),
                );

                // Rotate
                ui.add(
                    DragValue::new(&mut strip.rotation)
                        .prefix("Angle: ")
                        .suffix("°")
                        .speed(0.25),
                );

                // Duplicate
                if ui.button("Dup").clicked() {
                    do_dup = Some(idx);
                }

                // Delete
                if ui.button("🗑").clicked() {
                    do_remove = Some(idx);
                }
            });
        }
    });

    if let Some(idx) = do_remove {
        strips.remove(idx);
    }

    if let Some(idx) = do_dup {
        strips.insert(idx, strips[idx]);
    }
}

const COLOR_TABLE: [Color32; 17] = [
    Color32::GRAY,
    Color32::LIGHT_GRAY,
    Color32::WHITE,
    Color32::BROWN,
    Color32::DARK_RED,
    Color32::RED,
    Color32::LIGHT_RED,
    Color32::YELLOW,
    Color32::LIGHT_YELLOW,
    Color32::KHAKI,
    Color32::DARK_GREEN,
    Color32::GREEN,
    Color32::LIGHT_GREEN,
    Color32::DARK_BLUE,
    Color32::BLUE,
    Color32::LIGHT_BLUE,
    Color32::GOLD,
];
