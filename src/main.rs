use std::{convert::TryInto, fs::read, env};

// NAMED CONSTANTS============================================================
/* condition codes */
const EQ : u32 = 0;
const NE : u32 = 1;
const GE : u32 = 10;
const LT : u32 = 11;
const GT : u32 = 12;
const LE : u32 = 13;
const AL : u32 = 14;

/* opcodes */
const AND : u32 = 0;
const EOR : u32 = 1;
const SUB : u32 = 2;
const RSB : u32 = 3;
const ADD : u32 = 4;
const TST : u32 = 8;
const TEQ : u32 = 9;
const CMP : u32 = 10;
const ORR : u32 = 12;
const MOV : u32 = 13;

/* register alias */
const PC : usize = 15;

/* memory size (bytes) */
const MEMSIZE : usize = 0x8000;

// UTILITY FUNCTIONS============================================================
/* Return a range of bits:
data    <-  Source string of bits
start   <-  inclusive start
n       <-  number of bits */
fn get_bits(data : &u32, start : u32, n : u32) -> u32 {(data >> start) & ((1 << n) - 1)}

/* get bit at Location in a Word:
data    <-  the Word you are inspecting
n       <-  bit number (0-31) */
fn get_bit(data : &u32, n : u32) -> bool {(*data >> n) & 1 != 0}

/* Check the endian-ness of the system the emulator is being run on
return  <-  True (little endian), False (big endian) */
fn endian_check() -> bool {1u32.to_ne_bytes()[0] == 1}

// MACHINE STATE STRUCTS========================================================
struct Cpsr {
    n : bool,
    z : bool,
    c : bool,
    v : bool
}

struct CPU {
    registers : [u32; 16],
    cpsr : Cpsr,
    memory : Vec<u8>
}

// EMULATOR IMPLEMENTATION======================================================
impl CPU {

    /* Create a new CPU struct:
    return  <-  New CPU with registers, memory initialised */
    fn new() -> CPU {
        CPU {
            registers : [0; 16],
            cpsr : Cpsr {
                n : false,
                z : false,
                c : false,
                v : false
            },
            memory : vec![0; MEMSIZE]
        }
    }

    /* end emulator and display the state of the CPU 
    error   <- error message to display */
    fn fatal(&self, error : &str, data : &u32) {
        println!("Error: {}: {:#010x}", error, data);
        self.print_state();
        panic!();
    }
    
    /* Get the word at a given memory location
    loc     <-  location of the start of the 4 bytes in memory */
    fn get_mem_word(&self, loc : usize) -> u32 {u32::from_ne_bytes(self.memory[loc..loc+4].try_into().unwrap())}

    /* Get the word at a given memory location
    loc     <-  location of the start of the 4 bytes in memory
    val     <-  the value to be written */
    fn set_mem_word(&mut self, loc : usize, val : u32) {
        for (ind, byte) in val.to_ne_bytes().iter().enumerate() {
            self.memory[ind+loc] = *byte;
        }
    }

    // EMULATION MAIN FUNCTIONS-------------------------------------------------
    /* Get the file at 'filename' and load its contents into memory 
    filename <- relative path from executable to file */
    fn load_program(&mut self, filename: String) {
        match read(&filename) {
            Ok(bytes) => {
                if bytes.len() < MEMSIZE {
                    self.memory.splice(..bytes.len(), bytes);
                } else {
                    panic!("Binary file {} is too large for 16Kb memory", filename);
                }
            },
            Err(_) => panic!("Could not read file: {}", filename)
        }
    }

    // Run the main loop, fetching, decoding and executing instructions
    fn run_program(&mut self) {
        self.registers[PC] = 4;
        let mut current_instruction;

        loop {
            self.registers[PC] += 4;
            current_instruction = self.get_mem_word((self.registers[PC as usize] - 8) as usize);

            if current_instruction == 0 {break;}

            if self.check_condition(&current_instruction) {
                if get_bits(&current_instruction, 24, 4) == 0b1010 {
                    self.branch_instruction(&current_instruction);
                } else if get_bits(&current_instruction, 26, 2) != 0 && get_bits(&current_instruction, 21, 2) == 0 {
                    self.single_data_transfer_instruction(&current_instruction);
                } else if get_bits(&current_instruction, 22, 6) == 0 && get_bits(&current_instruction, 4, 4) == 0b1001 {
                    self.multiple_instruction(&current_instruction);
                } else if get_bits(&current_instruction, 26, 2) == 0 {
                    self.process_data_instruction(&current_instruction);
                } else {
                    self.fatal("Invalid instruction type", &current_instruction);
                }
            }
        }
    }

