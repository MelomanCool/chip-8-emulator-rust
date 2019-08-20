use std::fs;
use std::convert::TryInto;

struct Chip8 {
    memory: Vec<u8>,
    program_counter: u16,

    v: Vec<u8>,
    reg_i: u16,

    stack: Vec<u16>,

    delay_timer: u8,
    sound_timer: u8,

    keyboard: Vec<bool>,
    display_memory: Vec<bool>,

    rng: bool,
}

fn init() -> Chip8 {
    Chip8 {
        memory : vec![0; 4096],
        v : vec![0; 16],
        reg_i : 0,
        program_counter : 512,
        stack : Vec::new(),
        delay_timer : 0,
        sound_timer : 0,
        keyboard : vec![false; 16],
        display_memory : vec![false; 64 * 32],
        rng : false,
    }
}

#[derive(Debug)]
enum MetaOpcode {
    FlowControl(FlowControlOpcode),
    Regular(RegularOpcode),
    Unknown(u16),
}
use MetaOpcode::*;

#[derive(Debug)]
enum FlowControlOpcode {
    Jump { addr: u16 },
    JumpPlusV0 { addr: u16 },
    Call { addr: u16 },
    Return,
}
use FlowControlOpcode::*;

#[derive(Debug)]
enum RegularOpcode {
    SysCall,
    SkipIfRegValEqual { x: u8, value: u8 },
    SkipIfRegValNotEqual { x: u8, value: u8 },
    SkipIfRegRegEqual { x: u8, y: u8 },
    SkipIfRegRegNotEqual { x: u8, y: u8 },
    SkipIfKeyPressed { x: u8 },
    SkipIfKeyNotPressed { x: u8 },

    LoadValToReg { x: u8, value: u8 }, 
    LoadRegToReg { x: u8, y: u8 }, 
    LoadDelayTimerToReg { x: u8 },
    LoadKeyToReg { x: u8 },
    LoadRegToDelayTimer { x: u8 },
    LoadRegToSoundTimer { x: u8 },
    LoadValToI { value: u16 },
    LoadSpriteLocationToI { x: u8 }, 
    LoadRegBcdToMem { x: u8 },
    LoadRegsToMem { n: u8 },
    LoadMemToRegs { n: u8 },
    LoadRandomAndValToReg { x: u8, value: u8 },

    SubRegFromReg { x: u8, y: u8 }, 
    SubnRegFromReg { x: u8, y: u8 }, 
    AddValToReg { x: u8, value: u8 },
    AddRegToI { x: u8 },
    OrRegReg { x: u8, y: u8 },
    AndRegReg { x: u8, y: u8 },
    XorRegReg { x: u8, y: u8 },
    AddRegToReg { x: u8, y: u8 },
    ShiftRightReg { x: u8 },
    ShiftLeftReg { x: u8 }, 

    ClearScreen,
    DrawSprite { x: u8, y: u8, n: u8 },
}
use RegularOpcode::*;

