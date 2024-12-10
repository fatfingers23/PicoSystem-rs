//Not being used but may be needed later
pub struct Spi9Bit<'l> {
    sm: StateMachine<'l, PIO0, 0>,
}

impl<'l> Spi9Bit<'l> {
    pub fn new(
        pio: impl Peripheral<P = PIO0> + 'l,
        clk: impl PioPin,
        mosi: impl PioPin,
        cs: impl PioPin,
    ) -> Spi9Bit<'l> {
        let Pio {
            mut common,
            mut sm0,
            ..
        } = Pio::new(pio, Irqs);

        let prg = pio_proc::pio_asm!(
            r#"
            .side_set 2
            .wrap_target

            bitloop:
                out pins, 1        side 0x0
                jmp !osre bitloop  side 0x1     ; Fall-through if TXF empties
                nop                side 0x0 [1] ; CSn back porch

            public entry_point:                 ; Must set X,Y to n-2 before starting!
                pull ifempty       side 0x2 [1] ; Block with CSn high (minimum 2 cycles)

            .wrap                               ; Note ifempty to avoid time-of-check race

            "#,
        );
        let program = prg.program;

        let clk = common.make_pio_pin(clk);
        let mosi = common.make_pio_pin(mosi);
        let cs = common.make_pio_pin(cs);

        sm0.set_pin_dirs(embassy_rp::pio::Direction::Out, &[&clk, &mosi, &cs]);

        // let relocated = RelocatedProgram::new(&prg.program);
        let mut cfg = embassy_rp::pio::Config::default();
        let relocated = common.load_program(&program);
        // cs:  side set 0b10
        // clk: side set 0b01
        // fist side_set, lower bit in side_set
        cfg.use_program(&relocated, &[&clk, &cs]);

        cfg.clock_divider = 1u8.into(); // run at full speed
        cfg.set_out_pins(&[&mosi]);
        //  cfg.set_set_pins(&[&mosi]);
        cfg.shift_out = ShiftConfig {
            auto_fill: false,
            direction: ShiftDirection::Left,
            threshold: 9, // 9-bit mode
        };
        cfg.fifo_join = FifoJoin::TxOnly;
        sm0.set_config(&cfg);

        sm0.set_enable(true);

        Self { sm: sm0 }
    }

    #[inline]
    pub fn write_data(&mut self, val: u8) {
        // no need to busy wait
        while self.sm.tx().full() {}
        self.sm.tx().push(0x80000000 | ((val as u32) << 23));
    }

    #[inline]
    pub fn write_command(&mut self, val: u8) {
        while self.sm.tx().full() {}
        self.sm.tx().push((val as u32) << 23);
    }
}

impl<'l> WriteOnlyDataCommand for Spi9Bit<'l> {
    fn send_commands(&mut self, cmd: DataFormat<'_>) -> Result<(), DisplayError> {
        match cmd {
            DataFormat::U8(cmds) => {
                for &c in cmds {
                    self.write_command(c);
                }
            }
            _ => {
                defmt::todo!();
            }
        }
        Ok(())
    }

    fn send_data(&mut self, buf: DataFormat<'_>) -> Result<(), DisplayError> {
        match buf {
            DataFormat::U8(buf) => {
                for &byte in buf {
                    self.write_data(byte);
                }
            }
            DataFormat::U16BEIter(it) => {
                for raw in it {
                    self.write_data((raw >> 8) as u8);
                    self.write_data((raw & 0xff) as u8);
                }
            }
            _ => {
                defmt::todo!();
            }
        }

        Ok(())
    }
}
