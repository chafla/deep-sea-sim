use std::io::Cursor;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;

use eframe::egui;
use egui::{TopBottomPanel, Vec2};
use egui_extras::RetainedImage;

// Include the background image in our compiled exe
const BACKGROUND_IMAGE: &[u8] = include_bytes!("../../../UI_Graphics/underwater.jpg");

pub struct SeaGui {
    first_input: String,
    second_input: String,
    third_input: String,
    game_info: Vec<f32>,
    start: bool,
    get_dim: bool,
    get_animals: bool,
    run_simulation: bool,
    pause: bool,
    event_msg: Vec<String>,
    event_res: String,
    previous_disp: String,
    background_img: Option<RetainedImage>,
    tx: Sender<(String, Vec<String>, String, Sender<bool>)>,
    rx: Receiver<(String, Vec<String>, String, Sender<bool>)>,
    loop_tx: Option<Sender<bool>>,
    entities_info: Vec<String>,
}
impl Default for SeaGui {
    fn default() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        Self {
            first_input: String::new(),
            second_input: String::new(),
            third_input: String::new(),
            game_info: Vec::new(),
            start: true,
            get_dim: false,
            get_animals: false,
            run_simulation: false,
            pause: false,
            event_msg: Vec::new(),
            event_res: String::new(),
            previous_disp: String::new(),
            background_img: None,
            tx,
            rx,
            loop_tx: None,
            entities_info: Vec::new(),
        }
    }
}
impl SeaGui {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Default::default()
    }

    pub fn render_top_panel(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    // Logo
                    // Crab isn't showing up =( should find a different logo
                    ui.add(egui::Label::new("🐋"));
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let close_btn = ui.add(egui::Button::new("❌"));
                    if close_btn.clicked() {
                        frame.close();
                    }
                    if !self.pause {
                        let pause_btn = ui.add(egui::Button::new("⏸"));
                        if pause_btn.clicked() {
                            self.pause = true;
                            ctx.request_repaint();
                        }
                    } else {
                        let pause_btn = ui.add(egui::Button::new("▶"));
                        if pause_btn.clicked() {
                            self.pause = false;
                            ctx.request_repaint();
                        }
                    }
                })
            });
        });
    }
}
impl eframe::App for SeaGui {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.render_top_panel(ctx, frame);
        let background = egui::containers::Frame {
            fill: egui::Color32::from_rgb(97, 109, 128),
            ..Default::default()
        };
        if self.run_simulation {
            // Clear background frame
            let background = egui::containers::Frame {
                fill: egui::Color32::from_rgba_premultiplied(0, 0, 0, 0),
                ..Default::default()
            };
            egui::CentralPanel::default().show(ctx, |ui| {
                ctx.set_fonts(egui::FontDefinitions::default());
                if self.background_img.is_none() {
                    // Render the background image
                    let img = image::io::Reader::new(Cursor::new(BACKGROUND_IMAGE))
                        .with_guessed_format()
                        .unwrap()
                        .decode()
                        .unwrap();
                    let size = [img.width() as _, img.height() as _];
                    let image_buffer = img.to_rgba8();
                    let pixels = image_buffer.as_flat_samples();
                    let col_img = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
                    self.background_img = Some(egui_extras::RetainedImage::from_color_image(
                        "debug_name",
                        col_img,
                    ));
                }
                ui.image(
                    self.background_img.as_ref().unwrap().texture_id(ctx),
                    self.background_img.as_ref().unwrap().size_vec2(),
                );
                // Render the actual game info
                egui::CentralPanel::default()
                    .frame(background)
                    .show(ctx, |ui| {
                        // If there is not an event, process the next game tick
                        if self.event_msg.len() < 3 && !self.pause {
                            if let Ok(result) = self.rx.try_recv() {
                                self.previous_disp = result.0;
                                self.entities_info = result.1;
                                self.event_msg =
                                    result.2.split('*').map(|s| s.to_string()).collect();
                                self.loop_tx = Some(result.3);
                            }
                        }
                        // Display the board, either newly updated or the previous one
                        ui.with_layout(
                            egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                            |ui| {
                                ui.label(
                                    egui::RichText::new(format!("\n{}", self.previous_disp))
                                        .font(egui::FontId::proportional(110.0 * self.game_info[2]))
                                        .color(egui::Color32::from_rgb(10, 10, 10)),
                                );
                            },
                        );
                        // If there is an event, display it in a new window, pausing the game execution
                        // until the event has been handled
                        ui.with_layout(egui::Layout::top_down(egui::Align::Center), |_ui| {
                            egui::Window::new("Colony Info")
                                .vscroll(true)
                                .default_pos(egui::Pos2::new(1410.0, 0.0))
                                .show(ctx, |ui| {
                                    for i in self.entities_info.clone() {
                                        ui.label(
                                            egui::RichText::new(i)
                                                .font(egui::FontId::proportional(20.0)),
                                        );
                                    }
                                });
                        });
                        if self.event_msg.len() == 3 {
                            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |_ui| {
                                egui::Window::new("*EVENT*").show(ctx, |ui| {
                                    ui.label(
                                        egui::RichText::new(self.event_msg[0].clone())
                                            .font(egui::FontId::proportional(20.0)),
                                    );
                                    // process the result and display the result
                                    ui.with_layout(
                                        egui::Layout::top_down(egui::Align::Center),
                                        |ui| {
                                            let left = ui.add(
                                                egui::Button::new("1")
                                                    .min_size(egui::vec2(100.0, 30.0)),
                                            );
                                            if left.clicked() {
                                                self.event_res = self.event_msg[1].clone();
                                                let _ = self.loop_tx.clone().unwrap().send(false);
                                            }
                                            ui.add_space(20.0);
                                            let right = ui.add(
                                                egui::Button::new("2")
                                                    .min_size(egui::vec2(100.0, 30.0)),
                                            );
                                            if right.clicked() {
                                                self.event_res = self.event_msg[2].clone();
                                                let _ = self.loop_tx.clone().unwrap().send(true);
                                            }
                                        },
                                    );
                                    if !self.event_res.is_empty() {
                                        ui.label(
                                            egui::RichText::new(self.event_res.clone())
                                                .font(egui::FontId::proportional(20.0)),
                                        );
                                        ui.label("");
                                        ui.with_layout(
                                            egui::Layout::top_down(egui::Align::Center),
                                            |ui| {
                                                let done = ui.add(
                                                    egui::Button::new("Proceed")
                                                        .min_size(egui::vec2(100.0, 30.0)),
                                                );
                                                if done.clicked() {
                                                    self.event_msg = Vec::new();
                                                    self.event_res = String::new();
                                                    let _ =
                                                        self.loop_tx.clone().unwrap().send(true);
                                                }
                                            },
                                        );
                                    }
                                });
                            });
                        }
                    });
            });
        } else if self.start {
            egui::CentralPanel::default().frame(background).show(ctx, |ui| {
                render_header(ui);
                ui.label(egui::RichText::new("I see you have found yourself on the depths of the ocean. You must be here to manage the lawless lifeforms that call this place home. No doubt you posses the skills needed to make them thrive. When you are ready to begin, click play.").font(egui::FontId::proportional(20.0)).color(egui::Color32::from_rgb(10, 10, 10)));
                ui.label("");
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    let enter = ui.add(egui::Button::new(egui::RichText::new("Play").font(egui::FontId::proportional(20.0))).min_size(egui::vec2(100.0, 30.0)).fill(egui::Color32::from_rgb(10,10,10)));
                    if enter.clicked() {
                        self.start = false;
                        self.get_dim = true;
                    }
                });
            });
        } else if self.get_dim {
            egui::CentralPanel::default()
                .frame(background)
                .show(ctx, |ui| {
                    let mut parse_err = false;
                    render_header(ui);
                    ui.label(
                        egui::RichText::new(
                            "First, provide the desired dimensions of your colony.",
                        )
                        .font(egui::FontId::proportional(20.0))
                        .color(egui::Color32::from_rgb(10, 10, 10)),
                    );
                    ui.add_space(5.0);
                    ui.horizontal(|ui| {
                        let row_label = ui.label(
                            egui::RichText::new("Rows: ")
                                .font(egui::FontId::proportional(20.0))
                                .color(egui::Color32::from_rgb(10, 10, 10)),
                        );
                        ui.text_edit_singleline(&mut self.first_input)
                            .labelled_by(row_label.id);
                    });
                    ui.horizontal(|ui| {
                        let col_label = ui.label(
                            egui::RichText::new("Columns: ")
                                .font(egui::FontId::proportional(20.0))
                                .color(egui::Color32::from_rgb(10, 10, 10)),
                        );
                        ui.text_edit_singleline(&mut self.second_input)
                            .labelled_by(col_label.id);
                    });
                    ui.label("");
                    ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                        let enter = ui.add(
                            egui::Button::new(
                                egui::RichText::new("Enter").font(egui::FontId::proportional(20.0)),
                            )
                            .min_size(egui::vec2(100.0, 30.0))
                            .fill(egui::Color32::from_rgb(10, 10, 10)),
                        );
                        if enter.clicked() {
                            match self.first_input.trim().parse::<f32>() {
                                Ok(row) => match self.second_input.trim().parse::<f32>() {
                                    Ok(col) => {
                                        // We need to store or pass the data
                                        self.game_info.push(row);
                                        self.game_info.push(col);
                                        self.first_input = String::new();
                                        self.second_input = String::new();
                                        self.get_dim = false;
                                        self.get_animals = true;
                                    }
                                    Err(_) => parse_err = true,
                                },
                                Err(_) => parse_err = true,
                            }
                        }
                    });
                    if parse_err {
                        ui.label("Dimensions must be positive integers.");
                    }
                });
        } else if self.get_animals {
            egui::CentralPanel::default()
                .frame(background)
                .show(ctx, |ui| {
                    let board_size = self.game_info[0] * self.game_info[1];
                    // Scale display size based on the number of rows
                    self.game_info.push(5.0 / self.game_info[0]);
                    render_header(ui);
                    ui.label(
                        egui::RichText::new(
                            "Thank you...\nNow provide the starting animal populations",
                        )
                        .font(egui::FontId::proportional(20.0))
                        .color(egui::Color32::from_rgb(10, 10, 10)),
                    );
                    ui.add_space(5.0);
                    ui.horizontal(|ui| {
                        let row_label = ui.label(
                            egui::RichText::new(format!(
                                "Fish 🐠 (limit {}): ",
                                board_size as usize / 5
                            ))
                            .font(egui::FontId::proportional(20.0))
                            .color(egui::Color32::from_rgb(10, 10, 10)),
                        );
                        ui.text_edit_singleline(&mut self.first_input)
                            .labelled_by(row_label.id);
                    });
                    ui.horizontal(|ui| {
                        let row_label = ui.label(
                            egui::RichText::new(format!(
                                "Crab 🐚 (limit {}): ",
                                board_size as usize / 7
                            ))
                            .font(egui::FontId::proportional(20.0))
                            .color(egui::Color32::from_rgb(10, 10, 10)),
                        );
                        ui.text_edit_singleline(&mut self.second_input)
                            .labelled_by(row_label.id);
                    });
                    ui.horizontal(|ui| {
                        let row_label = ui.label(
                            egui::RichText::new(format!(
                                "Shark 🐬 (limit {}): ",
                                board_size as usize / 10
                            ))
                            .font(egui::FontId::proportional(20.0))
                            .color(egui::Color32::from_rgb(10, 10, 10)),
                        );
                        ui.text_edit_singleline(&mut self.third_input)
                            .labelled_by(row_label.id);
                    });
                    ui.label("");
                    ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                        let enter = ui.add(
                            egui::Button::new(
                                egui::RichText::new("Enter").font(egui::FontId::proportional(20.0)),
                            )
                            .min_size(egui::vec2(100.0, 30.0))
                            .fill(egui::Color32::from_rgb(10, 10, 10)),
                        );
                        if enter.clicked() {
                            match self.first_input.trim().parse::<usize>() {
                                Ok(fish) => match self.second_input.trim().parse::<usize>() {
                                    Ok(crab) => match self.third_input.trim().parse::<usize>() {
                                        Ok(shark) => {
                                            if fish > board_size as usize / 5
                                                || crab > board_size as usize / 7
                                                || shark > board_size as usize / 10
                                            {
                                                self.event_res = String::from(
                                                    "Values must be less than the limit!",
                                                );
                                            } else {
                                                game_data::initialize_board(
                                                    self.game_info[0] as usize,
                                                    self.game_info[1] as usize,
                                                    fish,
                                                    crab,
                                                    shark,
                                                    self.tx.clone(),
                                                    ctx.clone(),
                                                );
                                                self.event_res = String::new();
                                                self.get_animals = false;
                                                self.run_simulation = true;
                                            }
                                        }
                                        Err(_) => {
                                            self.event_res =
                                                String::from("Input must be positive number!")
                                        }
                                    },
                                    Err(_) => {
                                        self.event_res =
                                            String::from("Input must be positive number!")
                                    }
                                },
                                Err(_) => {
                                    self.event_res = String::from("Input must be positive number!")
                                }
                            }
                        }
                    });
                    if !self.event_res.is_empty() {
                        ui.label(
                            egui::RichText::new(self.event_res.clone())
                                .font(egui::FontId::proportional(20.0))
                                .color(egui::Color32::from_rgb(10, 10, 10)),
                        );
                    }
                });
        }
    }
}

fn render_header(ui: &mut egui::Ui) {
    ui.vertical_centered(|ui| {
        ui.heading(
            egui::RichText::new("Deep Sea Adventure")
                .color(egui::Color32::from_rgb(10, 10, 10))
                .font(egui::FontId::proportional(20.0)),
        );
    });
    ui.add_space(5.0);
    let sep = egui::Separator::default().spacing(20.0);
    ui.add(sep);
}

pub fn init() {
    let options = eframe::NativeOptions {
        initial_window_size: Some(Vec2::new(1410.0, 810.0)),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "Sea Simulation",
        options,
        Box::new(|cc| Box::new(SeaGui::new(cc))),
    );
}
