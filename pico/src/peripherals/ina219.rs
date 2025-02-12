use embedded_hal::i2c::I2c;

pub struct INA219<I2C> {
    i2c: I2C,
    addr: u8,
    cal_value: u16,
}

impl<I2C> INA219<I2C>
where
    I2C: I2c,
{
    pub fn new(i2c: I2C) -> Self {
        const INA219_DEFAULT_ADDR: u8 = 0x43;

        Self::new_with_addr(i2c, INA219_DEFAULT_ADDR)
    }

    pub fn new_with_addr(i2c: I2C, addr: u8) -> Self {
        let mut this = Self {
            i2c,
            addr,
            cal_value: 4096,
        };
        this.set_configuration();
        this
    }

    pub fn free(self) -> (I2C,) {
        let Self { i2c, .. } = self;
        (i2c,)
    }

    pub fn bus_voltage(&mut self) -> u16 {
        let mut voltage = 0_u16;

        // REG_BUSVOLTAGE
        self.read(0x02, &mut voltage);

        // From the datasheet:
        //     The Bus Voltage register bits are not right-aligned. In order to compute the value of
        //     the Bus Voltage, Bus Voltage Register contents must be shifted right by three bits.
        //     This shift puts the BD0 bit in the LSB position so that the contents can be multiplied
        //     by the Bus Voltage LSB of 4-mV to compute the bus voltage measured by the device.
        (voltage >> 3) / 25
    }

    pub fn current_milliamps(&mut self) -> u16 {
        let mut m_amps = 0_u16;

        // TODO: Cite docs on why we recalibate per read
        // REG_CALIBRATION
        self.write(0x05, self.cal_value);

        // REG_CURRENT
        self.read(0x04, &mut m_amps);

        // TODO: Clarify scaling and units of this
        m_amps
    }

    pub fn shunt_voltage(&mut self) -> i16 {
        let mut value = 0_u16;
        self.read(0x01, &mut value) as i16
    }

    pub fn power(&mut self) -> i16 {
        let mut value = 0_u16;
        self.read(0x03, &mut value) as i16
    }

    pub fn current(&mut self) -> i16 {
        let mut value = 0_u16;
        self.read(0x04, &mut value) as i16
    }

    pub fn write(&mut self, reg: u8, value: u16) {
        let bytes = [
            reg,                    //
            value.to_be_bytes()[0], //
            value.to_be_bytes()[1], //
        ];

        let _ = self.i2c.write(self.addr, &bytes);
    }

    pub fn read(&mut self, reg: u8, value: &mut u16) -> u16 {
        let mut bytes = [0_u8; 2];
        let _ = self.i2c.write_read(self.addr, &[reg], &mut bytes);

        // "All data bytes are transmitted most significant byte first"
        *value = u16::from_be_bytes(bytes);
        *value
    }

    fn set_calibation(&mut self) {
        // REG_CALIBRATION
        self.write(0x05, self.cal_value);
    }

    fn set_configuration(&mut self) {
        #![allow(dead_code)]

        // 0-16V Range
        const INA219_CONFIG_BVOLTAGERANGE_16V: u16 = 0x0000;
        // 0-32V Range
        const INA219_CONFIG_BVOLTAGERANGE_32V: u16 = 0x2000;

        // Gain 1, 40mV Range
        const INA219_CONFIG_GAIN_1_40MV: u16 = 0x0000;
        // Gain 2, 80mV Range
        const INA219_CONFIG_GAIN_2_80MV: u16 = 0x0800;
        // Gain 4, 160mV Range
        const INA219_CONFIG_GAIN_4_160MV: u16 = 0x1000;
        // Gain 8, 320mV Range
        const INA219_CONFIG_GAIN_8_320MV: u16 = 0x1800;

        // 9-bit bus res = 0..511
        const INA219_CONFIG_BADCRES_9BIT: u16 = 0x0000;
        // 10-bit bus res = 0..1023
        const INA219_CONFIG_BADCRES_10BIT: u16 = 0x0080;
        // 11-bit bus res = 0..2047
        const INA219_CONFIG_BADCRES_11BIT: u16 = 0x0100;
        // 12-bit bus res = 0..4097
        const INA219_CONFIG_BADCRES_12BIT: u16 = 0x0180;

        // 1 x 9-bit shunt sample
        const INA219_CONFIG_SADCRES_9BIT_1S_84US: u16 = 0x0000;
        // 1 x 10-bit shunt sample
        const INA219_CONFIG_SADCRES_10BIT_1S_148US: u16 = 0x0008;
        // 1 x 11-bit shunt sample
        const INA219_CONFIG_SADCRES_11BIT_1S_276US: u16 = 0x0010;
        // 1 x 12-bit shunt sample
        const INA219_CONFIG_SADCRES_12BIT_1S_532US: u16 = 0x0018;
        // 2 x 12-bit shunt samples averaged together
        const INA219_CONFIG_SADCRES_12BIT_2S_1060US: u16 = 0x0048;
        // 4 x 12-bit shunt samples averaged together
        const INA219_CONFIG_SADCRES_12BIT_4S_2130US: u16 = 0x0050;
        // 8 x 12-bit shunt samples averaged together
        const INA219_CONFIG_SADCRES_12BIT_8S_4260US: u16 = 0x0058;
        // 16 x 12-bit shunt samples averaged together
        const INA219_CONFIG_SADCRES_12BIT_16S_8510US: u16 = 0x0060;
        // 32 x 12-bit shunt samples averaged together
        const INA219_CONFIG_SADCRES_12BIT_32S_17MS: u16 = 0x0068;
        // 64 x 12-bit shunt samples averaged together
        const INA219_CONFIG_SADCRES_12BIT_64S_34MS: u16 = 0x0070;
        // 128 x 12-bit shunt samples averaged together
        const INA219_CONFIG_SADCRES_12BIT_128S_69MS: u16 = 0x0078;

        const INA219_CONFIG_MODE_SANDBVOLT_CONTINUOUS: u16 = 7;

        self.set_calibation();

        // Set Config register to take into account the settings above
        let config = INA219_CONFIG_BVOLTAGERANGE_32V
            | INA219_CONFIG_GAIN_8_320MV
            | INA219_CONFIG_BADCRES_12BIT
            | INA219_CONFIG_SADCRES_12BIT_32S_17MS
            | INA219_CONFIG_MODE_SANDBVOLT_CONTINUOUS;
        // REG_CONFIG
        self.write(0x00, config);
    }
}
