use egui_winit_vulkano::Gui;
use egui_winit_vulkano::GuiConfig;
use egui_winit_vulkano::egui;

use egui_winit_vulkano::egui::Pos2;
use egui_winit_vulkano::egui::{Color32, Frame};
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;

use std::fmt;

use crate::app::VulkanManager;
use crate::simulator::Simulator;

#[derive(PartialEq, Debug, Copy, Clone)]
pub enum SimulationLayer {
    Real = 0,
    Imaginary,
    Probability,
    Potential,
    WaveFunction,
}

impl fmt::Display for SimulationLayer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SimulationLayer::Real => write!(f, "Real"),
            SimulationLayer::Imaginary => write!(f, "Imaginary"),
            SimulationLayer::Probability => write!(f, "Probability"),
            SimulationLayer::Potential => write!(f, "Potential"),
            SimulationLayer::WaveFunction => write!(f, "Wave Function"),
        }
    }
}

#[derive(PartialEq, Debug, Copy, Clone)]
pub enum BoundaryCondition {
    Dirichlet = 0,
    Neumann,
    Periodic,
}

pub struct UIState {
    pub gui: Gui,
    pub gui_width: f32,
    pub mouse_x: f32,
    pub mouse_y: f32,

    pub time_step: f32,
    pub speed: f32,
    pub theta: f32,

    pub brush_x: i32,
    pub brush_y: i32,
    pub brush_enabled: i32,
    pub brush_radius: f32,
    pub brush_value: i32,

    pub visible_layer: SimulationLayer,
    pub brush_layer: SimulationLayer,
    
    pub boundary_condition: BoundaryCondition,
}

impl UIState {
    pub fn new(event_loop: &ActiveEventLoop, mgr: &mut VulkanManager) -> Self {
        let gui_config = GuiConfig {
            allow_srgb_render_target: true,
            is_overlay: true,
            ..Default::default()
        };

        let gui = {
            let renderer = mgr.windows.get_primary_renderer_mut().unwrap();
            Gui::new(
                event_loop,
                renderer.surface(),
                renderer.graphics_queue(),
                renderer.swapchain_format(),
                gui_config,
            )
        };

        UIState {
            gui,
            gui_width: 300f32,
            mouse_x: 0.0,
            mouse_y: 0.0,

            time_step: 0.180,
            speed: 1.0,
            theta: 0.0,

            brush_x: 0,
            brush_y: 0,
            brush_enabled: 0,
            brush_radius: 2.5,
            brush_value: 8,

            brush_layer: SimulationLayer::WaveFunction,
            visible_layer: SimulationLayer::Probability,
            
            boundary_condition: BoundaryCondition::Periodic,
        }
    }

    pub fn handle_event(&mut self, event: &WindowEvent) {
        self.gui.update(&event);
    }

