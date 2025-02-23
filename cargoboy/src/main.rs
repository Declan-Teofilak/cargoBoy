use std::collections::HashMap;
use std::fs;
use macroquad::prelude::*;
use std::thread::sleep;
use std::time::Duration;
use macroquad::miniquad::window::set_window_size;

const MEM_SIZE: usize = 4096;
const ROM_START: usize = 0x200;

struct chip8 {
    memory: [u8;4096],
    registers: [u8; 16],
    program_counter: u16,
    i_register: u16,
    // not used currently
    opcode_handlers: HashMap<u16, fn(&mut chip8)>,
    font: [u16;80],
    // for handling sub-routines
    stack: Vec<u16>,
}

// create our chip8 struct
impl chip8 {
    fn new() -> chip8 {
        let mut vm = chip8 {
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
            let mut x_coord = (self.registers[y as usize] % 64) as f32;
            let mut y_coord = (self.registers[x as usize] % 32) as f32;
            let pixel_height:f32  = n as f32;
            // let resolution_mod = 10.0;

            for row_num in 0..n {
                let sprite_row = self.memory[self.i_register as usize + row_num as usize];
                // we use val 128 (1000000) for the mask, and bitwise shift right every op
                // until we are at 0
                let mut mask_checker = 0x80;
                while mask_checker != 0 {
                    mask_checker = mask_checker >> 1;
                    if (sprite_row & mask_checker) == 0 {
                        draw_rectangle(x_coord, y_coord, 1.0, pixel_height, WHITE);
                    }
                    else {
                        draw_rectangle(x_coord, y_coord, 1.0, pixel_height, BLACK);
                    }

                    x_coord += 1.0;
                }

                y_coord += 1.0;
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

        return op_code;
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
    set_window_size(64 * 10, 32 * 10);
    let mut chip8 = chip8::new();
    chip8.prep_rom();
    clear_background(BLACK);

    loop {
        let current_instruction = chip8.fetch();
        chip8.decode_and_execute(current_instruction);

        next_frame().await;

        sleep(Duration::from_millis(60));
    }
}