fn parse_opcode(opcode: u16) -> MetaOpcode {
    let a: u8 = (0x000F & (opcode >> 12)).try_into().unwrap();
    let b: u8 = (0x000F & (opcode >>  8)).try_into().unwrap();
    let c: u8 = (0x000F & (opcode >>  4)).try_into().unwrap();
    let d: u8 = (0x000F & (opcode >>  0)).try_into().unwrap();

    let nnn = 0x0FFF & opcode;
    let  kk: u8 = (0x00FF & opcode).try_into().unwrap();

    return match (a, b, c, d) {
        (  0,   0, 0xE,   0) => Regular(ClearScreen),
        (  0,   0, 0xE, 0xE) => FlowControl(Return),
        (  0,   _,   _,   _) => Regular(SysCall),
        (  1,   _,   _,   _) => FlowControl(Jump { addr : nnn }),
        (  2,   _,   _,   _) => FlowControl(Call { addr : nnn }),
        (  3,   x,   _,   _) => Regular(SkipIfRegValEqual { x, value : kk }),
        (  4,   x,   _,   _) => Regular(SkipIfRegValNotEqual { x, value : kk }),
        (  5,   x,   y,   0) => Regular(SkipIfRegRegEqual { x, y }),
        (  6,   x,   _,   _) => Regular(LoadValToReg { x, value : kk }),
        (  7,   x,   _,   _) => Regular(AddValToReg { x, value : kk }),
        (  8,   x,   y,   0) => Regular(LoadRegToReg { x, y }),
        (  8,   x,   y,   1) => Regular(OrRegReg { x, y }),
        (  8,   x,   y,   2) => Regular(AndRegReg { x, y }),
        (  8,   x,   y,   3) => Regular(XorRegReg { x, y }),
        (  8,   x,   y,   4) => Regular(AddRegToReg { x, y }),
        (  8,   x,   y,   5) => Regular(SubRegFromReg { x, y }),
        (  8,   x,   _,   6) => Regular(ShiftLeftReg { x }),
        (  8,   x,   y,   7) => Regular(SubnRegFromReg { x, y }),
        (  8,   x,   _, 0xE) => Regular(ShiftRightReg { x }),
        (  9,   x,   y,   0) => Regular(SkipIfRegRegNotEqual { x, y }),
        (0xA,   _,   _,   _) => Regular(LoadValToI { value : nnn }),
        (0xB,   _,   _,   _) => FlowControl(JumpPlusV0 { addr : nnn }),
        (0xC,   x,   _,   _) => Regular(LoadRandomAndValToReg { x, value : kk }),
        (0xD,   x,   y,   n) => Regular(DrawSprite { x, y, n }),
        (0xE,   x,   9, 0xE) => Regular(SkipIfKeyPressed { x }),
        (0xE,   x, 0xA,   1) => Regular(SkipIfKeyNotPressed { x }),
        (0xF,   x,   0,   7) => Regular(LoadDelayTimerToReg { x }),
        (0xF,   x,   0, 0xA) => Regular(LoadKeyToReg { x }),
        (0xF,   x,   1,   5) => Regular(LoadRegToDelayTimer { x }),
        (0xF,   x,   1,   8) => Regular(LoadRegToSoundTimer { x }),
        (0xF,   x,   1, 0xE) => Regular(AddRegToI { x }),
        (0xF,   x,   2,   9) => Regular(LoadSpriteLocationToI { x }),
        (0xF,   x,   3,   3) => Regular(LoadRegBcdToMem { x }),
        (0xF,   n,   5,   5) => Regular(LoadRegsToMem { n }),
        (0xF,   n,   6,   5) => Regular(LoadMemToRegs { n }),
        (  _,   _,   _,   _) => Unknown(opcode)
    }
}

fn load_rom(chip8: Chip8, filename: &str) -> Chip8 {
    let rom = fs::read(filename).expect("Couldn't load the rom.");
    let mut memory = chip8.memory.to_vec();
    memory.splice(512.., rom);
    return Chip8 { memory , .. chip8 };
}

fn push<T: Clone>(vec: Vec<T>, x: T) -> Vec<T> {
    let mut v = vec.to_vec();
    v.push(x);
    return v;
}

fn pop<T: Clone>(vec: Vec<T>) -> (Vec<T>, T) {
    let mut v = vec.to_vec();
    let x = v.pop().expect("Can't pop the empty stack.");
    return (v, x);
}

fn replace<T: Clone>(vec: &Vec<T>, i: u8, x: T) -> Vec<T> {
    let mut v = vec.to_vec();
    v[i as usize] = x;
    return v;
}

fn byte_to_bits(b: &u8) -> Vec<bool> {
    return vec![
        1u8 == (1u8 & (b >> 7)),
        1u8 == (1u8 & (b >> 6)),
        1u8 == (1u8 & (b >> 5)),
        1u8 == (1u8 & (b >> 4)),
        1u8 == (1u8 & (b >> 3)),
        1u8 == (1u8 & (b >> 2)),
        1u8 == (1u8 & (b >> 1)),
        1u8 == (1u8 & (b >> 0)),
    ];
}

