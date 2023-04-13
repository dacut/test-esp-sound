use {
    core::{hint::spin_loop, sync::atomic::{AtomicBool, Ordering}, time::Duration},
    esp_idf_hal::{
        gpio::{Gpio1, Gpio2, Gpio4, AnyIOPin},
        i2s::{config as I2sConfig, I2sStdDriver, I2sEvent, I2sTx, I2sStdTxCallback, I2sTxChannel },
        prelude::*,
    },
    std::f32::consts::TAU,
};

const TIMEOUT: Duration = Duration::from_millis(100);
const SAMPLE_RATE_HZ: u32 = 48000;
const BITS_PER_SAMPLE: I2sConfig::DataBitWidth = I2sConfig::DataBitWidth::Bits16;

struct SendSinewave {
    samples: Vec<u8>,
}

struct SendSinewaveCallback {
    ready: AtomicBool,
}

impl Default for SendSinewaveCallback {
    fn default() -> Self {
        Self {
            ready: AtomicBool::new(true),
        }
    }
}

impl I2sStdTxCallback for SendSinewaveCallback {
    fn on_sent(&self, _port: u8, _event: &I2sEvent) -> bool {
        self.ready.store(true, Ordering::Relaxed);
        false
    }
}

impl SendSinewave {
    fn new(_freq: u32) -> Self {
        // let mut sample_size = SAMPLE_RATE_HZ as usize / freq as usize;
        // let period_remainder = SAMPLE_RATE_HZ as usize % freq as usize;

        // if period_remainder > 0 {
        //     // Extend the sample so we get an integral number of periods.
        //     sample_size += freq as usize / period_remainder + 1;
        // }

        // let mut samples: Vec<u8> = vec![0; sample_size * 4];y

        // let omega = TAU * freq as f32;

        // for i in (0..sample_size).step_by(4) {
        //     let t = omega * freq as f32 * i as f32;
        //     let value = ((omega * t * 0.2).sin() * i16::MAX as f32) as i16;
        //     let value_high = ((value as u16) >> 8) as u8;
        //     let value_low = ((value as u16) & 0xff) as u8;

        //     samples[i] = value_high;
        //     samples[i + 1] = value_low;
        //     samples[i + 2] = value_high;
        //     samples[i + 3] = value_low;
        // }

        // Self {
        //     samples,
        // }
        let mut samples = vec![0; 4096];
        for i in (0..samples.len()).step_by(4) {
            samples[i] = 0b10101010;
            samples[i + 1] = 0b10101010;
            samples[i + 2] = 0b11001100;
            samples[i + 3] = 0b11001100;
        }

        Self { samples }
    }

    fn run_forever(&mut self, tx_channel: &mut dyn I2sTxChannel) {
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
    let config = I2sConfig::Config::default();
    let tx_clk_config = I2sConfig::StdClkConfig::from_sample_rate_hz(SAMPLE_RATE_HZ);
    let tx_slot_config =
        I2sConfig::StdSlotConfig::philips_slot_default(BITS_PER_SAMPLE, I2sConfig::SlotMode::Stereo);
    let tx_gpio_config = I2sConfig::StdGpioConfig::default();
    let std_config = I2sConfig::StdConfig::new(config, tx_clk_config, tx_slot_config, tx_gpio_config);

    println!("Initializing I2S driver");
    let bclk = unsafe { Gpio2::new() };
    let dout = unsafe { Gpio4::new() };
    let ws = unsafe { Gpio1::new() };
    let mut i2s = I2sStdDriver::<I2sTx>::new_tx(peripherals.i2s0, std_config, bclk, Some(dout), AnyIOPin::none(), ws, None).unwrap();

    println!("Enabling transmit channel");
    let mut send_sine = SendSinewave::new(256); // middle C
    // let callback = SendSinewaveCallback::default();
    i2s.tx_enable().unwrap();

    std::env::set_var("RUST_BACKTRACE", "1");
    println!("Starting transmission");
    send_sine.run_forever(&mut i2s);

    println!("Program exited unexpectedly. Halting.");
    i2s.tx_disable().unwrap();
    drop(i2s);

    loop {
        spin_loop();
    }
}
