use egui_winit_vulkano::egui;
use egui_winit_vulkano::Gui;
use egui_winit_vulkano::GuiConfig;

use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;

use crate::app::VulkanManager;

pub struct UIState {
    pub gui: Gui,
    pub gui_width: f32,

    pub time_step: f32,
    pub brush_radius: i32,
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
            brush_radius: 3,
        }
    }

    pub fn handle_event(&mut self, event: &WindowEvent) {
        self.gui.update(&event);
    }

    pub fn setup_gui(&mut self) {
        let side_panel = egui::SidePanel::new(egui::panel::Side::Left, "side-panel");

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
                        ui.add(egui::widgets::Slider::new(&mut self.time_step, 0.0..=1.0).text("Time Step"));
                        ui.add(egui::widgets::Slider::new(&mut self.brush_radius, 1..=20).text("Brush Radius"));
                        ui.spacing();
                        ui.separator();
                    });
            });
        });
    }
}