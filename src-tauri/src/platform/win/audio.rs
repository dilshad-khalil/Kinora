//! Loudness from the default output device's peak meter (the same meter as the volume mixer).

use ::windows::Win32::{
    Media::Audio::{eConsole, eRender, Endpoints::IAudioMeterInformation, IMMDeviceEnumerator, MMDeviceEnumerator},
    System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED},
};

use crate::platform::AudioMeter;

pub struct Meter(IAudioMeterInformation);

impl Meter {
    pub fn open() -> Option<Meter> {
        open().ok().map(Meter)
    }
}

impl AudioMeter for Meter {
    fn levels(&mut self) -> Option<(f32, Vec<f32>)> {
        read(&self.0).ok()
    }
}

fn open() -> ::windows::core::Result<IAudioMeterInformation> {
    unsafe {
        // Per thread, and harmless to repeat.
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let devices: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let device = devices.GetDefaultAudioEndpoint(eRender, eConsole)?;
        device.Activate(CLSCTX_ALL, None)
    }
}

fn read(meter: &IAudioMeterInformation) -> ::windows::core::Result<(f32, Vec<f32>)> {
    unsafe {
        let peak = meter.GetPeakValue()?;
        let count = meter.GetMeteringChannelCount()?.min(8) as usize;
        let mut channels = vec![0f32; count];
        if count > 0 {
            meter.GetChannelsPeakValues(&mut channels)?;
        }
        Ok((peak, channels))
    }
}
