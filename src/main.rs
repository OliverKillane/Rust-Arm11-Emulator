use std::convert::TryInto;
use std::fs::read;

// my attempt to make the imperial college first year C Project emulator, but in rust!

fn main() {
    println!("Hello, world!");
}

// PANIC FORMATTING-------------------------------------------------------------
fn fatal_check(CPU: &CPU, cond: bool, error: String) {
    if cond {
        println!("{}", error);
        CPU.print_state();
        panic!();
    }
}

// CONSTANTS--------------------------------------------------------------------
const MEMSIZE : usize = 0x8000;

// MACHINE STATE STRUCTS--------------------------------------------------------
struct cpsr {
    N : bool,
    Z : bool,
    C : bool,
    V : bool
}

struct CPU {
    registers : [u32; 16],
    CPSR : cpsr,
    memory : Vec<u8>
}

// UTILITY FUNCTIONS------------------------------------------------------------

/* Return a range of bits:
data    <-  Source string of bits
start   <-  inclusive start
n       <-  number of bits
*/
fn get_bits(data : &u32, start : i32, n : i32) -> u32 {
    (data >> start) & ((1 << n) - 1)
}

/* get bit at Location in a Word:
data    <-  the Word you are inspecting
n       <-  bit number (0-31)
*/
fn get_bit(data : &u32, n : i32) -> bool {
    (*data >> n) & 1 != 0
}

/* Check the endian-ness of the system the emulator is being run on
return  <-  True (little endian), False (big endian)
*/
fn endian_check() -> bool {
    1u32.to_ne_bytes()[0] == 1
}

// CPU ACCESS-------------------------------------------------------------------
impl CPU {

    /* Create a new CPU struct:
    return  <-  New CPU with registers, memory initialised
    */
    pub fn new() -> CPU {
        CPU {
            registers : [0; 16],
            CPSR : cpsr {
                N : false,
                Z : false,
                C : false,
                V : false
            },
            memory : vec![0; MEMSIZE]
        }
    }
    
    /* Get the value stored in a register  
    reg     <-  register number
    */
    pub fn get_register(&self, reg : usize) -> u32 {
        self.registers[reg]
    }
    
    /* Set the value of a given register
    reg     <-  register number
    val     <-  value to store
    */
    pub fn set_register(&mut self, reg: usize, val : u32) {
        self.registers[reg] = val;
    }

    /* Get the word at a given memory location
    loc     <-  location of the start of the 4 bytes in memory
    */
    pub fn get_mem_word(&self, loc : usize) -> u32 {

        // yuck disgusting way, must improve!

        // given memory address is checked, will always return a value
        u32::from_ne_bytes(self.memory[loc..loc+4].try_into().unwrap())
    }

    /* Get the word at a given memory location
    loc     <-  location of the start of the 4 bytes in memory
    val     <-  the value to be written
    */
    pub fn set_mem_word(&mut self, loc : usize, val : u32) {

        // yuck disgusting way, must improve!
        for (ind, byte) in val.to_ne_bytes().iter().enumerate() {
            self.memory[ind+loc] = *byte;
        }
    }

    pub fn load_program(&mut self, filename: String) {
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

    pub fn print_state(&self) {
        println!("Registers:");
        for (ind, regval) in self.registers[..13].iter().enumerate() {
            print!("{reg:>3}: {val:010} ({val:#010x})", reg=ind, val=*regval);
        }
        print!("{reg:>3}: {val:010} ({val:#010x})", reg="PC", val=self.registers[15]);
        print!("CPSR: {val:010} ({val:#010x})", val=if self.CPSR.N {0x8000} else {0} + if self.CPSR.Z {0x4000} else {0} + if self.CPSR.C {0x2000} else {0} + if self.CPSR.V {0x1000} else {0});
        for loc in (0..MEMSIZE).step_by(4) {
            match (loc, self.get_mem_word(loc)) {
                (_,0) => (),
                (loc, val) => println!("{loc:#010x}: {val:#010x}", loc=loc, val=val)
            }
        }
    }
}

// INSTRUCTION PROCESSING FUNCTIONS --------------------------------------------

fn check_condition(instruction: &u32, cpsr : &cpsr) -> bool {
    match instruction {
        /*EQ*/ 0 => cpsr.Z,
        /*NE*/ 1 => !cpsr.Z,
        /*GE*/ 10 => cpsr.N == cpsr.V,
        /*LT*/ 11 => cpsr.N != cpsr.V,
        /*GT*/ 12 => !cpsr.Z && (cpsr.N == cpsr.V),
        /*LE*/ 13 => cpsr.Z || (cpsr.N != cpsr.V),
        /*AL*/ 14 => true,
        _ => false
    }
}

fn branch_instruction(instruction: &u32, CPU : &mut CPU) {

    // move the PC by a signed offset from bits 0-24, with -4 bytes 
    // (for offset pipeline emulation to work)

    CPU.set_register(14, (get_bits(instruction, 0, 23) - if get_bit(instruction, 23) {0x800001} else {1}) << 2)
}

fn shift_operation(CPU : &CPU, instruction : &u32) -> (u32, bool) {
    //implement
}

fn single_data_transfer_instruction(instruction: &u32, CPU : &mut CPU) {
    let RnBase = CPU.get_register(get_bits(instruction, 16, 4) as usize);
    let RdSrcDest = CPU.get_register(get_bits(instruction, 12, 4) as usize);

    let I = get_bit(instruction, 25);
    let P = get_bit(instruction, 24);
    let U = get_bit(instruction, 23);
    let L = get_bit(instruction, 20);

    fatal_check(CPU, CPU.get_register(14) == RdSrcDest, format!("Error: Data Transfer instruction uses PC as Rd: {:#010x}", instruction));

    let offset = if I {
        fatal_check(CPU, CPU.get_register(get_bits(instruction, 0, 4) as usize) == RdSrcDest && !P, format!("Error: Data Transfer instruction uses same register as Rn, Rm: {:#010x}", instruction));
        shift_operation(CPU, instruction).0
    } else { get_bits(instruction, 0, 12) } as i32 * if U {1} else {-1};
}


