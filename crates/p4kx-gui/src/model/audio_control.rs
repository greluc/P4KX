use super::Model;
use crate::{AudioOutput, AudioPreview, Browser, Player};

impl Model {
    /// Replaces the audio output sounds are played through, by default the system's default
    /// device.
    pub fn set_audio_output(&mut self, output: impl AudioOutput + 'static) {
        self.audio_output = Box::new(output);
    }

    /// Returns the volume of the audio preview, from 0 to 1.
    #[must_use]
    pub const fn audio_volume(&self) -> f32 {
        self.audio_volume
    }

    /// Sets the volume of the audio preview, clamped to 0 to 1, for the sound playing and those
    /// played later.
    pub fn set_audio_volume(&mut self, volume: f32) {
        let volume = volume.clamp(0.0, 1.0);
        self.audio_volume = volume;
        if let Some(player) = self.player_mut() {
            player.set_volume(volume);
        }
    }

    /// Plays the decoded sound of the selected WEM through the audio output, or pauses it while
    /// it plays; does nothing before it is decoded.
    pub fn toggle_playback(&mut self) {
        let volume = self.audio_volume;
        let Some(player) = self
            .browser
            .as_mut()
            .and_then(Browser::audio_mut)
            .and_then(AudioPreview::player_mut)
        else {
            return;
        };
        if player.is_playing() {
            player.pause();
        } else {
            player.set_volume(volume);
            player.play(self.audio_output.as_ref());
        }
    }

    /// Moves the sound of the selected WEM to `fraction` of its length.
    pub fn seek_playback(&mut self, fraction: f32) {
        if let Some(player) = self.player_mut() {
            player.seek(fraction);
        }
    }

    fn player_mut(&mut self) -> Option<&mut Player> {
        self.browser
            .as_mut()
            .and_then(Browser::audio_mut)
            .and_then(AudioPreview::player_mut)
    }
}
