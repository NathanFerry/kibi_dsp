use std::sync::Arc;

use crate::audio::decoder::decode_audio_file;
use crate::audio::player::Player;
use crate::dsp::chain::{ChainCommand, ParamUpdate, ProcessorChain};
use crate::dsp::processors::make_processor;
use crate::ui::bode::BodeView;
use crate::ui::chain_editor::{ChainEditor, ChainEditorOutput};
use crate::ui::controls::Controls;
use crate::ui::spectrogram::Spectrogram;
use crate::ui::spectrum::Spectrum;
use crate::ui::theme::{BACKGROUND, SURFACE, TEXT_MUTED};
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
    selected_processor: Option<usize>,
}

impl eframe::App for DspApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let vu_level = self
            .player
            .as_ref()
            .map(|p| f32::from_bits(p.vu_level.load(std::sync::atomic::Ordering::Relaxed)))
            .unwrap_or(0.0);

        let open_clicked = egui::Panel::top("toolbar")
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(BACKGROUND)
                    .inner_margin(egui::Margin::same(8)),
            )
            .show_inside(ui, |ui| {
                let clicked = self.toolbar.show(
                    ui,
                    self.player.as_ref(),
                    self.file_name.as_deref(),
                    vu_level,
                );
                if let Some(err) = &self.error_msg {
                    ui.colored_label(egui::Color32::RED, err);
                }
                clicked
            })
            .inner;

        if open_clicked
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
                    self.selected_processor = None;
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

        struct SidebarFx {
            add_name: Option<String>,
            remove_idx: Option<usize>,
            param_updates: Vec<ParamUpdate>,
            bode_dirty: bool,
        }

        let sidebar_fx = egui::Panel::left("chain_sidebar")
            .exact_size(220.0)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(SURFACE)
                    .inner_margin(egui::Margin::same(10)),
            )
            .show_inside(ui, |ui| {
                ui.label(
                    egui::RichText::new("PROCESSING CHAIN")
                        .size(10.0)
                        .color(TEXT_MUTED),
                );
                ui.separator();
                ui.add_space(4.0);

                let current_names = self.controls.processor_names();

                let editor_out = if self.player.is_some() {
                    let out = self
                        .chain_editor
                        .show(ui, &current_names, self.selected_processor);
                    if let Some(idx) = out.selected_idx {
                        self.selected_processor = Some(idx);
                        self.bode.mark_dirty();
                    }
                    out
                } else {
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("Open an audio file to\nbuild your processing chain")
                            .color(TEXT_MUTED)
                            .size(11.0),
                    );
                    ChainEditorOutput {
                        add_name: None,
                        remove_idx: None,
                        selected_idx: None,
                    }
                };

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                ui.label(
                    egui::RichText::new("PARAMETERS")
                        .size(10.0)
                        .color(TEXT_MUTED),
                );
                ui.separator();
                ui.add_space(4.0);

                let mut param_updates = vec![];
                let bode_dirty;

                if let Some(idx) = self.selected_processor {
                    let updates = self.controls.show_params_only(ui, idx);
                    bode_dirty = !updates.is_empty();
                    param_updates = updates;
                } else {
                    ui.add_space(8.0);
                    ui.vertical_centered(|ui| {
                        ui.label(
                            egui::RichText::new("Select a processor\nto edit its parameters")
                                .color(TEXT_MUTED)
                                .size(11.0),
                        );
                    });
                    bode_dirty = false;
                }

                SidebarFx {
                    add_name: editor_out.add_name,
                    remove_idx: editor_out.remove_idx,
                    param_updates,
                    bode_dirty,
                }
            })
            .inner;

        if let Some(player) = &self.player {
            let sr = player.sample_rate as f32;

            if let Some(name) = &sidebar_fx.add_name {
                if let Some(proc_audio) = make_processor(name, sr) {
                    let _ = player.chain_cmd_tx.send(ChainCommand::Add(proc_audio));
                }
                if let Some(proc_ui) = make_processor(name, sr) {
                    if let Some(chain) = &mut self.ui_chain {
                        let idx = chain.processors().len();
                        chain.add(proc_ui);
                        if let Some(p) = chain.processors().get(idx) {
                            self.controls.push_processor(p.as_ref());
                        }
                    }
                    self.bode.mark_dirty();
                }
            }

            if let Some(idx) = sidebar_fx.remove_idx {
                let _ = player.chain_cmd_tx.send(ChainCommand::Remove(idx));
                if let Some(chain) = &mut self.ui_chain {
                    chain.remove(idx);
                }
                self.controls.remove_processor(idx);
                if self.selected_processor == Some(idx) {
                    self.selected_processor = None;
                } else if let Some(sel) = self.selected_processor
                    && sel > idx
                {
                    self.selected_processor = Some(sel - 1);
                }
                self.bode.mark_dirty();
            }

            for update in &sidebar_fx.param_updates {
                let _ = player.param_tx.send(update.clone());
                if let Some(chain) = &mut self.ui_chain {
                    chain.apply_update(update.clone());
                }
            }
        }

        if sidebar_fx.bode_dirty {
            self.bode.mark_dirty();
        }

        let player_data = self.player.as_ref().map(|p| {
            (
                Arc::clone(&p.samples),
                Arc::clone(&p.cursor),
                p.sample_rate,
                Arc::clone(&p.waveform_queue),
                Arc::clone(&p.orig_queue),
                Arc::clone(&p.fft_queue),
                p.playing.load(std::sync::atomic::Ordering::Relaxed),
            )
        });

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(BACKGROUND)
                    .inner_margin(egui::Margin::same(8)),
            )
            .show_inside(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    if let Some((
                        samples,
                        cursor,
                        sample_rate,
                        waveform_queue,
                        orig_queue,
                        fft_queue,
                        is_playing,
                    )) = &player_data
                    {
                        let is_playing = *is_playing;

                        vis_card(ui, "WAVEFORM", |ui| {
                            self.waveform.show(
                                ui,
                                samples,
                                cursor,
                                *sample_rate,
                                waveform_queue,
                                is_playing,
                            );
                        });

                        vis_card(ui, "SPECTRUM", |ui| {
                            self.spectrogram.update(fft_queue, is_playing);
                            self.spectrum.show(
                                ui,
                                orig_queue,
                                self.spectrogram.latest_magnitude_db(),
                                *sample_rate,
                                is_playing,
                            );
                        });

                        vis_card(ui, "SPECTROGRAM", |ui| {
                            self.spectrogram.show(ui, *sample_rate);
                        });

                        let selected = self.selected_processor.unwrap_or(0);
                        let tf_owned: Option<(Vec<f32>, Vec<f32>)> = self
                            .ui_chain
                            .as_ref()
                            .and_then(|chain| chain.processors().get(selected))
                            .and_then(|p| p.transfer_function());
                        let cutoff_hz = self
                            .selected_processor
                            .and_then(|i| self.controls.cutoff_hz_for(i));

                        vis_card(ui, "BODE DIAGRAM", |ui| {
                            self.bode.show(
                                ui,
                                tf_owned.as_ref().map(|(b, a)| (b.as_slice(), a.as_slice())),
                                cutoff_hz,
                                *sample_rate as f32,
                            );
                        });

                        if is_playing {
                            ui.ctx().request_repaint();
                        }
                    } else {
                        ui.centered_and_justified(|ui| {
                            ui.label(
                                egui::RichText::new("Open an audio file to get started")
                                    .color(TEXT_MUTED)
                                    .size(16.0),
                            );
                        });
                    }
                });
            });
    }
}

fn vis_card(ui: &mut egui::Ui, title: &str, content: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(SURFACE)
        .corner_radius(egui::CornerRadius::same(8))
        .inner_margin(egui::Margin::same(12))
        .outer_margin(egui::Margin::symmetric(0, 4))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(title).size(11.0).color(TEXT_MUTED));
            ui.separator();
            content(ui);
        });
}
