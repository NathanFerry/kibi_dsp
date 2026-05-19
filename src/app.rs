use std::sync::Arc;
use std::sync::atomic::Ordering;

use crate::audio::decoder::decode_audio_file;
use crate::audio::player::Player;

#[derive(Default)]
pub struct DspApp {
    player: Option<Player>,
    file_name: Option<String>,
    error_msg: Option<String>,
}

impl eframe::App for DspApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.horizontal(|ui| {
            if ui.button("Open File").clicked()
                && let Some(path) = rfd::FileDialog::new()
                    .add_filter("Audio", &["wav", "mp3", "flac", "ogg", "aac", "m4a"])
                    .pick_file()
            {
                match decode_audio_file(&path) {
                    Ok((samples, sample_rate)) => {
                        self.file_name =
                            path.file_name().and_then(|n| n.to_str()).map(String::from);
                        self.error_msg = None;
                        match Player::new(Arc::new(samples), sample_rate) {
                            Ok(p) => {
                                self.player = Some(p);
                            }
                            Err(e) => {
                                log::error!("failed to create player: {e:#}");
                                self.error_msg = Some(format!("Playback error: {e:#}"));
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("failed to decode audio: {e:#}");
                        self.error_msg = Some(format!("Decode error: {e:#}"));
                    }
                }
            }

            if let Some(player) = &self.player {
                let is_playing = player.playing.load(Ordering::Relaxed);
                let label = if is_playing { "Pause" } else { "Play" };
                if ui.button(label).clicked() {
                    player.playing.store(!is_playing, Ordering::Relaxed);
                }

                if let Some(name) = &self.file_name {
                    ui.label(format!("File: {name}"));
                }
            } else {
                ui.add_enabled(false, egui::Button::new("Play"));
            }
        });

        if let Some(err) = &self.error_msg {
            ui.colored_label(egui::Color32::RED, err);
        }
    }
}