    // print the register and non-zero memory to the terminal
    fn print_state(&self) {
        println!("Registers:");
        for (ind, regval) in self.registers[..13].iter().enumerate() {
            println!("${reg:<3}: {val:>10} ({val:#010x})", reg=ind, val=*regval as i32);
        }
        println!("PC  : {val:>10} ({val:#010x})", val=self.registers[PC as usize] as i32);
        println!("CPSR: {val:>10} ({val:#010x})", val=(if self.cpsr.n {0x8000} else {0} + if self.cpsr.z {0x4000} else {0} + if self.cpsr.c {0x2000} else {0} + if self.cpsr.v {0x1000} else {0}) << 16);
        println!("Non-zero memory:");
        for loc in (0..MEMSIZE).step_by(4) {
            match (loc, self.get_mem_word(loc)) {
                (_,0) => (),
                (loc, val) => println!("{loc:#010x}: {val:#010x}", loc=loc, val=val.to_be())
            }
        }
    }

    /* INSTRUCTION PROCESSING-------------------------------------------------*/
    /* execute a branch instruction, updating the PC */
    fn branch_instruction(&mut self, instruction: &u32) {
        self.registers[PC] = (self.registers[PC] as i32 + (get_bits(instruction, 0, 23) as i32 - if get_bit(instruction, 23) {0x800000} else {0} + 1 << 2)) as u32
    }

    /* use condition bits of an instruction and the current cpsr to determine if an instruction should be executed */
    fn check_condition(&self, instruction: &u32) -> bool {
        match get_bits(instruction, 28, 4) {
            EQ => self.cpsr.z,
            NE => !self.cpsr.z,
            GE => self.cpsr.n == self.cpsr.v,
            LT => self.cpsr.n != self.cpsr.v,
            GT => !self.cpsr.z && (self.cpsr.n == self.cpsr.v),
            LE => self.cpsr.z || (self.cpsr.n != self.cpsr.v),
            AL => true,
            _ => false
        }
    }

    fn shift_operation(&mut self, instruction : &u32) -> (u32, bool) {
        let rm = get_bits(instruction, 0, 4) as usize;
        if rm == PC {self.fatal("invalid shift uses PC as Rm", instruction)}

        let rm_value = self.registers[rm];
        let shift_amount = 
            if !get_bit(instruction, 4) {
                /* <int>__0 case -> shift by immediate value */
                get_bits(instruction, 7, 5)
            } else if !get_bit(instruction, 7) {
                /* <RS>0__1 case -> shift specified by register */
                self.registers[get_bits(instruction, 8, 4) as usize]
            } else {
                self.fatal("Shift neither by constant, nor by register", instruction);
                panic!();
            };
        
        /* determine shift type and overflow/carryout using bits 5 & 6 of the instruction */
        if shift_amount == 0 {
            (rm_value, false)
        } else {
            match (get_bit(instruction, 6), get_bit(instruction, 5)) {
                /* logical left shift (lsl) */ (false, false) => (
                    rm_value << shift_amount, 
                    get_bit(&rm_value, 32 - shift_amount)
                ),
                /* logical right shift (lsr) */ (false, true) => (
                    rm_value >> shift_amount, 
                    get_bit(&rm_value, shift_amount - 1)
                ),
                /* arithmetic right shift (asr) */ (true, false) => (
                    (rm_value >> shift_amount) | if get_bit(&rm_value, 31) {u32::MAX << (32 - shift_amount)} else {0},
                    get_bit(&rm_value, shift_amount - 1)
                ),
                /* rotate right shift (ror) */ _ => (
                    (rm_value >> shift_amount) | (get_bits(&rm_value, 0, shift_amount) << (32 - shift_amount as i32)),
                    get_bit(&rm_value, shift_amount - 1)
                )
            }
        }
    }

