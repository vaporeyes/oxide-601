// ABOUTME: User interface and viewport renderer for the Minivac 601 emulator.
// ABOUTME: Implements custom hardware graphics, drag-and-drop cables, and the experiment manual.

use crate::simulation::{ComponentPort, MinivacSimulation};
use egui::epaint::{CubicBezierShape, Stroke};
use egui::{Color32, Pos2, Rect, Shape, Vec2};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DragState {
    None,
    Dragging {
        from_port: Uuid,
        from_hole: usize,
        color: [u8; 4],
    },
}

pub struct MinivacApp {
    pub sim: MinivacSimulation,
    pub port_positions: HashMap<(Uuid, usize), Pos2>,
    pub drag_state: DragState,
    pub active_color: [u8; 4],
    pub selected_experiment: usize,
    pub show_guides: bool,
    pub hovered_hole: Option<(Uuid, usize)>,
}

impl Default for MinivacApp {
    fn default() -> Self {
        Self {
            sim: MinivacSimulation::new(),
            port_positions: HashMap::new(),
            drag_state: DragState::None,
            active_color: [230, 30, 30, 255], // Red default
            selected_experiment: 0,
            show_guides: false,
            hovered_hole: None,
        }
    }
}

struct Experiment {
    name: &'static str,
    desc: &'static str,
    wiring: &'static [(&'static str, usize, &'static str, usize)], // (port_a, hole_a, port_b, hole_b)
    help_text: &'static str,
}

const EXPERIMENTS: &[Experiment] = &[
    Experiment {
        name: "Exp 1: Switch and Light",
        desc: "Turn on Light 1 using Slide Switch 1.",
        wiring: &[
            ("+0", 0, "SW1_S", 0),
            ("SW1_R", 0, "LGT1_A", 0),
            ("LGT1_B", 0, "-0", 0),
        ],
        help_text: "1. Connect Power +0 to Switch 1 Common (SW1_S).\n2. Connect Switch 1 Left Contact (SW1_R) to Light 1 Terminal A (LGT1_A).\n3. Connect Light 1 Terminal B (LGT1_B) to Power -0.\n4. Turn on the Power switch. Move Slide Switch 1 LEFT to turn the light ON.",
    },
    Experiment {
        name: "Exp 2: Pushbutton and Relay",
        desc: "Press Pushbutton 1 to energize Relay 1 and turn on its light.",
        wiring: &[
            ("+0", 0, "PB1_Y", 0),
            ("PB1_X", 0, "RLY1_coil_C", 0),
            ("RLY1_coil_F", 0, "-0", 0),
        ],
        help_text: "1. Connect Power +0 to Pushbutton 1 Common (PB1_Y).\n2. Connect Pushbutton 1 Normally Open (PB1_X) to Relay 1 Series Coil (RLY1_coil_C).\n3. Connect Relay 1 Coil Ground (RLY1_coil_F) to Power -0.\n4. Turn on Power. Press and hold Pushbutton 1 down. The relay will click ON and its light will illuminate.",
    },
    Experiment {
        name: "Exp 3: Self-Latching Relay (Memory)",
        desc: "Press Button 1 to lock Relay 1 ON. Press Button 2 to unlock it.",
        wiring: &[
            ("+0", 0, "PB2_Y", 0),
            ("PB2_Z", 0, "PB1_Y", 0),
            ("PB2_Z", 1, "RLY1_contact_H", 0),
            ("PB1_X", 0, "RLY1_coil_C", 0),
            ("RLY1_contact_G", 0, "RLY1_coil_C", 1),
            ("RLY1_coil_F", 0, "-0", 0),
        ],
        help_text: "1. Connect +0 to PB2 Common (PB2_Y).\n2. Connect PB2 Normally Closed (PB2_Z) to PB1 Common (PB1_Y).\n3. Connect PB2 Normally Closed (PB2_Z, second hole) to Relay 1 Contact Common (RLY1_contact_H).\n4. Connect PB1 Normally Open (PB1_X) to Relay 1 Series Coil (RLY1_coil_C).\n5. Connect Relay 1 Contact Normally Open (RLY1_contact_G) to Relay 1 Series Coil (RLY1_coil_C).\n6. Connect Relay 1 Coil Ground (RLY1_coil_F) to -0.\n7. Turn on Power. Tap Button 1. The relay clicks and locks ON. Tap Button 2. The relay unlocks and turns OFF.",
    },
    Experiment {
        name: "Exp 4: Relay Buzzer (Oscillator)",
        desc: "Connect a relay to its own contact to create an oscillator.",
        wiring: &[
            ("+0", 0, "RLY1_contact_H", 0),
            ("RLY1_contact_J", 0, "RLY1_coil_C", 0),
            ("RLY1_coil_F", 0, "-0", 0),
        ],
        help_text: "1. Connect Power +0 to Relay 1 Contact Common (RLY1_contact_H).\n2. Connect Relay 1 Normally Closed contact (RLY1_contact_J) to Relay 1 Series Coil (RLY1_coil_C).\n3. Connect Relay 1 Coil Ground (RLY1_coil_F) to Power -0.\n4. Turn on Power. The relay will oscillate rapidly, creating a clicking animation and flashing light.",
    },
    Experiment {
        name: "Exp 5: Motorized Dial Clock Generator",
        desc: "Make the dial motor run continuously and flash Light 1 as it rotates.",
        wiring: &[
            ("+0", 0, "M17", 0),
            ("M18", 0, "-0", 0),
            ("+0", 1, "D16", 0),
            ("D0", 0, "LGT1_A", 0),
            ("D8", 0, "LGT1_A", 1),
            ("LGT1_B", 0, "-0", 1),
        ],
        help_text: "1. Connect +0 to Motor RUN 17 (M17).\n2. Connect Motor RUN 18 (M18) to -0. (This turns on the motor).\n3. Connect +0 to Dial Wiper (D16).\n4. Connect Dial Contacts D0 and D8 to Light 1 A (LGT1_A).\n5. Connect Light 1 B (LGT1_B) to -0.\n6. Turn on Power. The dial will rotate, and Light 1 will flash twice per revolution.",
    },
];

