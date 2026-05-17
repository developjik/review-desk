use crate::ui::state::{AppViewState, ViewportMode, WorkspaceSection, viewport_mode_for_width};
use crate::ui::{bottom_status, dialogs, draft_pane, pr_workspace, queue_pane, theme, top_bar};
use eframe::egui;

#[derive(Debug, Clone)]
pub struct ReviewDeskApp {
    state: AppViewState,
}

impl Default for ReviewDeskApp {
    fn default() -> Self {
        Self {
            state: AppViewState::startup_auth_required(),
        }
    }
}

impl eframe::App for ReviewDeskApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let width = ui.available_width();

        self.state.viewport = viewport_mode_for_width(self.state.viewport, width);

        egui::Panel::top("reviewdesk_top_bar")
            .exact_size(theme::TOP_BAR_HEIGHT)
            .show_inside(ui, |ui| {
                top_bar::render(ui, &mut self.state);
            });

        egui::Panel::bottom("reviewdesk_bottom_status")
            .exact_size(theme::BOTTOM_STATUS_HEIGHT)
            .show_inside(ui, |ui| {
                bottom_status::render(ui, &self.state);
            });

        match self.state.viewport {
            ViewportMode::Wide => self.render_wide(ui),
            ViewportMode::Narrow(_) => self.render_narrow(ui),
        }

        let ctx = ui.ctx().clone();
        dialogs::render(&ctx, &mut self.state);
    }
}

impl ReviewDeskApp {
    fn render_wide(&mut self, ui: &mut egui::Ui) {
        egui::Panel::left("reviewdesk_queue_pane")
            .resizable(true)
            .default_size(theme::LEFT_PANE_WIDTH)
            .size_range(280.0..=360.0)
            .frame(egui::Frame::side_top_panel(ui.style()).fill(theme::panel_fill()))
            .show_inside(ui, |ui| {
                queue_pane::render(ui, &mut self.state);
            });

        egui::Panel::right("reviewdesk_draft_pane")
            .resizable(true)
            .default_size(theme::RIGHT_PANE_WIDTH)
            .size_range(340.0..=460.0)
            .frame(egui::Frame::side_top_panel(ui.style()).fill(theme::panel_fill()))
            .show_inside(ui, |ui| {
                draft_pane::render(ui, &mut self.state);
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::central_panel(ui.style()).fill(theme::app_background()))
            .show_inside(ui, |ui| {
                pr_workspace::render(ui, &mut self.state);
            });
    }

    fn render_narrow(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default()
            .frame(egui::Frame::central_panel(ui.style()).fill(theme::app_background()))
            .show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    let ViewportMode::Narrow(active) = self.state.viewport else {
                        return;
                    };
                    let mut next = active;
                    ui.selectable_value(&mut next, WorkspaceSection::Queue, "Queue");
                    ui.selectable_value(&mut next, WorkspaceSection::Review, "Review");
                    ui.selectable_value(&mut next, WorkspaceSection::Draft, "Draft");
                    self.state.viewport = ViewportMode::Narrow(next);
                });
                ui.separator();

                match self.state.viewport {
                    ViewportMode::Narrow(WorkspaceSection::Queue) => {
                        queue_pane::render(ui, &mut self.state)
                    }
                    ViewportMode::Narrow(WorkspaceSection::Review) => {
                        pr_workspace::render(ui, &mut self.state)
                    }
                    ViewportMode::Narrow(WorkspaceSection::Draft) => {
                        draft_pane::render(ui, &mut self.state)
                    }
                    ViewportMode::Wide => {}
                }
            });
    }
}
