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
const PC : u32 = 15;

/* memory size (bytes) */
const MEMSIZE : usize = 0x8000;

// UTILITY FUNCTIONS============================================================
/* Return a range of bits:
data    <-  Source string of bits
start   <-  inclusive start
n       <-  number of bits */
fn get_bits(data : &u32, start : u32, n : u32) -> u32 {
    (data >> start) & ((1 << n) - 1)
}

/* get bit at Location in a Word:
data    <-  the Word you are inspecting
n       <-  bit number (0-31) */
fn get_bit(data : &u32, n : u32) -> bool {
    (*data >> n) & 1 != 0
}

/* Check the endian-ness of the system the emulator is being run on
return  <-  True (little endian), False (big endian) */
fn endian_check() -> bool {
    1u32.to_ne_bytes()[0] == 1
}

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
    fn fatal(&self, error : String) {
        println!("{}", error);
        self.print_state();
        panic!();
    }
    
    /* Get the value stored in a register  
    reg     <-  register number (0-15) */
    fn get_register(&self, reg : u32) -> u32 {self.registers[reg as usize]}
    
    /* Set the value of a given register
    reg     <-  register number (0-15)
    val     <-  value to store */
    fn set_register(&mut self, reg: u32, val : u32) {self.registers[reg as usize] = val;}

    /* Get the word at a given memory location
    loc     <-  location of the start of the 4 bytes in memory */
    fn get_mem_word(&self, loc : usize) -> u32 {
        // yuck disgusting way, must improve!
        // given memory address is checked, will always return a value
        u32::from_ne_bytes(self.memory[loc..loc+4].try_into().unwrap())
    }

    /* Get the word at a given memory location
    loc     <-  location of the start of the 4 bytes in memory
    val     <-  the value to be written */
    fn set_mem_word(&mut self, loc : usize, val : u32) {
        // yuck disgusting way, must improve!
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
        self.registers[PC as usize] = 4;
        let mut current_instruction;

        loop {
            self.registers[PC as usize] += 4;
            current_instruction = self.get_mem_word((self.registers[PC as usize] - 8) as usize);

            if current_instruction == 0 {break;}

            if self.check_condition(&current_instruction) {
                if get_bits(&current_instruction, 24, 4) == 0b1100 {
                    self.branch_instruction(&current_instruction);
                } else if get_bits(&current_instruction, 26, 2) != 0 && get_bits(&current_instruction, 21, 2) == 0 {
                    self.single_data_transfer_instruction(&current_instruction);
                } else if get_bits(&current_instruction, 22, 6) == 0 && get_bits(&current_instruction, 4, 4) == 0b1001 {
                    self.multiple_instruction(&current_instruction);
                } else if get_bits(&current_instruction, 26, 2) == 0 {
                    self.process_data_instruction(&current_instruction);
                } else {
                    self.fatal(format!("Error: Invalid instruction type: {:#010x}", current_instruction));
                }
            }
        }
    }

    // print the register and non-zero memory to the terminal
    fn print_state(&self) {
        println!("Registers:");
        for (ind, regval) in self.registers[..13].iter().enumerate() {
            print!("{reg:>3}: {val:010} ({val:#010x})", reg=ind, val=*regval);
        }
        print!("{reg:>3}: {val:010} ({val:#010x})", reg="PC", val=self.registers[PC as usize]);
        print!("cpsr: {val:010} ({val:#010x})", val=if self.cpsr.n {0x8000} else {0} + if self.cpsr.z {0x4000} else {0} + if self.cpsr.c {0x2000} else {0} + if self.cpsr.v {0x1000} else {0});
        for loc in (0..MEMSIZE).step_by(4) {
            match (loc, self.get_mem_word(loc)) {
                (_,0) => (),
                (loc, val) => println!("{loc:#010x}: {val:#010x}", loc=loc, val=val)
            }
        }
    }

    /* INSTRUCTION PROCESSING-------------------------------------------------*/
    /* execute a branch instruction, updating the PC */
    fn branch_instruction(&mut self, instruction: &u32) {

        /* move the PC by a signed offset from bits 0-24, with -4 bytes 
        (for offset pipeline emulation to work) */
        self.set_register(14, (get_bits(instruction, 0, 23) - if get_bit(instruction, 23) {0x800001} else {1}) << 2)
    }

    /* use condition bits of an instruction and the current cpsr to determine if an instruction should be executed */
    fn check_condition(&self, instruction: &u32) -> bool {
        match *instruction {
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
        let rm = get_bits(instruction, 0, 4);
        if rm == PC {self.fatal(format!("Error: invalid shift uses PC as Rm: {:#010x}", instruction))}

        let rm_value = self.get_register(rm);
        let shift_amount = 
            if !get_bit(instruction, 4) {
                /* <int>__0 case -> shift by immediate value */
                get_bits(instruction, 7, 5)
            } else if !get_bit(instruction, 7) {
                /* <RS>0__1 case -> shift specified by register */
                self.get_register(get_bits(instruction, 8, 4))
            } else {
                self.fatal(format!("Error: Shift neither by constant, nor by register: {:#010x}", instruction));
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
        let rn_reg = get_bits(instruction, 16, 4);
        let rd_reg = get_bits(instruction, 12, 4);

        let i = get_bit(instruction, 25);
        let p = get_bit(instruction, 24);
        let u = get_bit(instruction, 23);
        let l = get_bit(instruction, 20);
    
        if PC == rd_reg {self.fatal( format!("Error: Data Transfer instruction uses PC as Rd: {:#010x}", instruction))};

        let offset = if i {
            if self.get_register(get_bits(instruction, 0, 4)) == rd_reg && !p {self.fatal(format!("Error: Data Transfer instruction uses same register as Rn, Rm: {:#010x}", instruction))};
            self.shift_operation(instruction).0
        } else {get_bits(instruction, 0, 12)} as i32 * if !u {-1} else {1};

        let memloc = if p {
            (self.get_register(rn_reg) as i32 + offset) as u32
        } else {
            let res = self.get_register(rn_reg);
            self.set_register(rn_reg, (res as i32 + offset) as u32);
            res
        } as usize;

        if memloc == 0x20200008 || memloc == 0x20200004 || memloc == 0x20200000 {
            let region = ((memloc & 0xF) >> 2) * 10;
            println!("One GPIO pin from {} to {} has been accessed", region, region + 9);
            if l {self.set_register(rd_reg, memloc as u32)}
        } else if memloc == 0x20200028 && !l {println!("PIN OFF")} 
        else if memloc == 0x2020001C && !l {println!("PIN ON")}
        else if memloc < MEMSIZE - 4 {
            if l {self.set_register(rd_reg, self.get_mem_word(memloc))}
            else {self.set_mem_word(memloc, self.get_register(rd_reg))}
        } else {println!("Error: Out of bounds memory access at address {:#010x}", memloc)}
    }

    fn multiple_instruction(&mut self, instruction : &u32) {
        let rd_reg = get_bits(instruction, 16, 4);
        let rm_reg = get_bits(instruction, 0, 4);
        let rs_reg = get_bits(instruction, 8, 4);
        let rn_reg = get_bits(instruction, 12, 4);

        if rd_reg == rm_reg || rd_reg == PC || rm_reg == PC || rs_reg == PC ||  rn_reg == PC {self.fatal(format!("Error: Multiply instruction uses same register for Rd, Rm: {:#010x}", instruction))};

        let a = get_bit(instruction, 21);
        let s = get_bit(instruction, 20);

        let result = self.get_register(rm_reg) * self.get_register(rs_reg) + if a {self.get_register(rn_reg)} else {0};

        self.set_register(rd_reg, result);

        if s {  
            self.cpsr.n = get_bit(&result, 31);
            self.cpsr.z = result == 0;
        }
    }

    fn process_data_instruction(&mut self, instruction : &u32) {
        let opcode = get_bits(instruction, 21, 4);
        let rd_reg = get_bits(instruction, 12, 4);
        let rn_val = self.get_register(get_bits(instruction, 16, 4));

        let i = get_bit(instruction, 25);
        let s = get_bit(instruction, 20);

        let (operand_2_value, carryout) = if i {
            let rotate = get_bits(instruction, 8, 4) << 1;
            let immediate = get_bits(instruction, 0, 8);
            ((immediate >> rotate) | (get_bits(&immediate, 0, rotate) << (32 - rotate)), if rotate > 0 {get_bit(&immediate, rotate - 1)} else {false})
        } else {self.shift_operation(instruction)};

        let result = match opcode {
            TST | AND => rn_val & operand_2_value,
            TEQ | EOR => rn_val ^ operand_2_value,
            CMP | SUB => rn_val - operand_2_value,
            RSB => operand_2_value - rn_val,
            ADD => rn_val + operand_2_value,
            ORR => rn_val | operand_2_value,
            MOV => operand_2_value,
            _ => {self.fatal(format!("Error: Invalid operation in instruction: {:#010x}", instruction)); panic!()}
        };

        if opcode != CMP && opcode != TEQ && opcode != TST {self.set_register(rd_reg, result);}

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
        println!("Error: Invalid arguments {:?}", args);
    }
}