fn get_port_uuid_by_spec(spec: &str, port_to_uuid: &HashMap<ComponentPort, Uuid>) -> Option<Uuid> {
    // Map string representation from experiments to logical ComponentPorts
    let port = if spec.starts_with("+") {
        let idx = spec[1..].parse::<usize>().unwrap_or(0);
        ComponentPort::PowerPlus(idx)
    } else if spec.starts_with("-") {
        let idx = spec[1..].parse::<usize>().unwrap_or(0);
        ComponentPort::PowerMinus(idx)
    } else if spec.starts_with("PB") {
        let num = spec[2..3].parse::<usize>().unwrap_or(1) - 1;
        let term = spec.chars().nth(4).unwrap_or('Y');
        ComponentPort::Pushbutton(num, term)
    } else if spec.starts_with("SW") {
        let num = spec[2..3].parse::<usize>().unwrap_or(1) - 1;
        let term = spec.chars().nth(4).unwrap_or('S');
        ComponentPort::SlideSwitch(num, term)
    } else if spec.starts_with("RLY") {
        let num = spec[3..4].parse::<usize>().unwrap_or(1) - 1;
        let is_coil = spec.contains("coil");
        let term = spec.chars().last().unwrap_or('E');
        if is_coil {
            ComponentPort::RelayCoil(num, term)
        } else {
            ComponentPort::RelayContact(num, term)
        }
    } else if spec.starts_with("LGT") {
        let num = spec[3..4].parse::<usize>().unwrap_or(1) - 1;
        let term = spec.chars().last().unwrap_or('A');
        ComponentPort::BinaryLight(num, term)
    } else if spec.starts_with("D16") {
        ComponentPort::DialWiper
    } else if spec.starts_with("D") {
        let num = spec[1..].parse::<usize>().unwrap_or(0);
        ComponentPort::DialContact(num)
    } else if spec.starts_with("MX") {
        // Must be checked before the bare "M" (motor) branch, otherwise "MX.."
        // matches "M" first and never resolves to a matrix point.
        let num = spec[2..3].parse::<usize>().unwrap_or(1) - 1;
        let term = spec.chars().last().unwrap_or('t');
        ComponentPort::MatrixPoint(num, term)
    } else if spec.starts_with("M") {
        let num = spec[1..].parse::<usize>().unwrap_or(17);
        ComponentPort::DialMotor(num)
    } else if spec.starts_with("COM") {
        let num = spec[3..4].parse::<usize>().unwrap_or(1) - 1;
        let hole = spec.chars().last().unwrap_or('0').to_digit(10).unwrap_or(0) as usize;
        ComponentPort::TiePoint(num, hole)
    } else {
        return None;
    };

    port_to_uuid.get(&port).copied()
}

// Helper struct to allow disjoint borrowing of fields during rendering
struct DrawContext<'a> {
    ui: &'a mut egui::Ui,
    painter: &'a egui::Painter,
    port_positions: &'a mut HashMap<(Uuid, usize), Pos2>,
    drag_state: &'a mut DragState,
    active_color: [u8; 4],
    hovered_hole: &'a mut Option<(Uuid, usize)>,
    connections: &'a mut Vec<crate::simulation::Connection>,
    port_to_uuid: &'a HashMap<ComponentPort, Uuid>,
}

impl<'a> DrawContext<'a> {
    fn draw_custom_port(&mut self, center: Pos2, port_uuid: Uuid, label: &str) {
        let hole_spacing = 8.0;
        let h0 = center + Vec2::new(-hole_spacing / 2.0, 0.0);
        let h1 = center + Vec2::new(hole_spacing / 2.0, 0.0);

        // Draw rivet plate background
        let plate_rect = Rect::from_center_size(center, Vec2::new(24.0, 14.0));
        self.painter.rect_filled(plate_rect, 2.0, Color32::from_gray(175));
        self.painter.rect_stroke(plate_rect, 2.0, Stroke::new(1.0, Color32::from_gray(110)));

        // Draw the two holes with brass rims
        self.painter.circle_filled(h0, 3.5, Color32::from_rgb(20, 20, 20));
        self.painter.circle_filled(h1, 3.5, Color32::from_rgb(20, 20, 20));
        self.painter.circle_stroke(h0, 3.5, Stroke::new(0.8, Color32::from_rgb(190, 150, 40)));
        self.painter.circle_stroke(h1, 3.5, Stroke::new(0.8, Color32::from_rgb(190, 150, 40)));

        // Label above or below
        self.painter.text(
            center - Vec2::new(0.0, 11.0),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(10.0),
            Color32::from_gray(220),
        );

        // Handle interactions for Hole 0 and Hole 1
        self.handle_hole(h0, port_uuid, 0);
        self.handle_hole(h1, port_uuid, 1);
    }

    fn handle_hole(&mut self, pos: Pos2, port_uuid: Uuid, hole_idx: usize) {
        self.port_positions.insert((port_uuid, hole_idx), pos);

        let id = self.ui.make_persistent_id((port_uuid, hole_idx));
        let hole_rect = Rect::from_center_size(pos, Vec2::new(12.0, 12.0));
        let response = self.ui.interact(hole_rect, id, egui::Sense::click_and_drag());

        // Check if occupied
        let mut occupied_idx = None;
        for (idx, conn) in self.connections.iter().enumerate() {
            if (conn.from == port_uuid && conn.from_hole == hole_idx)
                || (conn.to == port_uuid && conn.to_hole == hole_idx)
            {
                occupied_idx = Some(idx);
                break;
            }
        }

        if response.hovered() {
            *self.hovered_hole = Some((port_uuid, hole_idx));
            self.painter.circle_stroke(pos, 7.0, Stroke::new(1.5, Color32::from_rgb(255, 235, 59)));
        }

        if response.drag_started() {
            if let Some(idx) = occupied_idx {
                // Unplug existing
                let conn = self.connections.remove(idx);
                if conn.from == port_uuid && conn.from_hole == hole_idx {
                    *self.drag_state = DragState::Dragging {
                        from_port: conn.to,
                        from_hole: conn.to_hole,
                        color: conn.color,
                    };
                } else {
                    *self.drag_state = DragState::Dragging {
                        from_port: conn.from,
                        from_hole: conn.from_hole,
                        color: conn.color,
                    };
                }
            } else {
                // New connection
                *self.drag_state = DragState::Dragging {
                    from_port: port_uuid,
                    from_hole: hole_idx,
                    color: self.active_color,
                };
            }
        }
    }
}

