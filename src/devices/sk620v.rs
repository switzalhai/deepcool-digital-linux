//! Display module for:
//! - Sudokoo SK620V

use crate::monitor::cpu::Cpu;
use super::{device_error, Mode};
use hidapi::HidApi;
use std::{thread::sleep, time::Duration};

pub const DEFAULT_MODE: Mode = Mode::Auto;
// Wattage that fills the power bar under the wattage reading
const POWER_BAR_MAX_W: u16 = 200;

pub struct Display {
    cpu: Cpu,
    update: Duration,
    fahrenheit: bool,
}

impl Display {
    pub fn new(cpu: Cpu, update: Duration, fahrenheit: bool) -> Self {
        Display {
            cpu,
            update,
            fahrenheit,
        }
    }

    pub fn run(&self, api: &HidApi, vid: u16, pid: u16) {
        // Connect to device
        let device = api.open(vid, pid).unwrap_or_else(|_| device_error());

        // Display warning if a required module is missing
        self.cpu.warn_temp();
        self.cpu.warn_rapl();

        // Data packet
        let mut data: [u8; 64] = [0; 64];
        data[0] = 16;
        data[1] = 104;
        data[2] = 1;
        data[3] = 9;
        data[4] = 13;
        data[5] = 1;

        // Init sequence (show flag 0 clears the display)
        for _ in 0..3 {
            device.write(&Self::finalize(data.clone())).unwrap();
            sleep(Duration::from_millis(200));
        }
        data[6] = 2;

        // Display loop
        loop {
            // Initialize the packet
            let mut status_data = data.clone();

            // Read CPU utilization & energy consumption
            let cpu_instant = self.cpu.read_instant();
            let cpu_energy = self.cpu.read_energy();

            // Wait
            sleep(self.update);

            // ----- Write data to the package -----
            // Power consumption & power bar
            let power = self.cpu.get_power(cpu_energy, self.update.as_millis() as u64);
            status_data[7..=8].copy_from_slice(&power.to_be_bytes());
            status_data[9] = (power.min(POWER_BAR_MAX_W) as u32 * 100 / POWER_BAR_MAX_W as u32) as u8;

            // Temperature
            status_data[10] = if self.fahrenheit { 1 } else { 0 };
            status_data[11..=14].copy_from_slice(&(self.cpu.get_temp(self.fahrenheit) as f32).to_be_bytes());

            // Utilization
            status_data[15] = self.cpu.get_usage(cpu_instant);

            // Frequency
            status_data[16..=17].copy_from_slice(&self.cpu.get_frequency().to_be_bytes());

            device.write(&Self::finalize(status_data)).unwrap();
        }
    }

    /// Adds the checksum & termination byte.
    fn finalize(mut data: [u8; 64]) -> [u8; 64] {
        let checksum: u16 = data[1..=17].iter().map(|&x| x as u16).sum();
        data[18] = (checksum % 256) as u8;
        data[19] = 22;
        data
    }
}
