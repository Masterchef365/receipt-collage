use std::{
    fs::File,
    io::BufWriter,
    path::{Path, PathBuf},
};

use eframe::emath::Rot2;
use egui::{
    color_picker::{color_picker_color32, Alpha},
    panel::{Side, TopBottomSide},
    plot::{Line, Plot, PlotImage, PlotPoint, PlotUi},
    Button, Color32, ColorImage, Context, DragValue, Pos2, Stroke, TextureHandle, TextureId, Ui,
    Vec2,
};
use png::{BitDepth, ColorType};

use crate::{Dimensions, Scene, Strip};

const STRIP_DRAW_WIDTH: f32 = 4.8; // cm
const STRIP_PAPER_WIDTH: f32 = 5.8; // cm
const STRIP_PIXELS_PER_ROW: usize = 384;
const STRIP_DOTS_PER_CM: f32 = STRIP_PIXELS_PER_ROW as f32 / STRIP_DRAW_WIDTH;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct StripApp {
    scene: Scene,
    color_counter: usize,
    image_path: Option<PathBuf>,

    #[serde(skip)]
    texture: Option<TextureHandle>,

    #[serde(skip)]
    image_data: Option<ColorImage>,
}

impl Default for StripApp {
    fn default() -> Self {
        Self {
            texture: None,
            image_path: None,
            image_data: None,
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

    fn load_image(&mut self, ctx: &Context) {
        let Some(path) = self.image_path.as_ref() else {
            return;
        };

        let file = match File::open(path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("Failed to open {}; {:?}", path.display(), e);
                return;
            }
        };

        let decoder = png::Decoder::new(file);
        let mut reader = decoder.read_info().unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();

        if info.bit_depth != BitDepth::Eight {
            eprintln!("Bit depth must be 8, got {:?}", info.bit_depth);
            return;
        }

        if info.color_type != ColorType::Rgba {
            eprintln!("Color type must RGBA, got {:?}", info.color_type);
            return;
        }

        buf.truncate(info.buffer_size());

        let image =
            ColorImage::from_rgba_unmultiplied([info.width as usize, info.height as usize], &buf);

        let tex = ctx.load_texture(
            path.display().to_string(),
            image.clone(),
            egui::TextureFilter::Nearest,
        );

        self.image_data = Some(image);

        self.scene.dims.resolution = [info.width, info.height];

        self.texture = Some(tex);
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
        // Load image if not present!
        if self.image_path.is_some() && self.texture.is_none() {
            self.load_image(ctx);
        }

        egui::TopBottomPanel::new(TopBottomSide::Top, "Controls")
            .min_height(100.)
            .show(ctx, |ui| {
                // Load image
                if ui.button("Load image").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("PNG", &["png"])
                        .pick_file()
                    {
                        self.image_path = Some(path);
                        self.load_image(ui.ctx());
                    }
                }

                // Size controls
                ui.horizontal(|ui| {
                    ui.add(
                        DragValue::new(&mut self.scene.dims.width)
                            .prefix("Width: ")
                            .suffix("cm")
                            .clamp_range(0.0..=f32::MAX),
                    );
                    ui.label(format!("Height: {} cm", self.scene.dims.height()));
                });

                ui.horizontal(|ui| {
                    // Save config
                    if ui.button("Save config").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("RON", &["ron"])
                            .save_file()
                        {
                            let f = File::create(path).expect("Failed to create file");
                            ron::ser::to_writer_pretty(f, &self.scene, Default::default()).unwrap();
                        }
                    }

                    // Load config
                    if ui.button("Load config").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("RON", &["ron"])
                            .pick_file()
                        {
                            let f = File::open(path).expect("Failed to open file");
                            self.scene = ron::de::from_reader(f).unwrap();
                        }
                    }
                });

                if ui.button("Save images").clicked() {
                    if let Some(input_img) = self.image_data.as_ref() {
                        sample_strips(input_img, &self.scene.strips, &self.scene.dims)
                    }
                }

                // Stip controls
                strip_controls(ui, &mut self.scene.strips, &mut self.color_counter);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            strip_plot(ui, &self.scene, self.texture.as_ref().map(|t| t.id()));
        });
    }
}

fn strip_plot(ui: &mut Ui, scene: &Scene, tex_id: Option<TextureId>) {
    Plot::new("Plot").data_aspect(1.).show(ui, |ui| {
        // Reference image
        if let Some(id) = tex_id {
            let size = Vec2::new(scene.dims.width(), scene.dims.height());
            ui.image(PlotImage::new(
                id,
                PlotPoint::new(size.x / 2., size.y / 2.),
                size,
            ))
        }

        // Strips
        for strip in &scene.strips {
            draw_strip(ui, strip, &scene.dims);
        }
    });
}

