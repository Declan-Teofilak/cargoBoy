use std::collections::HashMap;
use std::fs;
use macroquad::prelude::*;
use std::thread::sleep;
use std::time::Duration;
use macroquad::miniquad::window::set_window_size;
use macroquad::prelude::scene::clear;

const MEM_SIZE: usize = 4096;
const ROM_START: usize = 0x200;

struct Chip8 {
    memory: [u8;4096],
    registers: [u8; 16],
    program_counter: u16,
    i_register: u16,
    // not used currently
    opcode_handlers: HashMap<u16, fn(&mut Chip8)>,
    font: [u16;80],
    // for handling sub-routines
    stack: Vec<u16>,
    screen_values: ScreenData,
}

struct ScreenData {
    x_values: [u8;640],
    y_values: [u8;320],
}

impl ScreenData {
    fn new() -> Self {
        let mut instance = ScreenData {
            x_values: [0;640],
            y_values: [0;320]
        };

        instance
    }
}

// create our chip8 struct
impl Chip8 {
    fn new() -> Chip8 {
        let mut vm = Chip8 {
            memory: [0;4096],
            registers: [0; 16],
            i_register: 0, // general purpose index register?
            program_counter: 0x200, // mem start location
            opcode_handlers: HashMap::new(),
            font: [0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
                0x20, 0x60, 0x20, 0x20, 0x70, // 1
                0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
                0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
                0x90, 0x90, 0xF0, 0x10, 0x10, // 4
                0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
                0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
                0xF0, 0x10, 0x20, 0x40, 0x40, // 7
                0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
                0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
                0xF0, 0x90, 0xF0, 0x90, 0x90, // A
                0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
                0xF0, 0x80, 0x80, 0x80, 0xF0, // C
                0xE0, 0x90, 0x90, 0x90, 0xE0, // D
                0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
                0xF0, 0x80, 0xF0, 0x80, 0x80  // F
            ],
            stack: Vec::with_capacity(MEM_SIZE),
            screen_values: ScreenData {
                x_values: [0;640],
                y_values: [0;320],
            }
        };

        vm.prepare_opcodes();

        vm
    }

    fn decode_and_execute(&mut self, opcode: u16) {
        // switch statement examining the first nibble of the instruction
        // WHat we are doing here is using the bitwise & against the hexvalue -xf000 (1111 0000 0000 0000)
        // to MASK the values beyond the first 4 bits in our opcode
        // match opcode & 0xF000 {
        //     0 => return 1;
        // }

        // Start by getting all the relevant bit values
        println!("opcode: {}", opcode);
        let nnn = opcode & 0x0FFF;
        let nn = (opcode & 0x00FF);
        let n = (opcode & 0x000F);
        let x = ((opcode & 0x0F00) >> 8);
        let y = ((opcode & 0x00F0) >> 4);

        // jump to NNN
        if opcode & 0xF000 == 0x1000 {
            self.program_counter = nnn;
        }
        // jump
        else if opcode & 0xF000 == 0x0000 {
            clear_background(BLACK);
        }
        // set register x to value nn
        else if opcode & 0xF000 == 0x6000 {
            self.registers[x as usize] = nn as u8;
        }
        // add to vx
        else if opcode & 0xF000 == 0x7000 {
            self.registers[x as usize] = (self.registers[x as usize]) + nn as u8;
        }
        // draw
        else if opcode & 0xF000 == 0xD000 {
            // prep our coordinates (ensuring that we prevent clipping with the mod (w/h)
            let mut x_coord = (self.registers[x as usize] % 64) as f32;
            let mut y_coord = (self.registers[y as usize] % 32) as f32;
            let resolution_mod = 10.0;
            let pixel_height:f32  = n as f32;
            x_coord *= resolution_mod;
            y_coord *= resolution_mod;

            for row_num in 0..n {
                let sprite_row = self.memory[self.i_register as usize + row_num as usize];

                for row_bit in (0..8).rev() {
                    //mask the first bit
                    let sprite_data = (sprite_row >> row_bit) & 1;

                    // fetch the drawing coords based on WHICH BIT we are currently operating on and the row we found it on
                    let draw_x = x_coord + ((7 - row_bit) as f32 * resolution_mod);
                    let draw_y = y_coord + ((row_num as f32) * resolution_mod);

                    if draw_x < 640.0 && draw_y < 320.0 {
                        // if the masked bit is a 1, we check to see if we are turning the bit off (it is already on) or turning it on
                        if sprite_data == 1 {
                            if (self.screen_values.x_values[draw_x as usize] == 1 && self.screen_values.y_values[draw_y as usize] == 1) {
                                draw_rectangle(draw_x , draw_y, 10.0, pixel_height, BLACK);
                                self.screen_values.x_values[draw_x as usize] = 0;
                                self.screen_values.y_values[draw_y as usize] = 0;
                            }
                            else {
                                draw_rectangle(draw_x , draw_y, 10.0, pixel_height, GREEN);
                                self.screen_values.x_values[draw_x as usize] = 1;
                                self.screen_values.y_values[draw_y as usize] = 1;
                            }
                        }
                    }
                }
            }
        }
        else if opcode & 0xF000 == 0xA000 {
            self.i_register = nnn;
        }
    }

    fn fetch(&mut self) -> u16 {
        println!("Entered the fetch function");
        println!("______________________");
        // examine the current program counter (PC) instruction
        // 2 bytes
        // increment the PC by 2 bytes to move to next instruction

        // Why use (high_byte << 8) | low_byte?
        // This operation is a way to combine two bytes into a single 16-bit value. In the context of CHIP-8 (and many other systems), opcodes are stored as two bytes, and we need to reassemble them into a full 16-bit opcode.
        //    high_byte is the most significant byte (MSB).
        //    low_byte is the least significant byte (LSB).
        //    By shifting high_byte to the left and then OR'ing it with low_byte, we get the complete 16-bit value for the opcode.
        println!("PC before fetch: {}", self.program_counter);
        // fetch most sig bit
        let high = self.memory[self.program_counter as usize] as u16;
        // fetch least sig bit (remember, each opcode is 16 bits)
        let low = self.memory[(self.program_counter + 1) as usize] as u16;

        self.program_counter = self.program_counter + 2;

        // perform operation on the high and low to create the 16 bit opcode
        // we shift the high bit left, then OR it with the low bit
        let op_code:u16 = (high << 8) | low;

        println!("current op_code: {}", op_code);
        println!("PC after fetch: {}", self.program_counter);

        op_code
    }

    // want to create a map of each opCode, but not worth it lol
    fn prepare_opcodes(&self) {
       // self.opcode_handlers.insert(00E0, fn() -> () { clear_background(BLACK)});
    }

    fn prep_rom(&mut self) {
        let data = fs::read("roms/ibm_logo.ch8");

        match data {
            Ok(data) => {
                if 0x200 + data.len() > self.memory.len() {
                    println!("Error: ROM too large");
                    return;
                }

                self.memory[0x200..0x200 + data.len()].copy_from_slice(&data);
                self.program_counter = 0x200;
            }
            Err(e) => {
                println!("Error loading ROM: {}", e);
            }
        }
    }

}

#[macroquad::main("chip8")]
async fn main() {
    set_window_size(640, 320);
    let mut chip8 = Chip8::new();
    chip8.prep_rom();

    loop {
        let current_instruction = chip8.fetch();
        chip8.decode_and_execute(current_instruction);

        next_frame().await;

        //sleep(Duration::from_millis(1));
    }
}