use eframe::egui;
use crate::dcc::types::BlockReasonCode;
use crate::output_mode::{RenderContext, OutputMode, render_output};

pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct CosynApp {
    input: String,
    output: String,
    status: String,
    log_lines: Vec<String>,
}

impl Default for CosynApp {
    fn default() -> Self {
        Self {
            input: String::new(),
            output: String::new(),
            status: "Ready".into(),
            log_lines: Vec::new(),
        }
    }
}

impl CosynApp {
    fn with_status(status: String) -> Self {
        Self {
            input: String::new(),
            output: String::new(),
            status,
            log_lines: Vec::new(),
        }
    }
}

impl CosynApp {
    fn run_pipeline(&mut self) {
        self.output.clear();
        self.log_lines.clear();
        self.status = "Running...".into();

        // Clear any prior telemetry
        crate::telemetry::take_log();

        match crate::orchestrator::run(&self.input) {
            Ok(locked) => {
                self.status = "PASS — output released".into();
                self.output = locked.text;
            }
            Err(e) => {
                let error_str = e.to_string();
                let friendly = friendly_block_message(&error_str);
                self.status = format!("BLOCKED — {}", friendly);
            }
        }

        self.log_lines = crate::telemetry::take_log();
        let dcc_lines = crate::dcc::telemetry::take_dcc_log();
        crate::telemetry::flush_to_file(&self.log_lines, &dcc_lines);
    }
}

impl eframe::App for CosynApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(format!("CoSyn v{}", APP_VERSION));
            ui.separator();

            ui.label("Request:");
            ui.text_edit_multiline(&mut self.input);

            if ui.button("Run governed pipeline").clicked() && !self.input.trim().is_empty() {
                self.run_pipeline();
            }

            ui.separator();
            ui.label(format!("Status: {}", self.status));

            if !self.log_lines.is_empty() {
                ui.separator();
                ui.label("Pipeline log:");
                for line in &self.log_lines {
                    ui.monospace(line);
                }
            }

            if !self.output.is_empty() {
                ui.separator();
                ui.label("Result:");

                let render_ctx = if detect_artifact_mode(&self.input) {
                    RenderContext { mode: OutputMode::Artifact }
                } else {
                    RenderContext { mode: OutputMode::Standard }
                };

                let rendered = render_output(&render_ctx, &self.output);

                #[cfg(debug_assertions)]
                {
                    if matches!(render_ctx.mode, OutputMode::Artifact) {
                        assert!(
                            !rendered.contains("Router (control-plane)"),
                            "Artifact mode contaminated with control-plane output"
                        );
                    }
                }

                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .show(ui, |ui| {
                        ui.monospace(&rendered);
                    });
            }
        });
    }
}

fn friendly_block_message(error: &str) -> String {
    let codes = [
        BlockReasonCode::BrSubjectUnknown,
        BlockReasonCode::BrEvidenceUnsat,
        BlockReasonCode::BrAmbiguity,
        BlockReasonCode::BrStructuralFail,
        BlockReasonCode::BrGroundingFail,
        BlockReasonCode::BrVersionConflict,
        BlockReasonCode::BrVersionUndefined,
        BlockReasonCode::BrReleaseDenied,
    ];
    for code in &codes {
        if error.contains(code.code()) {
            return format!("{}\n({})", code.user_message(), code.code());
        }
    }
    error.to_string()
}

fn detect_artifact_mode(input: &str) -> bool {
    let normalized = input.to_lowercase();
    normalized.contains("paste-ready")
        || normalized.contains("render as artifact")
        || normalized.contains("artifact only")
}

pub fn launch() -> Result<(), eframe::Error> {
    launch_with_status(None)
}

pub fn launch_with_status(warning: Option<String>) -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(&format!("CoSyn v{}", APP_VERSION))
            .with_inner_size([520.0, 480.0]),
        ..Default::default()
    };
    eframe::run_native(
        "CoSyn",
        options,
        Box::new(move |_cc| {
            let app = match warning {
                Some(msg) => CosynApp::with_status(format!("WARNING — {}", msg)),
                None => CosynApp::default(),
            };
            Ok(Box::new(app))
        }),
    )
}