fn draw_strip(ui: &mut PlotUi, strip: &Strip, dims: &Dimensions) {
    let mut draw_size = |width: f32| {
        draw_rectangle(
            ui,
            Pos2::from(strip.position.map(|v| v * dims.cm_per_norm())),
            Vec2::new(width, strip.size[1]),
            strip.color,
            strip.rotation.to_radians(),
        )
    };

    draw_size(strip.size[0]);
    draw_size(STRIP_PAPER_WIDTH);
}

fn draw_rectangle(ui: &mut PlotUi, pos: Pos2, size: Vec2, color: Color32, angle: f32) {
    let rot = Rot2::from_angle(angle);

    let radius = size / 2.;

    let points = [
        Vec2::new(radius.x, radius.y),
        Vec2::new(radius.x, -radius.y),
        Vec2::new(-radius.x, -radius.y),
        Vec2::new(-radius.x, radius.y),
        Vec2::new(radius.x, radius.y),
    ];

    let points = points.map(|v| pos + rot * v);

    for pair in points.windows(2) {
        let points = vec![
            [pair[0].x, pair[0].y].map(f64::from),
            [pair[1].x, pair[1].y].map(f64::from),
        ];
        ui.line(Line::new(points).color(color));
    }
}

fn strip_controls(ui: &mut Ui, strips: &mut Vec<Strip>, color_counter: &mut usize) {
    ui.horizontal(|ui| {
        if ui.button("+").clicked() {
            let color = COLOR_TABLE[*color_counter % COLOR_TABLE.len()];
            *color_counter += 1;
            strips.push(Strip {
                position: [0.5; 2],
                size: [STRIP_DRAW_WIDTH, 50.],
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
                let speed = 0.002;
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
                        .suffix(" cm")
                        .speed(0.5)
                        .clamp_range(0.0..=f32::MAX),
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

const COLOR_TABLE: [Color32; 17 - 2] = [
    //Color32::GRAY,
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
    //Color32::DARK_BLUE,
    Color32::BLUE,
    Color32::LIGHT_BLUE,
    Color32::GOLD,
];

fn sample_strips(input_img: &ColorImage, strips: &[Strip], dims: &Dimensions) {
    for (idx, strip) in strips.iter().enumerate() {
        let strip_img = sample_strip(input_img, strip, STRIP_DOTS_PER_CM, dims);
        let fname = format!("{}.png", idx);
        save_image(fname, &strip_img);
    }
}

fn save_image(path: impl AsRef<Path>, image: &ColorImage) {
    let file = File::create(path).unwrap();
    let ref mut w = BufWriter::new(file);

    let mut encoder = png::Encoder::new(w, image.width() as _, image.height() as _); // Width is 2 pixels and height is 1.
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();

    let bytes: Vec<u8> = image
        .pixels
        .iter()
        .map(|p| p.to_array())
        .flatten()
        .collect();

    writer.write_image_data(&bytes).unwrap();
}

fn sample_strip(
    input_img: &ColorImage,
    strip: &Strip,
    dots_per_cm: f32,
    dims: &Dimensions,
) -> ColorImage {
    let mut strip_img = ColorImage::new(
        strip.size.map(|v| (v * dots_per_cm) as usize),
        Color32::WHITE,
    );

    for y in 0..strip_img.height() {
        for x in 0..strip_img.width() {
            let cm = strip_pixel_cm(x, y, strip, dots_per_cm, dims);
            if let Some(idx) = image_cm_index(cm, dims) {
                strip_img[(x, y)] = input_img[idx];
            }
        }
    }

    strip_img
}

/// Translates the given pixel on the given strip into cm in the image space
fn strip_pixel_cm(x: usize, y: usize, strip: &Strip, dots_per_cm: f32, dims: &Dimensions) -> Vec2 {
    let px = Vec2::new(x as f32, y as f32);
    let wh = Vec2::from(strip.size);

    let xy = px / wh / dots_per_cm; // Normalize to 0 to 1
    let xy = xy * 2. - Vec2::splat(1.); // Convert to -1 to 1
    let xy = xy * wh / 2.; // Convert back to cm

    let r = Rot2::from_angle(-strip.rotation.to_radians());

    let pos = dims.cm_per_norm() * Vec2::from(strip.position);

    pos + r * xy
}

/// Translates a position in cm into a pixel index in the iamge
fn image_cm_index(xy: Vec2, dims: &Dimensions) -> Option<(usize, usize)> {
    let mut norm = xy / Vec2::new(dims.width(), dims.height());

    // The image is actually upside down!
    //norm.y = 1. - norm.y;

    let res = Vec2::from(dims.resolution.map(|v| v as f32));
    let px = res * norm;
    let x = px.x as isize;
    let y = px.y as isize;

    let x_bnd = x > 0 && x < dims.resolution[0] as isize;
    let y_bnd = y > 0 && y < dims.resolution[1] as isize;

    if x_bnd && y_bnd {
        Some((x as usize, y as usize))
    } else {
        None
    }
}
