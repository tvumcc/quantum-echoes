use egui_winit_vulkano::egui;
use egui_winit_vulkano::Gui;
use egui_winit_vulkano::GuiConfig;

use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;

use crate::app::VulkanManager;
use crate::quad_renderer::QuadRenderer;
use crate::simulator::Simulator;

#[derive(PartialEq, Debug, Copy, Clone)]
pub enum SimulationLayer {
    Real = 0,
    Imaginary,
    Probability,
    Potential,
}

pub struct UIState {
    pub gui: Gui,
    pub gui_width: f32,

    pub time_step: f32,

    pub brush_x: i32,
    pub brush_y: i32,
    pub brush_enabled: i32,
    pub brush_radius: i32,
    pub brush_value: i32,

    pub visible_layer: SimulationLayer,
    pub brush_layer: SimulationLayer,
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

            time_step: 0.01,

            brush_x: 0,
            brush_y: 0,
            brush_enabled: 0,
            brush_radius: 2,
            brush_value: 2,

            brush_layer: SimulationLayer::Real,
            visible_layer: SimulationLayer::Probability,
        }
    }

    pub fn handle_event(&mut self, event: &WindowEvent) {
        self.gui.update(&event);
    }

    pub fn setup_gui(&mut self, mgr: &VulkanManager, renderer: &mut QuadRenderer, simulator: &Simulator) {
        let side_panel = egui::SidePanel::new(egui::panel::Side::Left, "side-panel");
        let prev_visible_layer = self.visible_layer;

        self.gui.immediate_ui(|gui| {
            let ctx = gui.context();
            ctx.set_pixels_per_point(1.0);
            side_panel.show_separator_line(true).exact_width(self.gui_width).resizable(false).show(&ctx, |ui| {
                ui.vertical_centered(|ui|{
                    ui.heading("Quantum Echoes")
                });
                ui.separator();

                egui::ScrollArea::horizontal()
                    .show(ui, |ui|{
                        ui.add(egui::widgets::Slider::new(&mut self.time_step, 0.0..=0.06).text("Time Step"));
                        ui.add(egui::widgets::Slider::new(&mut self.brush_radius, 1..=8).text("Brush Radius"));
                        ui.add(egui::widgets::Slider::new(&mut self.brush_value, 1..=10).text("Brush Value"));
                        ui.spacing();
                        ui.separator();
                        if ui.button("Reset Grid").clicked() {
                            simulator.zero_grid(mgr);
                        }

                        egui::ComboBox::from_label("Visible Layer")
                            .selected_text(format!("{:?}", self.visible_layer))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.visible_layer, SimulationLayer::Real, "Real");
                                ui.selectable_value(&mut self.visible_layer, SimulationLayer::Imaginary, "Imaginary");
                                ui.selectable_value(&mut self.visible_layer, SimulationLayer::Probability, "Probability");
                            }
                        );

                        egui::ComboBox::from_label("Brush Layer")
                            .selected_text(format!("{:?}", self.brush_layer))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.brush_layer, SimulationLayer::Real, "Real");
                                ui.selectable_value(&mut self.brush_layer, SimulationLayer::Imaginary, "Imaginary");
                                ui.selectable_value(&mut self.brush_layer, SimulationLayer::Potential, "Potential");
                            }
                        );

                    });
            });
        });

        if prev_visible_layer != self.visible_layer {
            renderer.update_command_buffers(mgr, self);
        }
    }
}