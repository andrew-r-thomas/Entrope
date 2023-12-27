use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;
use rand::prelude::*;
use std::sync::Arc;

mod editor;

pub struct EntropeRust {
    params: Arc<EntropeRustParams>,
    gen: rand::rngs::StdRng,
}

#[derive(Params)]
struct EntropeRustParams {
    #[id = "crush"]
    pub crush: FloatParam,

    #[id = "redux"]
    pub redux: IntParam,

    #[id = "entropy"]
    pub entropy: IntParam,

    #[id = "clip"]
    pub clip: FloatParam,

    #[persist = "editor-state"]
    editor_state: Arc<ViziaState>,
}

impl Default for EntropeRust {
    fn default() -> Self {
        Self {
            params: Arc::new(EntropeRustParams::default()),
            gen: StdRng::from_entropy(),
        }
    }
}

impl Default for EntropeRustParams {
    fn default() -> Self {
        Self {
            editor_state: editor::default_state(),
            // This gain is stored as linear gain. NIH-plug comes with useful conversion functions
            // to treat these kinds of parameters as if we were dealing with decibels. Storing this
            // as decibels is easier to work with, but requires a conversion for every sample.
            crush: FloatParam::new(
                "Crush",
                32.0,
                FloatRange::Linear {
                    min: 2.0,
                    max: 32.0,
                },
            ),
            redux: IntParam::new("Redux", 1, IntRange::Linear { min: 1, max: 100 }),
            entropy: IntParam::new("Entropy", 1, IntRange::Linear { min: 1, max: 100 }),
            clip: FloatParam::new("Clip", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
        }
    }
}

impl Plugin for EntropeRust {
    const NAME: &'static str = "Entrope";
    const VENDOR: &'static str = "DIY Studios";
    const URL: &'static str = env!("CARGO_PKG_HOMEPAGE");
    const EMAIL: &'static str = "andrew.r.j.thomas@gmail.com";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    // The first audio IO layout is used as the default. The other layouts may be selected either
    // explicitly or automatically by the host or the user depending on the plugin API/backend.
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(1),
            main_output_channels: NonZeroU32::new(1),
            ..AudioIOLayout::const_default()
        },
    ];

    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    // If the plugin can send or receive SysEx messages, it can define a type to wrap around those
    // messages here. The type implements the `SysExMessage` trait, which allows conversion to and
    // from plain byte buffers.
    type SysExMessage = ();
    // More advanced plugins can use this to run expensive background tasks. See the field's
    // documentation for more information. `()` means that the plugin does not have any background
    // tasks.
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor::create(self.params.clone(), self.params.editor_state.clone())
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        _buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        true
    }

    fn reset(&mut self) {
        // Reset buffers and envelopes here. This can be called from the audio thread and may not
        // allocate. You can remove this function if you do not need it.
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let mut crush = self.params.crush.value();
        let redux = self.params.redux.value();
        let entropy = self.params.entropy.value();
        let clip = self.params.clip.value();
        let mut clip_max = 0.0;
        let mut clip_min = 0.0;

        if entropy > 1 {
            let n = self.gen.gen_range(1..entropy);
            crush = crush / n as f32;
            //redux = redux * n;
        }

        if clip < 1.0 {
            let mut max: f32 = 0.0;
            let mut min: f32 = 0.0;
            for sample in buffer.as_slice_immutable().concat() {
                if sample < max {
                    max = sample
                }
                if sample > min {
                    min = sample
                }
            }

            clip_max = clip * max;
            clip_min = clip * min;
        }

        // TODO still kinda seems like this is happening per channel
        let mut reduced: f32 = 0.0;

        for (i, channel_samples) in buffer.iter_samples().enumerate() {
            for sample in channel_samples.into_iter() {
                let base: f32 = 2.0;
                let total_q_levels = base.powf(crush);

                let remainder = *sample % (1.0 / total_q_levels);

                *sample -= remainder;

                if redux > 1 {
                    let modulo = i as i32 % redux;
                    if modulo != 0 {
                        *sample = reduced;
                    } else {
                        reduced = *sample;
                    }
                }

                if clip_max != 0.0 && *sample < clip_max {
                    *sample = clip_max
                }
                if clip_min != 0.0 && *sample > clip_min {
                    *sample = clip_min
                }
            }
        }

        ProcessStatus::Normal
    }
}

impl Vst3Plugin for EntropeRust {
    const VST3_CLASS_ID: [u8; 16] = *b"Exactly16Chars!!";

    // And also don't forget to change these categories
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Dynamics];
}

nih_export_vst3!(EntropeRust);