    fn single_data_transfer_instruction(&mut self, instruction: &u32) {
        let rn_reg = get_bits(instruction, 16, 4) as usize;
        let rd_reg = get_bits(instruction, 12, 4) as usize;

        let i = get_bit(instruction, 25);
        let p = get_bit(instruction, 24);
        let u = get_bit(instruction, 23);
        let l = get_bit(instruction, 20);
    
        if PC == rd_reg {self.fatal( "Data Transfer instruction uses PC as Rd", instruction)};

        let offset = if i {
            if get_bits(instruction, 0, 4) as usize == rd_reg && !p {self.fatal("Data Transfer instruction uses same register as Rn, Rm", instruction)};
            self.shift_operation(instruction).0
        } else {get_bits(instruction, 0, 12)} as i32 * if !u {-1} else {1};

        let memloc = if p {
            (self.registers[rn_reg] as i32 + offset) as u32
        } else {
            let res = self.registers[rn_reg];
            self.registers[rn_reg] = (res as i32 + offset) as u32;
            res
        } as usize;

        if memloc == 0x20200008 || memloc == 0x20200004 || memloc == 0x20200000 {
            let region = ((memloc & 0xF) >> 2) * 10;
            println!("One GPIO pin from {} to {} has been accessed", region, region + 9);
            if l {self.registers[rd_reg] = memloc as u32}
        } else if memloc == 0x20200028 && !l {println!("PIN OFF")} 
        else if memloc == 0x2020001C && !l {println!("PIN ON")}
        else if memloc < MEMSIZE - 4 {
            if l {self.registers[rd_reg] = self.get_mem_word(memloc)}
            else {self.set_mem_word(memloc, self.registers[rd_reg])}
        } else {println!("Error: Out of bounds memory access at address {:#010x}", memloc)}
    }

    fn multiple_instruction(&mut self, instruction : &u32) {
        let rd_reg = get_bits(instruction, 16, 4) as usize;
        let rm_reg = get_bits(instruction, 0, 4) as usize;
        let rs_reg = get_bits(instruction, 8, 4) as usize;
        let rn_reg = get_bits(instruction, 12, 4) as usize;

        if rd_reg == rm_reg || rd_reg == PC || rm_reg == PC || rs_reg == PC ||  rn_reg == PC {self.fatal("Multiply instruction uses same register for Rd, Rm", instruction)};

        let a = get_bit(instruction, 21);
        let s = get_bit(instruction, 20);

        let result = self.registers[rm_reg] * self.registers[rs_reg] + if a {self.registers[rn_reg]} else {0};
        self.registers[rd_reg] = result;

        if s {  
            self.cpsr.n = get_bit(&result, 31);
            self.cpsr.z = result == 0;
        }
    }

    fn process_data_instruction(&mut self, instruction : &u32) {
        let opcode = get_bits(instruction, 21, 4);
        let rd_reg = get_bits(instruction, 12, 4) as usize;
        let rn_val = self.registers[get_bits(instruction, 16, 4) as usize];

        let i = get_bit(instruction, 25);
        let s = get_bit(instruction, 20);

        let (operand_2_value, carryout) = if i {
            let rotate = get_bits(instruction, 8, 4) << 1;
            let immediate = get_bits(instruction, 0, 8);
            (immediate.rotate_right(rotate), if rotate > 0 {get_bit(&immediate, rotate - 1)} else {false})
        } else {self.shift_operation(instruction)};

        let result = match opcode {
            TST | AND => rn_val & operand_2_value,
            TEQ | EOR => rn_val ^ operand_2_value,
            CMP | SUB => (rn_val as i32 - operand_2_value as i32) as u32,
            RSB => (operand_2_value as i32 - rn_val as i32) as u32,
            ADD => rn_val + operand_2_value,
            ORR => rn_val | operand_2_value,
            MOV => operand_2_value,
            _ => {self.fatal("Invalid operation in instruction", instruction); panic!()}
        };

        if opcode != CMP && opcode != TEQ && opcode != TST {self.registers[rd_reg] = result;}

        if s {
            self.cpsr.c = match opcode {
                AND | EOR | ORR | TEQ | TST | MOV => carryout,
                ADD | RSB => (get_bit(&rn_val, 31) || get_bit(&operand_2_value, 31)) && !get_bit(&result, 31),
                _ => operand_2_value <= rn_val
            };

            self.cpsr.z = result == 0;
            self.cpsr.n = get_bit(&result, 31);
        }
    }
}

fn main() {
    let args : Vec<String> = env::args().collect();

    if args.len() == 2 {
        let mut cpu = CPU::new();
        cpu.load_program(args[1].clone());
        cpu.run_program();
        cpu.print_state();
    } else {
        println!("Error: Invalid arguments");
    }
}