    pub fn setup_gui(
        &mut self,
        mgr: &VulkanManager,
        simulator: &Simulator,
    ) {
        let width = mgr.windows.get_primary_window().as_ref().unwrap().inner_size().width as f32;
        let side_panel = egui::SidePanel::new(egui::panel::Side::Left, "side-panel");

        self.gui.immediate_ui(|gui| {
            let ctx = gui.context();
            ctx.set_pixels_per_point(1.0);
            
            // Draw the arrow indicator at the position of the mouse pointer
            egui::CentralPanel::default()
                .frame(Frame::NONE.fill(Color32::TRANSPARENT))
                .show(&ctx, |ui| {
                    let painter = ui.painter();
                    
                    let radius = self.speed * 20.0;
                    
                    let center = Pos2::new(self.mouse_x, self.mouse_y);
                    let endpoint = center + egui::vec2(radius * f32::cos(self.theta), radius * -f32::sin(self.theta));
                    
                    let x = 8.0;
                    let theta_offset = f32::atan2(x * std::f32::consts::FRAC_1_SQRT_2, radius - x * std::f32::consts::FRAC_1_SQRT_2);
                    let arrow_radius = f32::hypot(x * std::f32::consts::FRAC_1_SQRT_2, radius - x * std::f32::consts::FRAC_1_SQRT_2);
                    
                    let point_a = center + egui::vec2(arrow_radius * f32::cos(self.theta + theta_offset), arrow_radius * -f32::sin(self.theta + theta_offset));
                    let point_b = center + egui::vec2(arrow_radius * f32::cos(self.theta - theta_offset), arrow_radius * -f32::sin(self.theta - theta_offset));
                    
                    if self.speed > 0.0 {
                        painter.line_segment([center, endpoint], egui::Stroke::new(2.0, egui::Color32::WHITE));
                        painter.line_segment([endpoint, point_a], egui::Stroke::new(2.0, egui::Color32::WHITE));
                        painter.line_segment([endpoint, point_b], egui::Stroke::new(2.0, egui::Color32::WHITE));
                    }
                }
            );
            
            side_panel
                .show_separator_line(true)
                .exact_width(self.gui_width)
                .resizable(false)
                .show(&ctx, |ui| {
                    ui.vertical_centered(|ui| ui.heading("Quantum Echoes"));
                    ui.separator();
                    
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.heading("Brush");
                        ui.add(
                            egui::widgets::Slider::new(&mut self.brush_radius, 0.1..=3.0)
                                .text("Brush Size")
                        ).on_hover_text("The standard deviation (σ) of the Gaussian wave packet.");
                        ui.add(
                            egui::widgets::Slider::new(&mut self.brush_value, 1..=10)
                                .text("Brush Value"),
                        ).on_hover_text("The amplitude of the Gaussian wave packet.");
                        ui.add(
                            egui::widgets::Slider::new(&mut self.speed, 0.0..=5.0)
                                .text("Speed")
                        ).on_hover_text("The speed of the Gaussian wave packet.");
                        ui.spacing();
                        ui.separator();
                        
                        ui.heading("Simulation Domain");
                        egui::ComboBox::from_label("Visible Layer")
                            .selected_text(format!("{}", self.visible_layer))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.visible_layer,
                                    SimulationLayer::Real,
                                    "Real",
                                );
                                ui.selectable_value(
                                    &mut self.visible_layer,
                                    SimulationLayer::Imaginary,
                                    "Imaginary",
                                );
                                ui.selectable_value(
                                    &mut self.visible_layer,
                                    SimulationLayer::Probability,
                                    "Probability",
                                );
                                ui.selectable_value(
                                    &mut self.visible_layer,
                                    SimulationLayer::WaveFunction,
                                    "Wave Function",
                                );
                            });

                        egui::ComboBox::from_label("Brush Layer")
                            .selected_text(format!("{}", self.brush_layer))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.brush_layer,
                                    SimulationLayer::WaveFunction,
                                    "Wave Function",
                                );
                                ui.selectable_value(
                                    &mut self.brush_layer,
                                    SimulationLayer::Potential,
                                    "Potential",
                                );
                            });
                        
                        egui::ComboBox::from_label("Boundary Condition")
                            .selected_text(format!("{:?}", self.boundary_condition))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.boundary_condition,
                                    BoundaryCondition::Dirichlet,
                                    "Dirichlet",
                                );
                                ui.selectable_value(
                                    &mut self.boundary_condition,
                                    BoundaryCondition::Neumann,
                                    "Neumann",
                                );
                                ui.selectable_value(
                                    &mut self.boundary_condition,
                                    BoundaryCondition::Periodic,
                                    "Periodic",
                                );
                            });
                        
                        ui.add(
                            egui::widgets::Slider::new(&mut self.time_step, 0.0..=0.5)
                                .text("Time Step"),
                        ).on_hover_text("The time in between each frame to advance the simulation by.\n\nNOTE: Setting this too high may cause the simulation to \"explode\" from numerical instability. If this happens, reset the simulation using the button below.");
                        
                        if ui.button("Reset Simulation Domain").clicked() {
                            simulator.zero_grid(mgr);
                        }
                        
                        ui.separator();
                        
                        egui::CollapsingHeader::new("Brush Info").default_open(true).show(ui, |ui| {
                            ui.add(egui::widgets::Label::new("Click on the simulation domain (to the left) to draw a particle as a Gaussian wave packet with an initial velocity."));
                            ui.add(egui::widgets::Label::new("You can use the mouse scroll wheel to change the direction of this velocity (denoted by the arrow beneath the cursor)"));
                            ui.add(egui::widgets::Label::new("Try making two of these particles collide into each other."));
                            ui.add(egui::widgets::Label::new("Also, try playing around with the visible and brush layers. More info about these can be found below."));
                        });
                        egui::CollapsingHeader::new("Simulation Domain Info").default_open(true).show(ui, |ui| {
                            egui::CollapsingHeader::new("Visible Layer").show(ui, |ui| {
                                ui.separator();
                                ui.add(egui::widgets::Label::new("Real (Re(Ψ)) - Wave function's real component"));
                                ui.separator();
                                ui.add(egui::widgets::Label::new("Imaginary (Im(Ψ)) - Wave function's imaginary component"));
                                ui.separator();
                                ui.add(egui::widgets::Label::new("Probability (|Ψ|²) - Wave function's probability density function"));
                                ui.separator();
                                ui.add(egui::widgets::Label::new("Wave Function (Ψ) - The quantum wave function; hue denotes phase and brightness is proportional to amplitude"));
                                ui.separator();
                            });
                            egui::CollapsingHeader::new("Brush Layer").show(ui, |ui| {
                                ui.separator();
                                ui.add(egui::widgets::Label::new("Wave Function - Draw directly on the wave function"));
                                ui.separator();
                                ui.add(egui::widgets::Label::new("Potential - Draw regions of higher potential energy which can act as barriers for the wave function"));
                                ui.separator();
                            });
                            egui::CollapsingHeader::new("Boundary Conditions").show(ui, |ui| {
                                ui.add(egui::widgets::Label::new("Defines how derivatives are calculated on the edges of the simulation domain."));
                                ui.separator();
                                ui.add(egui::widgets::Label::new("Dirichlet - The edges take on the value of 0"));
                                ui.separator();
                                ui.add(egui::widgets::Label::new("Neumann - The derivative/flux perpendicular to each edge is always 0"));
                                ui.separator();
                                ui.add(egui::widgets::Label::new("Periodic - Each edge wraps around to the opposite edge of the domain"));
                                ui.separator();
                            });
                        });

                    });
                });
        });
    }
}
