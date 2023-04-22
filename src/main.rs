#![allow(dead_code)]

use {
    core::time::Duration,
    esp_idf_hal::{
        gpio::AnyIOPin,
        i2s::{config as I2sConfig, I2sStdDriver, I2sTx, I2sTxChannel, I2sTxSupported},
        prelude::*,
    },
    esp_idf_svc::log::{set_target_level, EspLogger},
    esp_idf_sys::EspError,
    log::{debug, LevelFilter},
    std::f32::consts::TAU,
};

const TIMEOUT: Duration = Duration::from_millis(100);
const SAMPLE_RATE_HZ: u32 = 16000;
const OMEGA_INC: f32 = TAU / SAMPLE_RATE_HZ as f32;
const BITS_PER_SAMPLE: I2sConfig::DataBitWidth = I2sConfig::DataBitWidth::Bits16;
const DMA_BUFFERS: usize = 12;
const DMA_FRAMES: usize = 240;

struct SendSinewave {
    freq: f32,
    omega: f32, // t * TAU
    buffers: Vec<u8>,
}

impl SendSinewave {
    fn new(freq: f32) -> Self {
        Self {
            freq,
            omega: 0.0,
            buffers: vec![0; DMA_BUFFERS * DMA_FRAMES * 4],
        }
    }
}

impl SendSinewave {
    pub fn send<Dir: I2sTxSupported>(
        &mut self,
        driver: &mut I2sStdDriver<'_, Dir>,
        n_buffers: usize,
    ) -> Result<usize, EspError> {
        for i in (0..n_buffers * DMA_FRAMES * 4).step_by(4) {
            let lsample = ((self.omega * self.freq).sin() * 0.5 * (i16::MAX as f32)) as u16;

            self.buffers[i] = (lsample & 0x00ff) as u8;
            self.buffers[i + 1] = ((lsample & 0xff00) >> 8) as u8;
            self.buffers[i + 2] = (lsample & 0x00ff) as u8;
            self.buffers[i + 3] = ((lsample & 0xff00) >> 8) as u8;
            self.omega += OMEGA_INC;

            if self.omega >= TAU {
                self.omega -= TAU;
            }
        }

        driver.write(&self.buffers[..n_buffers * DMA_FRAMES * 4], TIMEOUT)
    }
}

struct SendTriangleWave {
    buffer: Vec<u8>,
}

impl SendTriangleWave {
    fn new(freq: f32) -> Self {
        let buffer_size = (SAMPLE_RATE_HZ as f32 / freq) as usize;
        let mut buffer = vec![0; buffer_size * 4];
        let mut value: f32 = 0.0;
        let mut value_inc = 0.1 / (buffer_size as f32);

        for i in (0..buffer.len()).step_by(4) {
            let i_value = (value * (i16::MAX as f32)) as i16 as u16;

            buffer[i] = (i_value & 0x00ff) as u8;
            buffer[i + 1] = ((i_value & 0xff00) >> 8) as u8;
            buffer[i + 2] = (i_value & 0x00ff) as u8;
            buffer[i + 3] = ((i_value & 0xff00) >> 8) as u8;
            value += value_inc;

            if value_inc > 0.0 && value > 1.0 {
                value = 2.0 - value;
                value_inc = -value_inc;
            } else if value_inc < 0.0 && value < 1.0 {
                value = -2.0 - value;
                value_inc = -value_inc;
            }
        }

        Self {
            buffer,
        }
    }
}

impl SendTriangleWave {
    pub fn send<Dir: I2sTxSupported>(&mut self, driver: &mut I2sStdDriver<'_, Dir>) -> Result<usize, EspError> {
        driver.write(&self.buffer, TIMEOUT)
    }
}

fn main() {
    esp_idf_sys::link_patches();
    EspLogger::initialize_default();
    set_target_level("*", LevelFilter::Trace);

    std::env::set_var("RUST_BACKTRACE", "1");
    debug!("esp_log_write: Starting application");

    println!("Starting application");
    let peripherals = Peripherals::take().unwrap();
    let i2s_config = I2sConfig::Config::default().dma_desc(DMA_BUFFERS as u32);
    let clk_config = I2sConfig::StdClkConfig::from_sample_rate_hz(SAMPLE_RATE_HZ).clk_src(I2sConfig::ClockSource::Apll);
    let gpio_config = I2sConfig::StdGpioConfig::default();
    let slot_config = I2sConfig::StdSlotConfig::philips_slot_default(BITS_PER_SAMPLE, I2sConfig::SlotMode::Stereo);
    let std_config = I2sConfig::StdConfig::new(i2s_config, clk_config, slot_config, gpio_config);

    println!("Initializing I2S driver");
    let bclk = peripherals.pins.gpio2;
    let dout = peripherals.pins.gpio4;
    let ws = peripherals.pins.gpio1;
    let mut i2s =
        I2sStdDriver::<I2sTx>::new_tx(peripherals.i2s0, std_config, bclk, Some(dout), AnyIOPin::none(), ws).unwrap();

    let mut wave = SendTriangleWave::new(440.0);
    println!("Enabling output");
    i2s.tx_enable().unwrap();

    println!("Starting transmission");

    loop {
        wave.send(&mut i2s).unwrap();
    }
}
