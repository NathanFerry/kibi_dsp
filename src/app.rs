use std::sync::Arc;

use crate::audio::decoder::decode_audio_file;
use crate::audio::player::Player;
use crate::dsp::chain::{ChainCommand, ProcessorChain};
use crate::dsp::processors::make_processor;
use crate::ui::bode::BodeView;
use crate::ui::chain_editor::ChainEditor;
use crate::ui::controls::Controls;
use crate::ui::spectrogram::Spectrogram;
use crate::ui::spectrum::Spectrum;
use crate::ui::toolbar::Toolbar;
use crate::ui::waveform::Waveform;

#[derive(Default)]
pub struct DspApp {
    player: Option<Player>,
    file_name: Option<String>,
    error_msg: Option<String>,
    toolbar: Toolbar,
    chain_editor: ChainEditor,
    controls: Controls,
    waveform: Waveform,
    spectrum: Spectrum,
    spectrogram: Spectrogram,
    ui_chain: Option<ProcessorChain>,
    bode: BodeView,
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
                    let audio_chain = ProcessorChain::new();
                    let ui_chain = ProcessorChain::new();
                    self.controls = Controls::from_chain(&audio_chain);
                    self.waveform = Waveform::default();
                    self.spectrum = Spectrum::default();
                    self.spectrogram.reset();
                    self.ui_chain = Some(ui_chain);
                    self.bode.mark_dirty();
                    match Player::new(Arc::clone(&samples), sample_rate, audio_chain) {
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

        let player_data = self.player.as_ref().map(|p| {
            (
                Arc::clone(&p.samples),
                Arc::clone(&p.cursor),
                p.sample_rate,
                Arc::clone(&p.waveform_queue),
                Arc::clone(&p.orig_queue),
                Arc::clone(&p.fft_queue),
                p.param_tx.clone(),
                p.chain_cmd_tx.clone(),
                p.playing.load(std::sync::atomic::Ordering::Relaxed),
            )
        });

        if let Some((
            samples,
            cursor,
            sample_rate,
            waveform_queue,
            orig_queue,
            fft_queue,
            param_tx,
            chain_cmd_tx,
            is_playing,
        )) = player_data
        {
            // Chain editor: add / remove processors.
            let current_names = self.controls.processor_names();
            let editor_out = self.chain_editor.show(ui, &current_names);
            ui.separator();

            if let Some(name) = editor_out.add_name {
                let sr = sample_rate as f32;
                if let Some(proc_audio) = make_processor(&name, sr) {
                    let _ = chain_cmd_tx.send(ChainCommand::Add(proc_audio));
                }
                if let Some(proc_ui) = make_processor(&name, sr) {
                    if let Some(chain) = &mut self.ui_chain {
                        let idx = chain.processors().len();
                        chain.add(proc_ui);
                        // Push into controls using the just-added processor.
                        if let Some(p) = chain.processors().get(idx) {
                            self.controls.push_processor(p.as_ref());
                        }
                    }
                    self.bode.mark_dirty();
                }
            }

            if let Some(idx) = editor_out.remove_idx {
                let _ = chain_cmd_tx.send(ChainCommand::Remove(idx));
                if let Some(chain) = &mut self.ui_chain {
                    chain.remove(idx);
                }
                self.controls.remove_processor(idx);
                self.bode.mark_dirty();
            }

            ui.add_space(4.0);
            self.waveform
                .show(ui, &samples, &cursor, sample_rate, &waveform_queue);
            ui.separator();

            // Spectrogram owns fft_queue: drain it and compute FFT frames first,
            // then pass the latest frame to the spectrum so it can plot the processed line.
            self.spectrogram.update(&fft_queue);
            self.spectrum.show(
                ui,
                &orig_queue,
                self.spectrogram.latest_magnitude_db(),
                sample_rate,
            );
            ui.separator();
            self.spectrogram.show(ui, sample_rate);
            ui.separator();

            let ctrl_out = self.controls.show(ui);

            for update in &ctrl_out.updates {
                let _ = param_tx.send(update.clone());
                if let Some(chain) = &mut self.ui_chain {
                    chain.apply_update(update.clone());
                }
                self.bode.mark_dirty();
            }
            if ctrl_out.selection_changed {
                self.bode.mark_dirty();
            }

            let selected = ctrl_out.selected_proc;
            let tf_owned: Option<(Vec<f32>, Vec<f32>)> = self
                .ui_chain
                .as_ref()
                .and_then(|chain| chain.processors().get(selected))
                .and_then(|p| p.transfer_function());

            let cutoff_hz = self.controls.selected_cutoff_hz();

            ui.separator();
            self.bode.show(
                ui,
                tf_owned.as_ref().map(|(b, a)| (b.as_slice(), a.as_slice())),
                cutoff_hz,
                sample_rate as f32,
            );

            if is_playing {
                ui.ctx().request_repaint();
            }
        }
    }
}
