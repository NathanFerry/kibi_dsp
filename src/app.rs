use std::sync::Arc;

use crate::audio::decoder::decode_audio_file;
use crate::audio::player::Player;
use crate::dsp::chain::ProcessorChain;
use crate::ui::controls::Controls;
use crate::ui::toolbar::Toolbar;
use crate::ui::waveform::Waveform;

#[derive(Default)]
pub struct DspApp {
    player: Option<Player>,
    file_name: Option<String>,
    error_msg: Option<String>,
    toolbar: Toolbar,
    controls: Controls,
    waveform: Waveform,
}

impl eframe::App for DspApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if self
            .toolbar
            .show(ui, self.player.as_ref(), self.file_name.as_deref())
            && let Some(path) = rfd::FileDialog::new()
                .add_filter("Audio", &["wav", "mp3", "flac", "ogg", "aac", "m4a"])
                .pick_file()
        {
            match decode_audio_file(&path) {
                Ok((samples, sample_rate)) => {
                    self.file_name = path.file_name().and_then(|n| n.to_str()).map(String::from);
                    self.error_msg = None;
                    let samples = Arc::new(samples);
                    let chain = ProcessorChain::new();
                    self.controls = Controls::from_chain(&chain);
                    self.waveform = Waveform::default();
                    match Player::new(Arc::clone(&samples), sample_rate, chain) {
                        Ok(p) => self.player = Some(p),
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

        if let Some(err) = &self.error_msg {
            ui.colored_label(egui::Color32::RED, err);
        }

        // Extract player handles without holding a borrow across the mutable widget calls.
        let player_data = self.player.as_ref().map(|p| {
            (
                Arc::clone(&p.samples),
                Arc::clone(&p.cursor),
                p.sample_rate,
                Arc::clone(&p.waveform_queue),
                p.param_tx.clone(),
            )
        });

        if let Some((samples, cursor, sample_rate, waveform_queue, param_tx)) = player_data {
            ui.add_space(4.0);
            self.waveform
                .show(ui, &samples, &cursor, sample_rate, &waveform_queue);
            ui.separator();
            self.controls.show(ui, Some(&param_tx));

            if self
                .player
                .as_ref()
                .is_some_and(|p| p.playing.load(std::sync::atomic::Ordering::Relaxed))
            {
                ui.ctx().request_repaint();
            }
        }
    }
}
