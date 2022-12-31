use image::{DynamicImage, GrayImage};
use nannou::image;
use nannou::prelude::*;
use nannou_egui::{self, egui, Egui};
use rayon::prelude::*;
use wgpu::Texture;

const ORIGINAL_IMAGE_PATH: &str = "input.jpg";

fn main() {
    nannou::app(model).update(update).run();
}

#[derive(PartialEq, Clone)]
struct Settings {
    area_size: usize,
    values: Vec<f64>,
}

struct Model {
    original_image: GrayImage,
    original_texture: Texture,
    new_texture: Texture,
    egui: Egui,
    settings: Settings,
    prev_settings: Settings,
}

fn model(app: &App) -> Model {
    let window_id = app
        .new_window()
        .view(view)
        .maximized(true)
        .raw_event(raw_window_event)
        .build()
        .unwrap();
    let original_image_dynamic = image::open(ORIGINAL_IMAGE_PATH).unwrap().grayscale();
    let original_texture = grayscale_to_texture(app, original_image_dynamic.to_luma8());
    let original_image = original_image_dynamic.to_luma8();
    let window = app.window(window_id).unwrap();
    let egui = Egui::from_window(&window);
    let settings = Settings {
        area_size: 2,
        values: vec![1.0; 4],
    };
    Model {
        original_image,
        original_texture: original_texture.clone(),
        new_texture: original_texture,
        egui,
        settings: settings.clone(),
        prev_settings: settings,
    }
}

fn update(app: &App, model: &mut Model, update: Update) {
    let egui = &mut model.egui;
    let settings = &mut model.settings;

    egui.set_elapsed_time(update.since_start);
    let ctx = egui.begin_frame();
    egui::Window::new("Settings").show(&ctx, |ui| {
        ui.label("Area Size");
        ui.add(egui::Slider::new(&mut settings.area_size, 1..=8));
        if settings.area_size != model.prev_settings.area_size {
            settings.values = vec![1.0; settings.area_size * settings.area_size];
        }
        egui::Grid::new("grid_id").show(ui, |ui| {
            for y in 0..settings.area_size {
                for x in 0..settings.area_size {
                    ui.add(egui::DragValue::new(
                        &mut settings.values[y * settings.area_size + x],
                    ));
                }
                ui.end_row();
            }
        });
    });

    if model.prev_settings != *settings {
        let Settings { area_size, values } = settings;
        let original_image = &model.original_image;
        let new_image = GrayImage::new(
            original_image.width() - (*area_size as u32),
            original_image.height() - (*area_size as u32),
        );
        let v: Vec<f64> = new_image
            .par_iter()
            .enumerate()
            .map(|(index, _p)| {
                let index_x = index % (new_image.width() as usize);
                let index_y = index / (new_image.height() as usize);
                let mut out = 0.0;
                for offset_y in 0..*area_size {
                    for offset_x in 0..*area_size {
                        let lhs = original_image
                            .get_pixel((index_x + offset_x) as u32, (index_y + offset_y) as u32)[0]
                            as f64;
                        let rhs = values[offset_y * (*area_size) + offset_x];
                        out += lhs * rhs;
                    }
                }
                if out < 0.0 {
                    out = 0.0;
                }
                out
            })
            .collect();
        let min = v.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max = v.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let v = v
            .into_par_iter()
            .map(|p| map_range((min, max), (0.0, 255.0), p) as u8)
            .collect();
        let new_image = GrayImage::from_vec(new_image.width(), new_image.height(), v).unwrap();
        model.new_texture = grayscale_to_texture(app, new_image);
        model.prev_settings = settings.clone()
    }
}

fn view(app: &App, model: &Model, frame: Frame) {
    let draw = app.draw();
    draw.background().color(BLACK);
    let win_rect = app.window_rect();
    let (width, height) = {
        let extent = model.original_texture.extent();
        (extent.width, extent.height)
    };
    let original_rect = Rect::from_w_h(width as f32, height as f32).top_left_of(win_rect);
    let new_rect = Rect::from_w_h(width as f32, height as f32).top_right_of(win_rect);
    draw.texture(&model.original_texture)
        .xy(original_rect.xy())
        .wh(original_rect.wh());
    draw.texture(&model.new_texture)
        .xy(new_rect.xy())
        .wh(new_rect.wh());
    draw.to_frame(app, &frame).unwrap();
    model.egui.draw_to_frame(&frame).unwrap();
}

fn raw_window_event(_app: &App, model: &mut Model, event: &nannou::winit::event::WindowEvent) {
    // Let egui handle things like keyboard and mouse input.
    model.egui.handle_raw_event(event);
}
fn grayscale_to_texture(app: &App, i: GrayImage) -> Texture {
    Texture::from_image(
        app,
        &DynamicImage::ImageRgb8(DynamicImage::ImageLuma8(i).to_rgb8()),
    )
}

// Stolen from https://rosettacode.org/wiki/Map_range#Rust
pub fn map_range<T: Copy>(from_range: (T, T), to_range: (T, T), s: T) -> T
where
    T: std::ops::Add<T, Output = T>
        + std::ops::Sub<T, Output = T>
        + std::ops::Mul<T, Output = T>
        + std::ops::Div<T, Output = T>,
{
    to_range.0 + (s - from_range.0) * (to_range.1 - to_range.0) / (from_range.1 - from_range.0)
}