impl eframe::App for MinivacApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Only drive continuous redraws when something is actually animating
        // (relays transitioning, motor spinning, lights/coils settling). When the
        // power is off, egui still repaints on input events, so interaction stays
        // responsive without pinning the CPU/GPU at the full frame rate while idle.
        if self.sim.power_on {
            ctx.request_repaint();
        }

        // Apply a premium dark mode theme
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = Color32::from_rgb(18, 20, 24);
        visuals.window_fill = Color32::from_rgb(18, 20, 24);
        visuals.widgets.active.bg_fill = Color32::from_rgb(40, 50, 70);
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(30, 40, 60);
        visuals.widgets.inactive.bg_fill = Color32::from_rgb(24, 28, 36);
        ctx.set_visuals(visuals);

        let dt = ctx.input(|i| i.stable_dt).min(0.1);
        self.sim.tick(dt);

        self.hovered_hole = None;

        // Draw Left Sidebar Panel
        egui::SidePanel::left("sidebar")
            .resizable(false)
            .default_width(320.0)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("MINIVAC 601 MANUAL");
                });
                ui.separator();

                // System status
                ui.group(|ui| {
                    ui.label("System Power Status:");
                    ui.horizontal(|ui| {
                        if self.sim.breaker_tripped {
                            ui.colored_label(Color32::RED, "BREAKER TRIPPED!");
                            if ui.button("Reset Breaker").clicked() {
                                self.sim.breaker_tripped = false;
                            }
                        } else if !self.sim.power_on {
                            ui.colored_label(Color32::from_gray(120), "Power Supply: OFF");
                        } else {
                            ui.colored_label(Color32::GREEN, "Power Supply: OK (12V DC)");
                        }
                    });
                });

                ui.add_space(5.0);

                // Wire tools
                ui.group(|ui| {
                    ui.label("Cable Patch Color:");
                    ui.horizontal(|ui| {
                        let colors = [
                            ("Red", [230, 30, 30, 255]),
                            ("Blue", [30, 80, 230, 255]),
                            ("Yellow", [230, 180, 30, 255]),
                            ("Green", [30, 180, 50, 255]),
                            ("White", [235, 235, 235, 255]),
                            ("Black", [25, 25, 25, 255]),
                        ];
                        for (name, c_arr) in colors {
                            let is_active = self.active_color == c_arr;
                            let color32 = Color32::from_rgba_unmultiplied(
                                c_arr[0],
                                c_arr[1],
                                c_arr[2],
                                c_arr[3],
                            );
                            ui.horizontal(|ui| {
                                let (rect, _response) = ui.allocate_exact_size(Vec2::new(12.0, 12.0), egui::Sense::hover());
                                ui.painter().circle_filled(rect.center(), 5.0, color32);
                                ui.painter().circle_stroke(rect.center(), 5.0, Stroke::new(1.0, Color32::from_gray(120)));
                                if ui.selectable_label(is_active, name).clicked() {
                                    self.active_color = c_arr;
                                }
                            });
                        }
                    });
                    ui.add_space(3.0);
                    ui.horizontal(|ui| {
                        if ui.button("Clear All Wires").clicked() {
                            self.sim.connections.clear();
                        }
                        if ui.button("Reset Rotary Dial").clicked() {
                            self.sim.dial_position = 0.0;
                        }
                    });
                });

                ui.add_space(5.0);

                // Experimenter's manual
                ui.group(|ui| {
                    ui.label("Experiment Manual:");
                    egui::ComboBox::from_label("")
                        .selected_text(EXPERIMENTS[self.selected_experiment].name)
                        .show_ui(ui, |ui| {
                            for (idx, exp) in EXPERIMENTS.iter().enumerate() {
                                ui.selectable_value(&mut self.selected_experiment, idx, exp.name);
                            }
                        });

                    let exp = &EXPERIMENTS[self.selected_experiment];
                    ui.add_space(3.0);
                    ui.small(exp.desc);
                    ui.separator();
                    ui.label("Wiring Instructions:");
                    ui.add(egui::Label::new(exp.help_text).wrap(true));

                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.show_guides, "Show Guide Lines");
                        if ui.button("Auto-Wire").clicked() {
                            self.sim.connections.clear();
                            for &(src_name, src_hole, dst_name, dst_hole) in exp.wiring {
                                if let (Some(from_u), Some(to_u)) = (
                                    get_port_uuid_by_spec(src_name, &self.sim.port_to_uuid),
                                    get_port_uuid_by_spec(dst_name, &self.sim.port_to_uuid),
                                ) {
                                    self.sim.add_connection(from_u, src_hole, to_u, dst_hole, self.active_color);
                                }
                            }
                        }
                    });
                });
            });

        // Draw Central Panel for Minivac Faceplate
        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                let (response, painter) =
                    ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());

                let canvas_rect = Rect::from_min_size(response.rect.min, Vec2::new(1000.0, 780.0));

                // 3D wood case frame border bevels
                painter.rect_filled(canvas_rect, 12.0, Color32::from_rgb(78, 52, 46)); // Lighter outer mahogany wood
                painter.rect_filled(canvas_rect.shrink(4.0), 8.0, Color32::from_rgb(62, 39, 35)); // Medium mahogany
                painter.rect_filled(canvas_rect.shrink(8.0), 4.0, Color32::from_rgb(46, 26, 21)); // Dark shadow wood
                painter.rect_stroke(canvas_rect, 12.0, Stroke::new(1.0, Color32::from_rgb(40, 25, 20)));

                // Faceplate panel background
                let faceplate = canvas_rect.shrink(12.0);
                painter.rect_filled(faceplate, 4.0, Color32::from_rgb(25, 45, 75)); // Blue steel look

                // Silver screws at 4 corners with drop shadow
                let corners = [
                    faceplate.left_top() + Vec2::new(8.0, 8.0),
                    faceplate.right_top() + Vec2::new(-8.0, 8.0),
                    faceplate.left_bottom() + Vec2::new(8.0, -8.0),
                    faceplate.right_bottom() + Vec2::new(-8.0, -8.0),
                ];
                for screw_pos in corners {
                    painter.circle_filled(screw_pos + Vec2::new(1.0, 1.0), 5.0, Color32::from_rgba_unmultiplied(0, 0, 0, 120));
                    painter.circle_filled(screw_pos, 5.0, Color32::from_gray(180));
                    painter.circle_stroke(screw_pos, 5.0, Stroke::new(0.5, Color32::from_gray(120)));
                    painter.line_segment(
                        [screw_pos - Vec2::new(3.5, 0.0), screw_pos + Vec2::new(3.5, 0.0)],
                        Stroke::new(1.0, Color32::from_gray(60)),
                    );
                }

                // Section division lines
                let grid_stroke = Stroke::new(2.0, Color32::from_rgba_unmultiplied(60, 180, 220, 150)); // Bright cyan-blue lines like the original
                // Horizontal dividers for Left Pane
                painter.line_segment([faceplate.left_top() + Vec2::new(0.0, 130.0), faceplate.left_top() + Vec2::new(680.0, 130.0)], grid_stroke);
                painter.line_segment([faceplate.left_top() + Vec2::new(0.0, 340.0), faceplate.left_top() + Vec2::new(680.0, 340.0)], grid_stroke);
                painter.line_segment([faceplate.left_top() + Vec2::new(0.0, 390.0), faceplate.left_top() + Vec2::new(680.0, 390.0)], grid_stroke);
                painter.line_segment([faceplate.left_top() + Vec2::new(0.0, 550.0), faceplate.left_top() + Vec2::new(680.0, 550.0)], grid_stroke);

                // Vertical divider separating Left and Right panes
                painter.line_segment(
                    [faceplate.left_top() + Vec2::new(680.0, 0.0), faceplate.left_top() + Vec2::new(680.0, faceplate.height())],
                    grid_stroke,
                );
                // Horizontal divider in Right Pane (between Matrix and Dial)
                painter.line_segment([faceplate.left_top() + Vec2::new(680.0, 440.0), faceplate.right_top() + Vec2::new(0.0, 440.0)], grid_stroke);

                // Title banner text (Top Right)
                painter.text(
                    faceplate.left_top() + Vec2::new(820.0, 40.0),
                    egui::Align2::CENTER_CENTER,
                    "Minivac 601",
                    egui::FontId::proportional(32.0),
                    Color32::from_gray(245),
                );
                painter.text(
                    faceplate.left_top() + Vec2::new(820.0, 65.0),
                    egui::Align2::CENTER_CENTER,
                    "SCIENTIFIC DEVELOPMENT CORPORATION",
                    egui::FontId::proportional(8.0),
                    Color32::from_gray(200),
                );

                let offset = faceplate.min;

                // Create draw context to handle split mutable borrows cleanly
                let mut ctx_draw = DrawContext {
                    ui,
                    painter: &painter,
                    port_positions: &mut self.port_positions,
                    drag_state: &mut self.drag_state,
                    active_color: self.active_color,
                    hovered_hole: &mut self.hovered_hole,
                    connections: &mut self.sim.connections,
                    port_to_uuid: &self.sim.port_to_uuid,
                };

                // ------------------ 1. POWER PANEL ------------------
                let power_panel_center = offset + Vec2::new(920.0, 120.0); // Right side, below logo
                ctx_draw.painter.text(
                    power_panel_center - Vec2::new(0.0, 30.0),
                    egui::Align2::CENTER_CENTER,
                    "POWER",
                    egui::FontId::proportional(10.0),
                    Color32::from_gray(220),
                );

                // Circuit breaker button
                let breaker_rect = Rect::from_center_size(power_panel_center + Vec2::new(0.0, 10.0), Vec2::new(22.0, 22.0));
                let breaker_id = ctx_draw.ui.make_persistent_id("breaker_btn");
                let breaker_resp = ctx_draw.ui.interact(breaker_rect, breaker_id, egui::Sense::click());
                if breaker_resp.clicked() && self.sim.breaker_tripped {
                    self.sim.breaker_tripped = false;
                }
                ctx_draw.painter.circle_filled(breaker_rect.center(), 10.0, Color32::from_gray(40));
                if self.sim.breaker_tripped {
                    ctx_draw.painter.circle_filled(breaker_rect.center(), 7.0, Color32::from_gray(190));
                    ctx_draw.painter.circle_filled(breaker_rect.center(), 5.0, Color32::RED);
                } else {
                    ctx_draw.painter.circle_filled(breaker_rect.center(), 7.0, Color32::from_rgb(120, 20, 20));
                }
                ctx_draw.painter.text(
                    breaker_rect.center() - Vec2::new(0.0, 18.0),
                    egui::Align2::CENTER_CENTER,
                    "CIRCUIT\nBREAKER",
                    egui::FontId::proportional(7.0),
                    Color32::from_gray(200),
                );

                // Main power toggle switch
                let pwr_switch_rect = Rect::from_center_size(power_panel_center + Vec2::new(0.0, 55.0), Vec2::new(20.0, 30.0));
                let pwr_switch_id = ctx_draw.ui.make_persistent_id("power_toggle");
                let pwr_resp = ctx_draw.ui.interact(pwr_switch_rect, pwr_switch_id, egui::Sense::click());
                if pwr_resp.clicked() {
                    self.sim.power_on = !self.sim.power_on;
                }
                ctx_draw.painter.rect_filled(pwr_switch_rect, 2.0, Color32::from_gray(40));
                ctx_draw.painter.rect_stroke(pwr_switch_rect, 2.0, Stroke::new(1.0, Color32::from_gray(100)));
                let toggle_lever_center = if self.sim.power_on {
                    pwr_switch_rect.center() - Vec2::new(0.0, 6.0)
                } else {
                    pwr_switch_rect.center() + Vec2::new(0.0, 6.0)
                };
                ctx_draw.painter.circle_filled(toggle_lever_center, 5.0, Color32::from_gray(180));
                ctx_draw.painter.line_segment(
                    [pwr_switch_rect.center(), toggle_lever_center],
                    Stroke::new(2.5, Color32::from_gray(140)),
                );
                ctx_draw.painter.text(
                    pwr_switch_rect.center() - Vec2::new(0.0, 22.0),
                    egui::Align2::CENTER_CENTER,
                    "ON",
                    egui::FontId::proportional(8.0),
                    Color32::from_gray(200),
                );
                ctx_draw.painter.text(
                    pwr_switch_rect.center() + Vec2::new(0.0, 22.0),
                    egui::Align2::CENTER_CENTER,
                    "OFF",
                    egui::FontId::proportional(8.0),
                    Color32::from_gray(200),
                );

                // Power ports + and -
                for i in 0..4 {
                    // Arrange in a small 2x2 grid to the left of the switch
                    let col = i % 2;
                    let row = i / 2;
                    let px = offset.x + 800.0 + (col as f32 * 40.0);
                    let py = offset.y + 120.0 + (row as f32 * 45.0);
                    
                    let plus_uuid = self.sim.port_to_uuid[&ComponentPort::PowerPlus(i)];
                    ctx_draw.draw_custom_port(Pos2::new(px, py), plus_uuid, "+");

                    let minus_uuid = self.sim.port_to_uuid[&ComponentPort::PowerMinus(i)];
                    ctx_draw.draw_custom_port(Pos2::new(px, py + 20.0), minus_uuid, "-");
                }

                // ------------------ 2. MATRIX (MIDDLE RIGHT) ------------------
                ctx_draw.painter.text(
                    offset + Vec2::new(710.0, 220.0),
                    egui::Align2::LEFT_CENTER,
                    "MATRIX",
                    egui::FontId::proportional(10.0),
                    Color32::from_gray(220),
                );
                for i in 0..11 {
                    // Arrange 3x4 grid on the right side
                    let col = i % 3;
                    let row = i / 3;
                    let mx_x = offset.x + 740.0 + (col as f32 * 75.0);
                    let mx_y = offset.y + 250.0 + (row as f32 * 45.0);

                    // Draw outline grouping box
                    let box_rect = Rect::from_center_size(Pos2::new(mx_x, mx_y), Vec2::new(60.0, 36.0));
                    ctx_draw.painter.rect_stroke(box_rect, 2.0, Stroke::new(1.0, Color32::from_rgba_unmultiplied(200, 200, 255, 20)));
                    ctx_draw.painter.text(
                        box_rect.left_top() + Vec2::new(5.0, 5.0),
                        egui::Align2::LEFT_TOP,
                        format!("{}", i + 1),
                        egui::FontId::proportional(8.0),
                        Color32::from_gray(180),
                    );

                    let t_uuid = self.sim.port_to_uuid[&ComponentPort::MatrixPoint(i, 't')];
                    ctx_draw.draw_custom_port(Pos2::new(mx_x - 12.0, mx_y + 4.0), t_uuid, "");

                    let b_uuid = self.sim.port_to_uuid[&ComponentPort::MatrixPoint(i, 'b')];
                    ctx_draw.draw_custom_port(Pos2::new(mx_x + 12.0, mx_y + 4.0), b_uuid, "");
                }

                // ------------------ 3. BINARY OUTPUT LIGHTS ------------------
                ctx_draw.painter.text(
                    offset + Vec2::new(340.0, 18.0),
                    egui::Align2::CENTER_CENTER,
                    "BINARY OUTPUT",
                    egui::FontId::proportional(12.0),
                    Color32::from_gray(220),
                );
                for c in 0..6 {
                    let center_x = 90.0 + c as f32 * 100.0;
                    let light_center = offset + Vec2::new(center_x, 60.0);

                    // Pilot Light metallic bezel
                    ctx_draw.painter.circle_stroke(light_center, 14.5, Stroke::new(2.0, Color32::from_gray(160))); // Chrome ring
                    ctx_draw.painter.circle_filled(light_center, 13.0, Color32::from_gray(40)); // Inner housing

                    // Draw Light Bulb Lens
                    let b = self.sim.light_brightness[c];
                    let glow_color = if b > 0.0 {
                        Color32::from_rgb(
                            (255.0 * b).max(180.0) as u8,
                            (200.0 * b).max(100.0) as u8,
                            (40.0 * b) as u8,
                        )
                    } else {
                        Color32::from_rgb(80, 40, 20) // unlit dark glass
                    };
                    ctx_draw.painter.circle_filled(light_center, 11.0, glow_color);
                    ctx_draw.painter.circle_stroke(light_center, 11.0, Stroke::new(1.0, Color32::from_gray(50)));
                    if b > 0.0 {
                        // Glare and faceted pilot lamp jewel pattern
                        ctx_draw.painter.circle_stroke(light_center, 7.5, Stroke::new(0.8, Color32::from_rgba_unmultiplied(255, 255, 255, 100)));
                        ctx_draw.painter.circle_filled(light_center - Vec2::new(3.0, 3.0), 2.5, Color32::from_rgba_unmultiplied(255, 255, 255, 140));
                    }

                    // LGT Ports A and B
                    let a_uuid = self.sim.port_to_uuid[&ComponentPort::BinaryLight(c, 'A')];
                    ctx_draw.draw_custom_port(offset + Vec2::new(center_x - 16.0, 100.0), a_uuid, "A");

                    let b_uuid = self.sim.port_to_uuid[&ComponentPort::BinaryLight(c, 'B')];
                    ctx_draw.draw_custom_port(offset + Vec2::new(center_x + 16.0, 100.0), b_uuid, "B");

                    // Label numbers
                    ctx_draw.painter.text(
                        light_center - Vec2::new(0.0, 22.0),
                        egui::Align2::CENTER_CENTER,
                        format!("{}", c + 1),
                        egui::FontId::proportional(10.0),
                        Color32::from_gray(220),
                    );
                }

                // ------------------ 4. RELAYS ------------------
                ctx_draw.painter.text(
                    offset + Vec2::new(340.0, 142.0),
                    egui::Align2::CENTER_CENTER,
                    "STORAGE/PROCESSING",
                    egui::FontId::proportional(10.0),
                    Color32::from_gray(220),
                );
                for c in 0..6 {
                    let center_x = 90.0 + c as f32 * 100.0;

                    // C, E, F coil terminals
                    let c_uuid = self.sim.port_to_uuid[&ComponentPort::RelayCoil(c, 'C')];
                    ctx_draw.draw_custom_port(offset + Vec2::new(center_x - 22.0, 165.0), c_uuid, "C");

                    let e_uuid = self.sim.port_to_uuid[&ComponentPort::RelayCoil(c, 'E')];
                    ctx_draw.draw_custom_port(offset + Vec2::new(center_x, 165.0), e_uuid, "E");

                    let f_uuid = self.sim.port_to_uuid[&ComponentPort::RelayCoil(c, 'F')];
                    ctx_draw.draw_custom_port(offset + Vec2::new(center_x + 22.0, 165.0), f_uuid, "F");

                    // Relay Coil Indicator Light
                    let relay_light_center = offset + Vec2::new(center_x, 195.0);
                    let rb = self.sim.relay_light_brightness[c];
                    let r_glow = if rb > 0.0 {
                        Color32::from_rgb(
                            (255.0 * rb).max(180.0) as u8,
                            (160.0 * rb).max(80.0) as u8,
                            (30.0 * rb) as u8,
                        )
                    } else {
                        Color32::from_rgb(60, 20, 20)
                    };
                    ctx_draw.painter.circle_filled(relay_light_center, 6.0, r_glow);
                    ctx_draw.painter.circle_stroke(relay_light_center, 6.0, Stroke::new(1.0, Color32::from_gray(40)));

                    // Relay Body Casing Box
                    let relay_box = Rect::from_center_size(offset + Vec2::new(center_x, 245.0), Vec2::new(50.0, 68.0));
                    ctx_draw.painter.rect_filled(relay_box, 3.0, Color32::from_rgba_unmultiplied(210, 140, 50, 35));
                    ctx_draw.painter.rect_stroke(relay_box, 3.0, Stroke::new(1.5, Color32::from_rgba_unmultiplied(210, 140, 50, 130)));

                    // Copper Coil
                    let coil_rect = Rect::from_center_size(offset + Vec2::new(center_x, 235.0), Vec2::new(18.0, 32.0));
                    ctx_draw.painter.rect_filled(coil_rect, 2.0, Color32::from_rgb(180, 100, 50));
                    ctx_draw.painter.rect_stroke(coil_rect, 2.0, Stroke::new(1.0, Color32::from_rgb(120, 70, 30)));

                    // Metal Armature Line
                    let arm_x_offset = if self.sim.relay_progress[c] >= 0.9 {
                        5.0
                    } else if self.sim.relay_progress[c] <= 0.1 {
                        -5.0
                    } else {
                        0.0
                    };
                    let arm_start = offset + Vec2::new(center_x + arm_x_offset, 265.0);
                    let arm_end = offset + Vec2::new(center_x + arm_x_offset, 280.0);
                    ctx_draw.painter.line_segment([arm_start, arm_end], Stroke::new(3.0, Color32::from_gray(190)));

                    // Relay Contact terminals (G, H, J / K, L, N)
                    let g_uuid = self.sim.port_to_uuid[&ComponentPort::RelayContact(c, 'G')];
                    ctx_draw.draw_custom_port(offset + Vec2::new(center_x - 22.0, 305.0), g_uuid, "G");

                    let h_uuid = self.sim.port_to_uuid[&ComponentPort::RelayContact(c, 'H')];
                    ctx_draw.draw_custom_port(offset + Vec2::new(center_x, 305.0), h_uuid, "H");

                    let j_uuid = self.sim.port_to_uuid[&ComponentPort::RelayContact(c, 'J')];
                    ctx_draw.draw_custom_port(offset + Vec2::new(center_x + 22.0, 305.0), j_uuid, "J");

                    let k_uuid = self.sim.port_to_uuid[&ComponentPort::RelayContact(c, 'K')];
                    ctx_draw.draw_custom_port(offset + Vec2::new(center_x - 22.0, 335.0), k_uuid, "K");

                    let l_uuid = self.sim.port_to_uuid[&ComponentPort::RelayContact(c, 'L')];
                    ctx_draw.draw_custom_port(offset + Vec2::new(center_x, 335.0), l_uuid, "L");

                    let n_uuid = self.sim.port_to_uuid[&ComponentPort::RelayContact(c, 'N')];
                    ctx_draw.draw_custom_port(offset + Vec2::new(center_x + 22.0, 335.0), n_uuid, "N");
                }

                // ------------------ 5. TIE-POINTS (1com..6com) ------------------
                for c in 0..6 {
                    let center_x = 90.0 + c as f32 * 100.0;
                    
                    ctx_draw.painter.text(
                        offset + Vec2::new(center_x, 350.0),
                        egui::Align2::CENTER_CENTER,
                        "COMMON",
                        egui::FontId::proportional(8.0),
                        Color32::from_gray(160),
                    );

                    let com_box = Rect::from_center_size(offset + Vec2::new(center_x, 365.0), Vec2::new(54.0, 16.0));
                    ctx_draw.painter.rect_stroke(com_box, 2.0, Stroke::new(1.0, Color32::from_rgba_unmultiplied(200, 200, 255, 20)));

                    for h in 0..4 {
                        let hole_x = center_x - 18.0 + (h as f32 * 12.0);
                        let com_uuid = self.sim.port_to_uuid[&ComponentPort::TiePoint(c, h)];

                        // Draw simple independent ports for the common block
                        let hole_pos = offset + Vec2::new(hole_x, 365.0);
                        ctx_draw.painter.circle_filled(hole_pos, 3.5, Color32::from_rgb(15, 15, 15));
                        ctx_draw.painter.circle_stroke(hole_pos, 3.5, Stroke::new(1.0, Color32::from_gray(120)));
                        ctx_draw.handle_hole(hole_pos, com_uuid, 0);
                    }
                }

                // ------------------ 6. SLIDE SWITCHES ------------------
                ctx_draw.painter.text(
                    offset + Vec2::new(340.0, 400.0),
                    egui::Align2::CENTER_CENTER,
                    "SECONDARY STORAGE",
                    egui::FontId::proportional(10.0),
                    Color32::from_gray(220),
                );
                for c in 0..6 {
                    let center_x = 90.0 + c as f32 * 100.0;

                    // R, S, T / U, V, W terminals
                    let r_uuid = self.sim.port_to_uuid[&ComponentPort::SlideSwitch(c, 'R')];
                    ctx_draw.draw_custom_port(offset + Vec2::new(center_x - 22.0, 430.0), r_uuid, "R");

                    let s_uuid = self.sim.port_to_uuid[&ComponentPort::SlideSwitch(c, 'S')];
                    ctx_draw.draw_custom_port(offset + Vec2::new(center_x, 430.0), s_uuid, "S");

                    let t_uuid = self.sim.port_to_uuid[&ComponentPort::SlideSwitch(c, 'T')];
                    ctx_draw.draw_custom_port(offset + Vec2::new(center_x + 22.0, 430.0), t_uuid, "T");

                    let u_uuid = self.sim.port_to_uuid[&ComponentPort::SlideSwitch(c, 'U')];
                    ctx_draw.draw_custom_port(offset + Vec2::new(center_x - 22.0, 465.0), u_uuid, "U");

                    let v_uuid = self.sim.port_to_uuid[&ComponentPort::SlideSwitch(c, 'V')];
                    ctx_draw.draw_custom_port(offset + Vec2::new(center_x, 465.0), v_uuid, "V");

                    let w_uuid = self.sim.port_to_uuid[&ComponentPort::SlideSwitch(c, 'W')];
                    ctx_draw.draw_custom_port(offset + Vec2::new(center_x + 22.0, 465.0), w_uuid, "W");

                    // Slide switch switchplate bezel
                    let plate_rect = Rect::from_center_size(offset + Vec2::new(center_x, 505.0), Vec2::new(48.0, 24.0));
                    ctx_draw.painter.rect_filled(plate_rect, 2.0, Color32::from_gray(150));
                    ctx_draw.painter.rect_stroke(plate_rect, 2.0, Stroke::new(1.0, Color32::from_gray(95)));

                    // Slide switch lever slot
                    let slot_center = offset + Vec2::new(center_x, 505.0);
                    let slot_rect = Rect::from_center_size(slot_center, Vec2::new(36.0, 14.0));
                    let slot_id = ctx_draw.ui.make_persistent_id(("switch_lever", c));
                    let slot_resp = ctx_draw.ui.interact(slot_rect, slot_id, egui::Sense::click());
                    if slot_resp.clicked() {
                        self.sim.switches_right[c] = !self.sim.switches_right[c];
                    }

                    // Draw track
                    ctx_draw.painter.rect_filled(slot_rect, 1.0, Color32::from_gray(30));

                    // Draw classic black slide knob with a white stripe
                    let knob_center = if self.sim.switches_right[c] {
                        slot_center + Vec2::new(11.0, 0.0)
                    } else {
                        slot_center - Vec2::new(11.0, 0.0)
                    };
                    let knob_rect = Rect::from_center_size(knob_center, Vec2::new(12.0, 16.0));
                    ctx_draw.painter.rect_filled(knob_rect, 1.0, Color32::from_gray(50));
                    ctx_draw.painter.rect_stroke(knob_rect, 1.0, Stroke::new(1.0, Color32::from_gray(25)));
                    ctx_draw.painter.line_segment(
                        [knob_center - Vec2::new(0.0, 6.0), knob_center + Vec2::new(0.0, 6.0)],
                        Stroke::new(1.5, Color32::WHITE),
                    );

                    ctx_draw.painter.text(
                        slot_center - Vec2::new(26.0, 0.0),
                        egui::Align2::RIGHT_CENTER,
                        "L",
                        egui::FontId::proportional(9.0),
                        Color32::from_gray(180),
                    );
                    ctx_draw.painter.text(
                        slot_center + Vec2::new(26.0, 0.0),
                        egui::Align2::LEFT_CENTER,
                        "R",
                        egui::FontId::proportional(9.0),
                        Color32::from_gray(180),
                    );
                }

                // ------------------ 7. PUSHBUTTONS ------------------
                ctx_draw.painter.text(
                    offset + Vec2::new(340.0, 550.0),
                    egui::Align2::CENTER_CENTER,
                    "BINARY INPUT",
                    egui::FontId::proportional(10.0),
                    Color32::from_gray(220),
                );
                for c in 0..6 {
                    let center_x = 90.0 + c as f32 * 100.0;

                    // X, Y, Z terminals
                    let x_uuid = self.sim.port_to_uuid[&ComponentPort::Pushbutton(c, 'X')];
                    ctx_draw.draw_custom_port(offset + Vec2::new(center_x - 22.0, 580.0), x_uuid, "X");

                    let y_uuid = self.sim.port_to_uuid[&ComponentPort::Pushbutton(c, 'Y')];
                    ctx_draw.draw_custom_port(offset + Vec2::new(center_x, 580.0), y_uuid, "Y");

                    let z_uuid = self.sim.port_to_uuid[&ComponentPort::Pushbutton(c, 'Z')];
                    ctx_draw.draw_custom_port(offset + Vec2::new(center_x + 22.0, 580.0), z_uuid, "Z");

                    // Pushbutton circular actuator
                    let button_center = offset + Vec2::new(center_x, 630.0);
                    let button_rect = Rect::from_center_size(button_center, Vec2::new(32.0, 32.0));
                    let button_id = ctx_draw.ui.make_persistent_id(("pushbutton", c));
                    let button_resp = ctx_draw.ui.interact(button_rect, button_id, egui::Sense::click_and_drag());

                    let is_pressed = button_resp.contains_pointer()
                        && ctx_draw.ui.input(|i| i.pointer.button_down(egui::PointerButton::Primary));
                    self.sim.buttons_pressed[c] = is_pressed;

                    // Bezel shadow
                    ctx_draw.painter.circle_filled(button_center + Vec2::new(1.5, 1.5), 16.0, Color32::from_rgba_unmultiplied(0, 0, 0, 100));
                    // Chrome housing bezel
                    ctx_draw.painter.circle_filled(button_center, 16.0, Color32::from_gray(150));
                    ctx_draw.painter.circle_stroke(button_center, 16.0, Stroke::new(1.0, Color32::from_gray(90)));
                    ctx_draw.painter.circle_filled(button_center, 13.0, Color32::from_gray(40));

                    // Red button plunger actuator cap
                    let btn_radius = if is_pressed { 9.0 } else { 10.5 };
                    let btn_color = if is_pressed {
                        Color32::from_rgb(140, 25, 25)
                    } else {
                        Color32::from_rgb(190, 40, 40)
                    };
                    ctx_draw.painter.circle_filled(button_center, btn_radius, btn_color);
                    ctx_draw.painter.circle_stroke(button_center, btn_radius, Stroke::new(1.0, Color32::from_rgb(110, 15, 15)));
                }

                // ------------------ 8. DECIMAL INPUT-OUTPUT DIAL ------------------
                let dial_center = offset + Vec2::new(820.0, 580.0);
                ctx_draw.painter.text(
                    dial_center + Vec2::new(0.0, 160.0),
                    egui::Align2::CENTER_CENTER,
                    "DECIMAL INPUT-OUTPUT",
                    egui::FontId::proportional(12.0),
                    Color32::from_gray(220),
                );

                // Draw dial disc background
                ctx_draw.painter.circle_filled(dial_center, 105.0, Color32::from_rgb(18, 30, 50));
                ctx_draw.painter.circle_stroke(dial_center, 105.0, Stroke::new(2.0, Color32::from_gray(100)));

                // Wiper D16 port (Placed inside near the top edge of the dial face)
                let d16_pos = dial_center + Vec2::new(0.0, -118.0);
                let d16_uuid = self.sim.port_to_uuid[&ComponentPort::DialWiper];
                ctx_draw.draw_custom_port(d16_pos, d16_uuid, "D16");

                // Dial contacts D0 to D15 in a circle of radius 74
                let contact_radius = 74.0;
                for i in 0..16 {
                    let angle = (i as f32 * std::f32::consts::PI / 8.0) - std::f32::consts::FRAC_PI_2;
                    let contact_pos = dial_center + Vec2::new(contact_radius * angle.cos(), contact_radius * angle.sin());
                    let d_uuid = self.sim.port_to_uuid[&ComponentPort::DialContact(i)];

                    ctx_draw.port_positions.insert((d_uuid, 0), contact_pos + Vec2::new(-4.0, 0.0));
                    ctx_draw.port_positions.insert((d_uuid, 1), contact_pos + Vec2::new(4.0, 0.0));

                    // Simplified single hole drawing for contacts in circular panel to keep it neat
                    ctx_draw.painter.circle_filled(contact_pos, 3.5, Color32::from_rgb(15, 15, 15));
                    ctx_draw.painter.circle_stroke(contact_pos, 3.5, Stroke::new(1.0, Color32::from_gray(120)));

                    ctx_draw.handle_hole(contact_pos, d_uuid, 0);

                    // Draw labels outside
                    let label_pos = dial_center + Vec2::new(90.0 * angle.cos(), 90.0 * angle.sin());
                    ctx_draw.painter.text(
                        label_pos,
                        egui::Align2::CENTER_CENTER,
                        format!("{i}"),
                        egui::FontId::proportional(9.0),
                        Color32::from_gray(200),
                    );
                }

                // Dial center pointer knob
                let knob_radius = 32.0;
                let knob_rect = Rect::from_center_size(dial_center, Vec2::new((knob_radius + 6.0) * 2.0, (knob_radius + 6.0) * 2.0));
                let knob_id = ctx_draw.ui.make_persistent_id("dial_knob");
                let knob_resp = ctx_draw.ui.interact(knob_rect, knob_id, egui::Sense::drag());

                // If user drags the knob manually (and motor is not active)
                if knob_resp.dragged() && !self.sim.motor_running
                    && let Some(m_pos) = ctx.pointer_interact_pos() {
                        let delta = m_pos - dial_center;
                        let mut angle = delta.y.atan2(delta.x) + std::f32::consts::FRAC_PI_2;
                        if angle < 0.0 {
                            angle += std::f32::consts::TAU;
                        }
                        self.sim.dial_position = (angle / std::f32::consts::TAU * 16.0) % 16.0;
                    }

                // Draw outer fluted knob skirt with shadows
                ctx_draw.painter.circle_filled(dial_center + Vec2::new(2.0, 2.0), knob_radius + 6.0, Color32::from_rgba_unmultiplied(0, 0, 0, 140));
                ctx_draw.painter.circle_filled(dial_center, knob_radius + 6.0, Color32::from_rgb(20, 20, 20));
                ctx_draw.painter.circle_stroke(dial_center, knob_radius + 6.0, Stroke::new(1.0, Color32::from_gray(60)));

                // Draw silver/chrome center cap insert
                ctx_draw.painter.circle_filled(dial_center, knob_radius - 8.0, Color32::from_gray(160));
                ctx_draw.painter.circle_stroke(dial_center, knob_radius - 8.0, Stroke::new(1.0, Color32::from_gray(215)));

                // Draw beautiful white triangular indicator needle
                let ptr_angle = (self.sim.dial_position * std::f32::consts::PI / 8.0) - std::f32::consts::FRAC_PI_2;
                let ptr_base_left = dial_center + Vec2::new(5.0 * (ptr_angle + std::f32::consts::FRAC_PI_2).cos(), 5.0 * (ptr_angle + std::f32::consts::FRAC_PI_2).sin());
                let ptr_base_right = dial_center + Vec2::new(5.0 * (ptr_angle - std::f32::consts::FRAC_PI_2).cos(), 5.0 * (ptr_angle - std::f32::consts::FRAC_PI_2).sin());
                let ptr_tip = dial_center + Vec2::new(60.0 * ptr_angle.cos(), 60.0 * ptr_angle.sin());
                ctx_draw.painter.add(Shape::convex_polygon(
                    vec![ptr_base_left, ptr_base_right, ptr_tip],
                    Color32::from_rgb(235, 235, 235),
                    Stroke::new(1.0, Color32::from_gray(180)),
                ));

                // Motor control terminals (17, 18, 19)
                let m17_uuid = self.sim.port_to_uuid[&ComponentPort::DialMotor(17)];
                ctx_draw.draw_custom_port(dial_center + Vec2::new(-40.0, 130.0), m17_uuid, "17");

                let m18_uuid = self.sim.port_to_uuid[&ComponentPort::DialMotor(18)];
                ctx_draw.draw_custom_port(dial_center + Vec2::new(0.0, 130.0), m18_uuid, "18");

                let m19_uuid = self.sim.port_to_uuid[&ComponentPort::DialMotor(19)];
                ctx_draw.draw_custom_port(dial_center + Vec2::new(40.0, 130.0), m19_uuid, "19");

                ctx_draw.painter.text(
                    dial_center + Vec2::new(-20.0, 145.0),
                    egui::Align2::CENTER_CENTER,
                    "RUN",
                    egui::FontId::proportional(8.0),
                    Color32::from_gray(180),
                );
                ctx_draw.painter.text(
                    dial_center + Vec2::new(20.0, 145.0),
                    egui::Align2::CENTER_CENTER,
                    "STOP",
                    egui::FontId::proportional(8.0),
                    Color32::from_gray(180),
                );

                // ------------------ 9. WIRING HELP GUIDES OVERLAY ------------------
                if self.show_guides {
                    let exp = &EXPERIMENTS[self.selected_experiment];
                    for &(src_name, src_hole, dst_name, dst_hole) in exp.wiring {
                        if let (Some(from_uuid), Some(to_uuid)) = (
                            get_port_uuid_by_spec(src_name, ctx_draw.port_to_uuid),
                            get_port_uuid_by_spec(dst_name, ctx_draw.port_to_uuid),
                        )
                            && let (Some(&p_from), Some(&p_to)) = (
                                ctx_draw.port_positions.get(&(from_uuid, src_hole)),
                                ctx_draw.port_positions.get(&(to_uuid, dst_hole)),
                            ) {
                                // Draw dashed guide line
                                let dist = p_from.distance(p_to);
                                let sag = 15.0 + dist * 0.12;
                                let cp1 = p_from + Vec2::new(0.0, sag);
                                let cp2 = p_to + Vec2::new(0.0, sag);

                                ctx_draw.painter.add(Shape::CubicBezier(CubicBezierShape::from_points_stroke(
                                    [p_from, cp1, cp2, p_to],
                                    false,
                                    Color32::TRANSPARENT,
                                    Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 235, 59, 100)),
                                )));
                            }
                    }
                }

                // ------------------ 10. DRAW PLUGGED CABLES ------------------
                for conn in ctx_draw.connections.iter() {
                    if let (Some(&p_from), Some(&p_to)) = (
                        ctx_draw.port_positions.get(&(conn.from, conn.from_hole)),
                        ctx_draw.port_positions.get(&(conn.to, conn.to_hole)),
                    ) {
                        let dist = p_from.distance(p_to);
                        let sag = 20.0 + dist * 0.15;
                        let cp1 = p_from + Vec2::new(0.0, sag);
                        let cp2 = p_to + Vec2::new(0.0, sag);

                        let color = Color32::from_rgba_unmultiplied(
                            conn.color[0],
                            conn.color[1],
                            conn.color[2],
                            conn.color[3],
                        );

                        // Draw cable plug pins at both ends
                        ctx_draw.painter.circle_filled(p_from, 3.5, Color32::from_gray(60));
                        ctx_draw.painter.circle_filled(p_to, 3.5, Color32::from_gray(60));

                        // Draw soft cable drop shadow on faceplate
                        let shadow_offset = Vec2::new(3.0, 8.0);
                        let sp_from = p_from + shadow_offset;
                        let sp_to = p_to + shadow_offset;
                        let scp1 = cp1 + shadow_offset;
                        let scp2 = cp2 + shadow_offset;
                        ctx_draw.painter.add(Shape::CubicBezier(CubicBezierShape::from_points_stroke(
                            [sp_from, scp1, scp2, sp_to],
                            false,
                            Color32::TRANSPARENT,
                            Stroke::new(3.5, Color32::from_rgba_unmultiplied(0, 0, 0, 90)),
                        )));

                        // Thick main wire jacket
                        ctx_draw.painter.add(Shape::CubicBezier(CubicBezierShape::from_points_stroke(
                            [p_from, cp1, cp2, p_to],
                            false,
                            Color32::TRANSPARENT,
                            Stroke::new(3.2, color),
                        )));

                        // Wire lighting sheen on top
                        ctx_draw.painter.add(Shape::CubicBezier(CubicBezierShape::from_points_stroke(
                            [p_from, cp1, cp2, p_to],
                            false,
                            Color32::TRANSPARENT,
                            Stroke::new(0.8, Color32::from_rgba_unmultiplied(255, 255, 255, 80)),
                        )));
                    }
                }

                // ------------------ 11. DRAW CURRENT ACTIVE DRAGGED CABLE ------------------
                if let DragState::Dragging { from_port, from_hole, color } = *ctx_draw.drag_state
                    && let Some(&p_from) = ctx_draw.port_positions.get(&(from_port, from_hole)) {
                        let cursor_pos = ctx.pointer_interact_pos().unwrap_or(p_from);

                        let dist = p_from.distance(cursor_pos);
                        let sag = 20.0 + dist * 0.15;
                        let cp1 = p_from + Vec2::new(0.0, sag);
                        let cp2 = cursor_pos + Vec2::new(0.0, sag);

                        let color32 = Color32::from_rgba_unmultiplied(
                            color[0],
                            color[1],
                            color[2],
                            color[3],
                        );

                        // Draw pin at start
                        ctx_draw.painter.circle_filled(p_from, 3.5, Color32::from_gray(60));

                        // Draw soft drop shadow for dragged cable
                        let shadow_offset = Vec2::new(3.0, 8.0);
                        let sp_from = p_from + shadow_offset;
                        let scursor_pos = cursor_pos + shadow_offset;
                        let scp1 = cp1 + shadow_offset;
                        let scp2 = cp2 + shadow_offset;
                        ctx_draw.painter.add(Shape::CubicBezier(CubicBezierShape::from_points_stroke(
                            [sp_from, scp1, scp2, scursor_pos],
                            false,
                            Color32::TRANSPARENT,
                            Stroke::new(3.5, Color32::from_rgba_unmultiplied(0, 0, 0, 70)),
                        )));

                        // Draw main wire jacket
                        ctx_draw.painter.add(Shape::CubicBezier(CubicBezierShape::from_points_stroke(
                            [p_from, cp1, cp2, cursor_pos],
                            false,
                            Color32::TRANSPARENT,
                            Stroke::new(3.2, color32),
                        )));

                        // Wire sheen
                        ctx_draw.painter.add(Shape::CubicBezier(CubicBezierShape::from_points_stroke(
                            [p_from, cp1, cp2, cursor_pos],
                            false,
                            Color32::TRANSPARENT,
                            Stroke::new(0.8, Color32::from_rgba_unmultiplied(255, 255, 255, 80)),
                        )));

                        // Draw pin tip at cursor
                        ctx_draw.painter.circle_filled(cursor_pos, 2.5, Color32::from_gray(180));
                    }

                // ------------------ 12. GLOBAL DRAG DROP RESOLUTION ------------------
                if *ctx_draw.drag_state != DragState::None && ctx.input(|i| i.pointer.any_released()) {
                    if let DragState::Dragging { from_port, from_hole, color } = *ctx_draw.drag_state
                        && let Some((to_port, to_hole)) = *ctx_draw.hovered_hole {
                            // Verify not same port and not already occupied
                            let not_same = to_port != from_port;
                            let occupied = ctx_draw.connections.iter().any(|conn| {
                                (conn.from == to_port && conn.from_hole == to_hole)
                                    || (conn.to == to_port && conn.to_hole == to_hole)
                            });

                            if not_same && !occupied {
                                ctx_draw.connections.push(crate::simulation::Connection {
                                    from: from_port,
                                    from_hole,
                                    to: to_port,
                                    to_hole,
                                    color,
                                });
                            }
                        }
                    self.drag_state = DragState::None;
                }
            });
    }
}
