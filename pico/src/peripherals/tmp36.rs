use cortex_m::prelude::*;
use rp_pico::hal;
use rp_pico::hal::gpio;

// This is specific to the rp2040 board we're using
type GpioPin = hal::gpio::Pin<gpio::bank0::Gpio26, gpio::FunctionNull, gpio::PullDown>;

pub struct Tmp36Sensor {
    adc: hal::Adc,
    sensor: hal::adc::TempSense,
    adc_pin_0: hal::adc::AdcPin<GpioPin>,
}

impl Tmp36Sensor {
    pub fn new(mut adc: hal::Adc, pin: GpioPin) -> Self {
        let sensor: hal::adc::TempSense = adc.take_temp_sensor().unwrap();
        let adc_pin_0 = hal::adc::AdcPin::new(pin).unwrap();

        Self {
            adc,
            sensor,
            adc_pin_0,
        }
    }

    pub fn chip_fahrenheit(&mut self) -> f32 {
        let chip_voltage_24bit: u16 = self.adc.read(&mut self.sensor).unwrap();
        chip_f(chip_voltage_24bit)
    }

    pub fn read_fahrenheit(&mut self) -> f32 {
        let tmp36_voltage_24bit: u16 = self.adc.read(&mut self.adc_pin_0).unwrap();
        tmp36_f(tmp36_voltage_24bit)
    }
}

/// Convert the voltage from a TMP36 sensor into a temperature reading.
///
/// The sensor returns 0.5V at 0°C and voltage changes ±0.01V for every
/// degree Celcius with higher temps resolting in higher voltages within
/// the range of -40°C to 125°C.
fn tmp36_f(adc_reading: u16) -> f32 {
    let voltage: f32 = adc_reading_to_voltage(adc_reading);
    let c = (100.0 * voltage) - 50.0;
    c_to_f(c)
}

/// Convert the voltage from the onboard temp sensor into a temp reading.
///
/// From §4.9.5 from the rp2040-datasheet.pdf, the temperature can be
/// approximated as T = 27 - (ADC_voltage - 0.706) / 0.001721.
fn chip_f(adc_reading: u16) -> f32 {
    let voltage: f32 = adc_reading_to_voltage(adc_reading);
    let c: f32 = 27.0 - ((voltage - 0.706) / 0.001721);
    c_to_f(c)
}

/// Basic Celsius-to-Fahrenheit conversion
fn c_to_f(c: f32) -> f32 {
    (c * 9.0 / 5.0) + 32.0
}

/// Convert ADC binary value to a float voltage value.
///
/// The ADC has a 12-bit resolution of voltage, meaning that there
/// are 2^12 or 4096 unique levels from OFF (0V) to FULL (3V). This
/// function converts the ADC reading into a float measurement in volts.
fn adc_reading_to_voltage(reading_12bit: u16) -> f32 {
    const REFERENCE_VOLTAGE: f32 = 3.3;
    const STEPS_12BIT: f32 = 4096.0;

    (reading_12bit as f32 / STEPS_12BIT) * REFERENCE_VOLTAGE
}
