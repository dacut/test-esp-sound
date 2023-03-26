use {
    core::{hint::spin_loop, sync::atomic::{AtomicBool, Ordering}, time::Duration},
    esp_idf_hal::{
        i2s::{config as I2sConfig, I2sDriver, I2sEvent, I2sTxCallback, I2sTxChannel, I2sChannel},
        prelude::*,
    },
    std::f32::consts::TAU,
};

const TIMEOUT: Duration = Duration::from_millis(100);
const SAMPLE_RATE_HZ: u32 = 48000;
const BITS_PER_SAMPLE: I2sConfig::DataBitWidth = I2sConfig::DataBitWidth::Bits16;

struct SendSinewave {
    samples: Vec<u8>,
    ready: AtomicBool,
}

impl I2sTxCallback for SendSinewave {
    fn on_sent(&mut self, _tx_channel: &I2sTxChannel<'_>, _event: &I2sEvent) -> bool {
        self.ready.store(true, Ordering::Relaxed);
        false
    }
}

impl SendSinewave {
    fn new(freq: u32) -> Self {
        let mut sample_size = SAMPLE_RATE_HZ as usize / freq as usize;
        let period_remainder = SAMPLE_RATE_HZ as usize % freq as usize;

        if period_remainder > 0 {
            // Extend the sample so we get an integral number of periods.
            sample_size += freq as usize / period_remainder + 1;
        }

        let mut samples: Vec<u8> = vec![0; sample_size * 4];

        let omega = TAU * freq as f32;

        for i in (0..sample_size).step_by(4) {
            let t = omega * freq as f32 * i as f32;
            let value = ((omega * t).sin() * i16::MAX as f32) as i16;
            let value_high = ((value as u16) >> 8) as u8;
            let value_low = ((value as u16) & 0xff) as u8;

            samples[i] = value_high;
            samples[i + 1] = value_low;
            samples[i + 2] = value_high;
            samples[i + 3] = value_low;
        }

        Self {
            samples,
            ready: AtomicBool::new(true),
        }
    }

    fn run_forever(&mut self, tx_channel: &mut I2sTxChannel<'_>) {
        loop {
            let mut total_written = 0;
            while total_written < self.samples.len() {
                let n_written = match tx_channel.write(&self.samples[total_written..], TIMEOUT) {
                    Ok(n_written) => n_written,
                    Err(e) => {
                        println!("Error writing to I2S: {e}");
                        return;
                    }
                };
                total_written += n_written;
            }
        }
    }
}

fn main() {
    esp_idf_sys::link_patches();

    println!("Starting application");
    let peripherals = Peripherals::take().unwrap();
    let config = I2sConfig::Config::new().channels(I2sConfig::ChannelOpen::Tx);

    println!("Initializing I2S driver");
    let mut i2s = I2sDriver::new(peripherals.i2s0, config).unwrap();

    println!("Opening transmit channel");
    let tx_clk_config = I2sConfig::StdClkConfig::from_sample_rate_hz(SAMPLE_RATE_HZ);
    let tx_slot_config =
        I2sConfig::StdSlotConfig::philips_slot_default(BITS_PER_SAMPLE, I2sConfig::SlotMode::Stereo);
    let tx_gpio_config = I2sConfig::StdGpioConfigBuilder::default()
        .bclk(peripherals.pins.gpio5)
        .ws(peripherals.pins.gpio7)
        .data_out(peripherals.pins.gpio3)
        .build();
    let tx_config = I2sConfig::StdChanConfig::new(tx_clk_config, tx_slot_config, tx_gpio_config);
    let mut tx = i2s.open_tx_channel(&tx_config).unwrap();

    println!("Enabling transmit channel");
    let mut send_sine = SendSinewave::new(256); // middle C
    // tx.set_callback_handler(&mut send_sine).unwrap();
    tx.enable().unwrap();

    std::env::set_var("RUST_BACKTRACE", "1");
    println!("Starting transmission");
    send_sine.run_forever(&mut tx);

    println!("Program exited unexpectedly. Halting.");
    tx.disable().unwrap();
    drop(tx);
    drop(i2s);

    loop {
        spin_loop();
    }
}