fn step(chip8: Chip8) -> Chip8 {
    let raw_opcode =
        ((chip8.memory[chip8.program_counter as usize] as u16) << 8)
        | chip8.memory[(chip8.program_counter + 1) as usize] as u16;
    let meta_opcode = parse_opcode(raw_opcode);

    print!("{:04X} ", raw_opcode);

    match meta_opcode {
        FlowControl(ref opcode) => println!("{:X?}", opcode),
        Regular(ref opcode) => println!("{:X?}", opcode),
        ref unknown => println!("{:X?}", unknown)
    };

    return match meta_opcode {
        FlowControl(opcode) => match opcode {
            Jump { addr } =>
                Chip8 { program_counter : addr, .. chip8 },
            JumpPlusV0 { addr } =>
                Chip8 { program_counter : addr + chip8.v[0] as u16, .. chip8 },
            Call { addr } =>
                Chip8 { program_counter : addr, stack : push(chip8.stack, chip8.program_counter), .. chip8 },
            Return => {
                let (new_stack, pc) = pop(chip8.stack);
                Chip8 { program_counter : pc, stack : new_stack, .. chip8 }
            }
        }
        Regular(opcode) => {
            let res = match opcode {
                SysCall =>
                    chip8,
                LoadValToReg { x, value } =>
                    Chip8 { v : replace(&chip8.v, x, value), .. chip8 },
                LoadValToI { value } =>
                    Chip8 { reg_i : value, .. chip8 },
                LoadRandomAndValToReg { x, value } =>
                    Chip8 { v : replace(&chip8.v, x, (chip8.rng as u8) & value), rng : !chip8.rng, .. chip8 },
                AddValToReg { x, value } =>
                    Chip8 { v : replace(&chip8.v, x, chip8.v[x as usize] + value), .. chip8 },
                SkipIfRegValEqual { x, value } =>
                    if chip8.v[x as usize] == value {
                        Chip8 { program_counter : chip8.program_counter + 2, .. chip8 }
                    } else {
                        chip8
                    },
                DrawSprite { x, y, n } => {
                    let lines = chip8.memory.get((chip8.reg_i as usize)..((chip8.reg_i + n as u16) as usize)).expect("Idk").into_iter().map(byte_to_bits).collect::<Vec<Vec<bool>>>();
                    let mut dm = chip8.display_memory.to_vec();
                    let start_x = chip8.v[x as usize] as usize;
                    let start_y = chip8.v[y as usize] as usize;
                    let mut vf = 0;
                    for (yy, l) in lines.into_iter().enumerate() {
                        for (xx, pix) in l.into_iter().enumerate() {
                            let pos = ((start_x + xx) % 64 + ((start_y + yy) % 32) * 64) as usize;
                            let xored = dm[pos] ^ pix;
                            if (dm[pos] == true) && (xored == false) {
                                vf = 1;
                            }
                            dm[pos] = xored;
                        }
                    }
                    display(&dm);
                    Chip8 { display_memory : dm, v : replace(&chip8.v, 0xF, vf), .. chip8 }
                },
                _ =>
                    chip8
            };
            Chip8 { program_counter : res.program_counter + 2, .. res }
        }
        Unknown(_) => chip8
    }
}

fn display(display_memory: &Vec<bool>) {
    for y in 0..32 {
        for x in 0..64 {
            print!("{}", if display_memory[x + 64 * y] {"#"} else {" "});
        }
        println!();
    }
}

fn main() {
    let mut chip8 = init();
    chip8 = load_rom(chip8, &"roms/maze.rom");
    for _ in 0..1200 {
        chip8 = step(chip8);
        println!("{:X?}", (chip8.reg_i, &chip8.v));
    }
}
