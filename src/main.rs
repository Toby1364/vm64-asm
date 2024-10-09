use colored::Colorize;
use indoc::indoc;
use std::{collections::HashMap, fs, path::PathBuf};
use image::ImageReader;

fn print_usage() {
    let uasge = indoc! {"
        Usage:
            -i      <input_folder>
            -o      <output_file>

            -cfg    <file_path>             File for more verbose build arguments.
            -inter  <output_file>           Generates intermediate represantation.
            -align  <alignment in hex>      Used for aligning labels in absolute mode.
    "};

    println!("{}", uasge);
}

fn resolve_args(args: Vec<String>) -> Vec<Option<String>> {
    let mut sorted_args = Vec::new();

    let mut input_path = None;
    let mut output_path = None;
    let mut cfg_path = None;
    let mut inter_path = None;
    let mut alignment = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_ref() {
            "-i" => {
                input_path = Some(args[i + 1].clone());
                i += 1;
            }
            "-o" => {
                output_path = Some(args[i + 1].clone());
                i += 1;
            }
            "-cfg" => {
                cfg_path = Some(args[i + 1].clone());
                i += 1;
            }
            "-inter" => {
                inter_path = Some(args[i + 1].clone());
                i += 1;
            }
            "-align" => {
                alignment = Some(args[i + 1].clone());
                i += 1;
            }

            _ => {}
        }
        i += 1;
    }

    sorted_args.push(input_path);
    sorted_args.push(output_path);
    sorted_args.push(cfg_path);
    sorted_args.push(inter_path);
    sorted_args.push(alignment);

    return sorted_args;
}

fn get_all_files(path: String) -> Vec<PathBuf> {
    let mut file_paths = Vec::new();

    for entry in fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() {
            file_paths.push(path);
        }
        else {
            file_paths.append(&mut get_all_files(path.to_str().unwrap().to_string()));
        }
    }

    return file_paths;
}

type Line = (Control, Vec<u8>, String, Vec<String>, String, usize);

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 || args[1] == "help" || args[1] == "-h" {
        print_usage();
        return;
    }

    let mut args = resolve_args(args);
    let mut input_path = args[0].clone();
    let mut output_path = args[1].clone();
    let     cfg_path = args[2].clone();
    let mut inter_path = args[3].clone();
    let mut alignment = args[4].clone();
    let mut align = 0;

    if let Some(cfg_path) = cfg_path {
        let cfg = fs::read_to_string(cfg_path).expect("Unable to read config file.")
            .replace("\n", " ")
            .replace("_", "")
            .replace("\r", "")
            .replace("  ", " ")
            .replace(", ", " ")
            .replace(",", " ");
        args = resolve_args(cfg.split(" ").map(|s| s.trim().to_owned()).collect());

        input_path = args[0].clone();
        output_path = args[1].clone();
        inter_path = args[3].clone();
        alignment = args[4].clone();
    }

    let input_path = input_path.expect("Input path must be specified.");
    let output_path = output_path.expect("Output path must be specified.");

    if let Some(alignment) = alignment {
        align = usize::from_str_radix(&alignment, 16).expect("Invalid hex literal for alignment.");
    }

    let paths = get_all_files(input_path.clone());

    let mut instructions = lex_files(paths);

    println!("Todo: Alignment, Abstractions, Images");

    let mut labels: HashMap<String, usize> = HashMap::new();
    let mut data_pointers: HashMap<String, usize> = HashMap::new();

    let mut index = align;
    let mut i = 0;
    while i < instructions.len() {
        match &instructions[i].0 {
            Control::Label => {
                labels.insert(instructions[i].2.clone().strip_suffix(":").unwrap().to_owned(), index);
            }
            Control::ImgDataPointer(path) => {
                let img = ImageReader::open(&format!("{}/{}", &input_path, path)).expect(&format!("Couldn't open {}", path)).decode().unwrap();

                let bytes: Vec<u8> = img.as_rgb8().unwrap().clone().into_raw().to_vec();

                instructions.push((Control::Data, bytes, instructions[i].2.clone(), instructions[i].3.clone(), instructions[i].4.clone(), instructions[i].5.clone()));
            }
            Control::Data => {
                data_pointers.insert(instructions[i].3[0].clone(), index);
            }

            _ => {}
        }
        index += instructions[i].1.len();
        i += 1;
    }

    for inst in instructions.iter_mut() {
        if inst.0 == Control::ReqLabel {
            if let Some(label) = labels.get(inst.3.last().unwrap().as_str()) {
                let l = inst.1.len();

                inst.1[l-4..].copy_from_slice(&label.to_be_bytes()[4..]);
            }
        }
        if inst.0 == Control::ReqDataPointer {
            if let Some(pointer) = data_pointers.get(inst.3[1].as_str()) {

                inst.1[1..5].copy_from_slice(&pointer.to_be_bytes()[4..]);
            }
        }
    }

    let mut bytes = Vec::new();

    for inst in instructions.iter() {
        bytes.append(&mut inst.1.clone());
    }

    if let Some(path) = inter_path {
        let mut buf = String::new();

        let mut index = align;

        for inst in instructions.iter() {
            match inst.0 {
                Control::DataPointer(_) => { continue }
                Control::ImgDataPointer(_) => { continue }
                _ => {}
            }

            let sl = buf.len();
            if inst.0 != Control::Label { buf.push_str(&format!("0x{:08x}:", index)) }

            while buf.len() < sl + 20 { buf.push(' ') }
            
            if inst.0 == Control::Data {
                for n in inst.1.clone()[..5].into_iter() {
                    buf.push_str(&format!("{:02x} ", n));
                }
                buf.push_str(". . . ");
                for n in inst.1.clone()[inst.1.len() - 5..].into_iter() {
                    buf.push_str(&format!("{:02x} ", n));
                }
            }
            else {
                for n in inst.1.clone().into_iter() {
                    buf.push_str(&format!("{:02x} ", n));
                }
            }

            while buf.len() < sl + 70 { buf.push(' ') }

            buf.push_str(&format!("{} ", inst.2));
            for arg in inst.3.clone().into_iter() {
                buf.push_str(&format!("{} ", arg));
            }

            while buf.len() < sl + 110 { buf.push(' ') }

            buf.push_str(&format!("{}:{}\n", inst.4, inst.5));

            index += inst.1.len();
        }

        fs::write(path, buf).unwrap();
    }

    fs::write(output_path, bytes).unwrap();
}

#[derive(Debug, Clone, PartialEq)]
enum Control {
    None,
    Inst,
    Label,
    ReqLabel,
    Data,
    DataPointer(String),
    ImgDataPointer(String),
    ReqDataPointer,
}

fn lex_files(paths: Vec<PathBuf>) -> Vec<Line> {
    let mut instructions: Vec<Line> = Vec::new();
    for path in paths {
        if path.extension().unwrap() != "asm" { continue }
        let mut code: Vec<char> = fs::read_to_string(&path).unwrap()
            .replace("\r", "")
            .replace("\\\n", " ")
            .replace(", ", " ")
            .replace(",", " ")
            .replace("  ", " ")
            .replace("  ", " ")
            .replace("  ", " ")
            .replace("  ", " ")
            .replace("0x", "&")
            .trim()
            .chars().collect();

        let mut comment = false;
        let mut i = 0;
        while i < code.len() {
            if code[i] == ';' { comment = true }
            if code[i] == '\n' { comment = false }
            if comment { code.remove(i); }
            if !comment { i += 1 }
        }

        let code = code.into_iter().collect::<String>();

        let lines = code.split("\n").map(|s| s.trim().to_owned()).collect::<Vec<String>>();

        for line in 0..lines.len() {
            if lines[line].is_empty() { continue }
            let parts = lines[line].split(" ").collect::<Vec<&str>>();

            instructions.push((
                Control::None,
                Vec::new(), 
                parts[0].to_owned(), 
                parts[1..].into_iter().map(|s| s.to_owned().to_owned()).collect(), 
                path.display().to_string(), 
                line + 1
            ));

            let lline = instructions.last().unwrap().to_owned();

            if lline.2.ends_with(":") {
                instructions.last_mut().unwrap().0 = Control::Label;
            }
            else if lline.2.starts_with("#") {
                let cmd = lline.2.strip_prefix("#").unwrap();

                match cmd.to_lowercase().as_str() {
                    "image" => { instructions.last_mut().unwrap().0 = Control::ImgDataPointer(parts[2].to_owned().to_owned()) }
                    "bytes" => { instructions.last_mut().unwrap().0 = Control::DataPointer(parts[2].to_owned().to_owned()) }

                    _ => { error(lline.clone(), "Unknown assembler command.") }
                }
            }
            else {
                let mut args: Vec<Arg> = Vec::new();

                for arg in parts[1..].into_iter() {
                    let arg = resolve_arg(arg.to_owned().to_owned());

                    match arg {
                        Ok(arg) => args.push(arg),
                        Err(e) => error(lline.clone(), e),
                    }
                }

                match resolve_inst(lline.2.clone(), args) {
                    Ok(res) => { instructions.last_mut().unwrap().0 = res.1; instructions.last_mut().unwrap().1 = res.0 },
                    Err(e) => error(lline.clone(), e),
                }
            }
        }
    }
    instructions
}

enum Arg {
    Freg(u8),
    Ureg(u8),
    Liter(Vec<u8>),
    Label([u8; 4]),
}

fn resolve_arg(mut arg: String) -> Result<Arg, &'static str> {
    arg = arg.replace("_", "");

    if arg.starts_with("r") && arg.len() < 4 {
        let n = u8::from_str_radix(&arg[1..], 16);
        
        match n {
            Ok(n) => Ok(Arg::Ureg(n)),
            Err(_) => Err("Invalid register index."),
        }
    }
    else if arg.starts_with("f") && arg.len() < 4 {
        let n = u8::from_str_radix(&arg[1..], 16);

        match n {
            Ok(n) => Ok(Arg::Freg(n)),
            Err(_) => Err("Invalid register index."),
        }
    }
    else if arg.starts_with("&") {
        let n = u64::from_str_radix(&arg[1..], 16);

        match n {
            Ok(n) => Ok(Arg::Liter(n.to_be_bytes().to_vec())),
            Err(_) => Err("Invalid hex literal."),
        }
    }
    else if arg.parse::<u64>().is_ok() {
        Ok(Arg::Liter(arg.parse::<u64>().unwrap().to_be_bytes().to_vec()))
    }
    else if arg.parse::<u128>().is_ok() {
        Err("Mate, that doesn't fit into a u64, what the hell are you trying to do?!")
    }
    else {
        Ok(Arg::Label([0; 4]))
    }
}

fn resolve_inst(inst: String, args: Vec<Arg>) -> Result<(Vec<u8>, Control), &'static str> {

    let ci = Control::Inst;
    let rl = Control::ReqLabel;
    let rd = Control::ReqDataPointer;


    match inst.to_lowercase().as_ref() {
        "nop" => { Ok((vec![0x00], ci)) }
        
        "mov" => {
            match args.len() {
                2 => {
                    let mut float = false;
                    let dest_reg = match &args[0] {
                        Arg::Ureg(n) =>        { n }
                        Arg::Freg(n) =>        { float = true; n }  
                        Arg::Liter(_) =>            { return Err("Invalid argument, expected floating point register or register, got literal.") }
                        Arg::Label(_) =>            { return Err("Invalid argument, expected floating point register or register, got label.") }
                    };
                    match &args[1] {
                        Arg::Ureg(n) =>        { if float {return Ok((vec![0x03, *dest_reg, *n], ci))} Ok((vec![0x01, *dest_reg, *n], ci)) }
                        Arg::Freg(n) =>        { if float {return Ok((vec![0x02, *dest_reg, *n], ci))} Ok((vec![0x04, *dest_reg, *n], ci)) }  
                        Arg::Liter(n) =>  { if float {let mut b = vec![0x06, *dest_reg]; b.extend_from_slice(&n); return Ok((b, ci))} let mut b = vec![0x05, *dest_reg]; b.extend_from_slice(&n); Ok((b, ci)) }
                        Arg::Label(_) =>            { Err("Invalid argument, expected floating point register, literal, or register, got label.") }
                    }
                }
                3 => {
                    match (&args[0], &args[1], &args[2]) {
                        (Arg::Ureg(dest), Arg::Liter(len), Arg::Liter(addr)) => { let mut b = vec![0x07, *dest]; b.extend_from_slice(&len[7..]); b.extend_from_slice(&addr[4..]); Ok((b, ci)) }
                        (Arg::Freg(dest), Arg::Liter(len), Arg::Liter(addr)) => { let mut b = vec![0x08, *dest]; b.extend_from_slice(&len[7..]); b.extend_from_slice(&addr[4..]); Ok((b, ci)) }
                        (Arg::Liter(addr), Arg::Ureg(src), Arg::Liter(len)) => { let mut b = vec![0x09]; b.extend_from_slice(&addr[4..]); b.extend_from_slice(&len[7..]); b.push(*src); Ok((b, ci)) }
                        (Arg::Liter(addr), Arg::Freg(src), Arg::Liter(len)) => { let mut b = vec![0x0A]; b.extend_from_slice(&addr[4..]); b.extend_from_slice(&len[7..]); b.push(*src); Ok((b, ci)) }

                        (Arg::Ureg(dest), Arg::Ureg(src), Arg::Liter(len)) => { let mut b = vec![0x0B, *dest]; b.extend_from_slice(&len[7..]); b.push(*src); Ok((b, ci)) }
                        (Arg::Ureg(dest), Arg::Freg(src), Arg::Liter(len)) => { let mut b = vec![0x0C, *dest]; b.extend_from_slice(&len[7..]); b.push(*src); Ok((b, ci)) }
                        (Arg::Ureg(dest), Arg::Liter(len), Arg::Ureg(src)) => { let mut b = vec![0x0D, *dest]; b.extend_from_slice(&len[7..]); b.push(*src); Ok((b, ci)) }
                        (Arg::Freg(dest), Arg::Liter(len), Arg::Ureg(src)) => { let mut b = vec![0x0E, *dest]; b.extend_from_slice(&len[7..]); b.push(*src); Ok((b, ci)) }
                    
                        _ => { Err("Invalid argument arrangement.") }
                    }
                }
                4 => {
                    match (&args[0], &args[1], &args[2], &args[3]) {
                        (Arg::Liter(off), Arg::Ureg(dest), Arg::Ureg(src), Arg::Liter(len)) => { let mut b = vec![0x17, *dest]; b.extend_from_slice(&len[7..]); b.push(*src); b.extend_from_slice(&off[4..]); Ok((b, ci)) }
                        (Arg::Liter(off), Arg::Ureg(dest), Arg::Freg(src), Arg::Liter(len)) => { let mut b = vec![0x18, *dest]; b.extend_from_slice(&len[7..]); b.push(*src); b.extend_from_slice(&off[4..]); Ok((b, ci)) }
                        (Arg::Ureg(dest), Arg::Liter(len), Arg::Liter(off), Arg::Ureg(src)) => { let mut b = vec![0x19, *dest]; b.extend_from_slice(&len[7..]); b.push(*src); b.extend_from_slice(&off[4..]); Ok((b, ci)) }
                        (Arg::Freg(dest), Arg::Liter(len), Arg::Liter(off), Arg::Ureg(src)) => { let mut b = vec![0x1A, *dest]; b.extend_from_slice(&len[7..]); b.push(*src); b.extend_from_slice(&off[4..]); Ok((b, ci)) }

                        _ => { Err("Invalid argument arrangement.") }
                    }
                }

                _ => { Err("Invalid number of arguments.") }
            }
        }
        "mva" => {
            match args.len() {
                3 => {
                    match (&args[0], &args[1], &args[2]) {
                        (Arg::Ureg(dest), Arg::Liter(len), Arg::Liter(addr)) => { let mut b = vec![0x0F, *dest]; b.extend_from_slice(&len[7..]); b.extend_from_slice(&addr[4..]); Ok((b, ci)) }
                        (Arg::Freg(dest), Arg::Liter(len), Arg::Liter(addr)) => { let mut b = vec![0x10, *dest]; b.extend_from_slice(&len[7..]); b.extend_from_slice(&addr[4..]); Ok((b, ci)) }
                        (Arg::Liter(addr), Arg::Ureg(src), Arg::Liter(len)) => { let mut b = vec![0x11]; b.extend_from_slice(&addr[4..]); b.extend_from_slice(&len[7..]); b.push(*src); Ok((b, ci)) }
                        (Arg::Liter(addr), Arg::Freg(src), Arg::Liter(len)) => { let mut b = vec![0x12]; b.extend_from_slice(&addr[4..]); b.extend_from_slice(&len[7..]); b.push(*src); Ok((b, ci)) }

                        (Arg::Ureg(dest), Arg::Ureg(src), Arg::Liter(len)) => { let mut b = vec![0x13, *dest]; b.extend_from_slice(&len[7..]); b.push(*src); Ok((b, ci)) }
                        (Arg::Ureg(dest), Arg::Freg(src), Arg::Liter(len)) => { let mut b = vec![0x14, *dest]; b.extend_from_slice(&len[7..]); b.push(*src); Ok((b, ci)) }
                        (Arg::Ureg(dest), Arg::Liter(len), Arg::Ureg(src)) => { let mut b = vec![0x15, *dest]; b.extend_from_slice(&len[7..]); b.push(*src); Ok((b, ci)) }
                        (Arg::Freg(dest), Arg::Liter(len), Arg::Ureg(src)) => { let mut b = vec![0x16, *dest]; b.extend_from_slice(&len[7..]); b.push(*src); Ok((b, ci)) }
                    
                        _ => { Err("Invalid argument arrangement.") }
                    }
                }
                4 => {
                    match (&args[0], &args[1], &args[2], &args[3]) {
                        (Arg::Liter(off), Arg::Ureg(dest), Arg::Ureg(src), Arg::Liter(len)) => { let mut b = vec![0x1B, *dest]; b.extend_from_slice(&len[7..]); b.push(*src); b.extend_from_slice(&off[4..]); Ok((b, ci)) }
                        (Arg::Liter(off), Arg::Ureg(dest), Arg::Freg(src), Arg::Liter(len)) => { let mut b = vec![0x1C, *dest]; b.extend_from_slice(&len[7..]); b.push(*src); b.extend_from_slice(&off[4..]); Ok((b, ci)) }
                        (Arg::Ureg(dest), Arg::Liter(len), Arg::Liter(off), Arg::Ureg(src)) => { let mut b = vec![0x1D, *dest]; b.extend_from_slice(&len[7..]); b.push(*src); b.extend_from_slice(&off[4..]); Ok((b, ci)) }
                        (Arg::Freg(dest), Arg::Liter(len), Arg::Liter(off), Arg::Ureg(src)) => { let mut b = vec![0x1E, *dest]; b.extend_from_slice(&len[7..]); b.push(*src); b.extend_from_slice(&off[4..]); Ok((b, ci)) }

                        _ => { Err("Invalid argument arrangement.") }
                    }
                }

                _ => { Err("Invalid number of arguments.") }
            }
        }
        "mvd" => {
            match args.len() {
                3 => {
                    match (&args[0], &args[1], &args[2]) {
                        (Arg::Ureg(dest), Arg::Liter(len), Arg::Liter(addr)) => { let mut b = vec![0x1F, *dest]; b.extend_from_slice(&len[7..]); b.extend_from_slice(&addr[4..]); Ok((b, ci)) }
                        (Arg::Freg(dest), Arg::Liter(len), Arg::Liter(addr)) => { let mut b = vec![0x20, *dest]; b.extend_from_slice(&len[7..]); b.extend_from_slice(&addr[4..]); Ok((b, ci)) }
                        (Arg::Liter(addr), Arg::Ureg(src), Arg::Liter(len)) => { let mut b = vec![0x21]; b.extend_from_slice(&addr[4..]); b.extend_from_slice(&len[7..]); b.push(*src); Ok((b, ci)) }
                        (Arg::Liter(addr), Arg::Freg(src), Arg::Liter(len)) => { let mut b = vec![0x22]; b.extend_from_slice(&addr[4..]); b.extend_from_slice(&len[7..]); b.push(*src); Ok((b, ci)) }

                        (Arg::Ureg(dest), Arg::Ureg(src), Arg::Liter(len)) => { let mut b = vec![0x23, *dest]; b.extend_from_slice(&len[7..]); b.push(*src); Ok((b, ci)) }
                        (Arg::Ureg(dest), Arg::Freg(src), Arg::Liter(len)) => { let mut b = vec![0x24, *dest]; b.extend_from_slice(&len[7..]); b.push(*src); Ok((b, ci)) }
                        (Arg::Ureg(dest), Arg::Liter(len), Arg::Ureg(src)) => { let mut b = vec![0x25, *dest]; b.extend_from_slice(&len[7..]); b.push(*src); Ok((b, ci)) }
                        (Arg::Freg(dest), Arg::Liter(len), Arg::Ureg(src)) => { let mut b = vec![0x26, *dest]; b.extend_from_slice(&len[7..]); b.push(*src); Ok((b, ci)) }
                    
                        _ => { Err("Invalid argument arrangement.") }
                    }
                }
                4 => {
                    match (&args[0], &args[1], &args[2], &args[3]) {
                        (Arg::Liter(off), Arg::Ureg(dest), Arg::Ureg(src), Arg::Liter(len)) => { let mut b = vec![0x27, *dest]; b.extend_from_slice(&len[7..]); b.push(*src); b.extend_from_slice(&off[4..]); Ok((b, ci)) }
                        (Arg::Liter(off), Arg::Ureg(dest), Arg::Freg(src), Arg::Liter(len)) => { let mut b = vec![0x28, *dest]; b.extend_from_slice(&len[7..]); b.push(*src); b.extend_from_slice(&off[4..]); Ok((b, ci)) }
                        (Arg::Ureg(dest), Arg::Liter(len), Arg::Liter(off), Arg::Ureg(src)) => { let mut b = vec![0x29, *dest]; b.extend_from_slice(&len[7..]); b.push(*src); b.extend_from_slice(&off[4..]); Ok((b, ci)) }
                        (Arg::Freg(dest), Arg::Liter(len), Arg::Liter(off), Arg::Ureg(src)) => { let mut b = vec![0x2A, *dest]; b.extend_from_slice(&len[7..]); b.push(*src); b.extend_from_slice(&off[4..]); Ok((b, ci)) }

                        _ => { Err("Invalid argument arrangement.") }
                    }
                }

                _ => { Err("Invalid number of arguments.") }
            }
        }

        "add" => {
            let mut float = false;
            let dest_reg = match &args[0] {
                Arg::Ureg(n) =>        { n }
                Arg::Freg(n) =>        { float = true; n }  
                Arg::Liter(_) =>            { return Err("Invalid argument, expected floating point register or register, got literal.") }
                Arg::Label(_) =>            { return Err("Invalid argument, expected floating point register or register, got label.") }
            };
            let reg = match &args[1] {
                Arg::Ureg(n) =>        { if float {return Err("Mismatched register types.")} n }
                Arg::Freg(n) =>        { if !float {return Err("Mismatched register types.")} n }  
                Arg::Liter(_) =>            { return Err("Invalid argument, expected floating point register or register, got literal.") }
                Arg::Label(_) =>            { return Err("Invalid argument, expected floating point register or register, got label.") }
            };
            match &args[2] {
                Arg::Ureg(n) =>        { if float {return Err("Mismatched register types.")} Ok((vec![0x30, *dest_reg, *reg, *n], ci)) }
                Arg::Freg(n) =>        { if !float {return Err("Mismatched register types.")} Ok((vec![0x31, *dest_reg, *reg, *n], ci)) }  
                Arg::Liter(_) =>            { Err("Invalid argument, expected floating point register or register, got literal.") }
                Arg::Label(_) =>            { Err("Invalid argument, expected floating point register or register, got label.") }
            }
        }
        "sub" => {
            let mut float = false;
            let dest_reg = match &args[0] {
                Arg::Ureg(n) =>        { n }
                Arg::Freg(n) =>        { float = true; n }  
                Arg::Liter(_) =>            { return Err("Invalid argument, expected floating point register or register, got literal.") }
                Arg::Label(_) =>            { return Err("Invalid argument, expected floating point register or register, got label.") }
            };
            let reg = match &args[1] {
                Arg::Ureg(n) =>        { if float {return Err("Mismatched register types.")} n }
                Arg::Freg(n) =>        { if !float {return Err("Mismatched register types.")} n }  
                Arg::Liter(_) =>            { return Err("Invalid argument, expected floating point register or register, got literal.") }
                Arg::Label(_) =>            { return Err("Invalid argument, expected floating point register or register, got label.") }
            };
            match &args[2] {
                Arg::Ureg(n) =>        { if float {return Err("Mismatched register types.")} Ok((vec![0x32, *dest_reg, *reg, *n], ci)) }
                Arg::Freg(n) =>        { if !float {return Err("Mismatched register types.")} Ok((vec![0x33, *dest_reg, *reg, *n], ci)) }  
                Arg::Liter(_) =>            { Err("Invalid argument, expected floating point register or register, got literal.") }
                Arg::Label(_) =>            { Err("Invalid argument, expected floating point register or register, got label.") }
            }
        }
        "mul" => {
            let mut float = false;
            let dest_reg = match &args[0] {
                Arg::Ureg(n) =>        { n }
                Arg::Freg(n) =>        { float = true; n }  
                Arg::Liter(_) =>            { return Err("Invalid argument, expected floating point register or register, got literal.") }
                Arg::Label(_) =>            { return Err("Invalid argument, expected floating point register or register, got label.") }
            };
            let reg = match &args[1] {
                Arg::Ureg(n) =>        { if float {return Err("Mismatched register types.")} n }
                Arg::Freg(n) =>        { if !float {return Err("Mismatched register types.")} n }  
                Arg::Liter(_) =>            { return Err("Invalid argument, expected floating point register or register, got literal.") }
                Arg::Label(_) =>            { return Err("Invalid argument, expected floating point register or register, got label.") }
            };
            match &args[2] {
                Arg::Ureg(n) =>        { if float {return Err("Mismatched register types.")} Ok((vec![0x34, *dest_reg, *reg, *n], ci)) }
                Arg::Freg(n) =>        { if !float {return Err("Mismatched register types.")} Ok((vec![0x35, *dest_reg, *reg, *n], ci)) }  
                Arg::Liter(_) =>            { Err("Invalid argument, expected floating point register or register, got literal.") }
                Arg::Label(_) =>            { Err("Invalid argument, expected floating point register or register, got label.") }
            }
        }
        "div" => {
            let mut float = false;
            let dest_reg = match &args[0] {
                Arg::Ureg(n) =>        { n }
                Arg::Freg(n) =>        { float = true; n }  
                Arg::Liter(_) =>            { return Err("Invalid argument, expected floating point register or register, got literal.") }
                Arg::Label(_) =>            { return Err("Invalid argument, expected floating point register or register, got label.") }
            };
            let reg = match &args[1] {
                Arg::Ureg(n) =>        { if float {return Err("Mismatched register types.")} n }
                Arg::Freg(n) =>        { if !float {return Err("Mismatched register types.")} n }  
                Arg::Liter(_) =>            { return Err("Invalid argument, expected floating point register or register, got literal.") }
                Arg::Label(_) =>            { return Err("Invalid argument, expected floating point register or register, got label.") }
            };
            match &args[2] {
                Arg::Ureg(n) =>        { if float {return Err("Mismatched register types.")} Ok((vec![0x36, *dest_reg, *reg, *n], ci)) }
                Arg::Freg(n) =>        { if !float {return Err("Mismatched register types.")} Ok((vec![0x37, *dest_reg, *reg, *n], ci)) }  
                Arg::Liter(_) =>            { Err("Invalid argument, expected floating point register or register, got literal.") }
                Arg::Label(_) =>            { Err("Invalid argument, expected floating point register or register, got label.") }
            }
        }
        "mod" => {
            let mut float = false;
            let dest_reg = match &args[0] {
                Arg::Ureg(n) =>        { n }
                Arg::Freg(n) =>        { float = true; n }  
                Arg::Liter(_) =>            { return Err("Invalid argument, expected floating point register or register, got literal.") }
                Arg::Label(_) =>            { return Err("Invalid argument, expected floating point register or register, got label.") }
            };
            let reg = match &args[1] {
                Arg::Ureg(n) =>        { if float {return Err("Mismatched register types.")} n }
                Arg::Freg(n) =>        { if !float {return Err("Mismatched register types.")} n }  
                Arg::Liter(_) =>            { return Err("Invalid argument, expected floating point register or register, got literal.") }
                Arg::Label(_) =>            { return Err("Invalid argument, expected floating point register or register, got label.") }
            };
            match &args[2] {
                Arg::Ureg(n) =>        { if float {return Err("Mismatched register types.")} Ok((vec![0x38, *dest_reg, *reg, *n], ci)) }
                Arg::Freg(n) =>        { if !float {return Err("Mismatched register types.")} Ok((vec![0x39, *dest_reg, *reg, *n], ci)) }  
                Arg::Liter(_) =>            { Err("Invalid argument, expected floating point register or register, got literal.") }
                Arg::Label(_) =>            { Err("Invalid argument, expected floating point register or register, got label.") }
            }
        }
        "shl" => {
            let dest_reg = match &args[0] {
                Arg::Ureg(n) =>        { n }
                Arg::Freg(_) =>             { return Err("Invalid argument, expected register, got floating point register.") }  
                Arg::Liter(_) =>            { return Err("Invalid argument, expected register, got literal.") }
                Arg::Label(_) =>            { return Err("Invalid argument, expected register, got label.") }
            };
            let reg = match &args[1] {
                Arg::Ureg(n) =>        { n }
                Arg::Freg(_) =>             { return Err("Invalid argument, expected register, got floating point register.") }  
                Arg::Liter(_) =>            { return Err("Invalid argument, expected register, got literal.") }
                Arg::Label(_) =>            { return Err("Invalid argument, expected register, got label.") }
            };
            match &args[2] {
                Arg::Ureg(n) =>        { Ok((vec![0x3A, *dest_reg, *reg, *n], ci)) }
                Arg::Freg(_) =>             { Err("Invalid argument, expected register, got floating point register.") }  
                Arg::Liter(_) =>            { Err("Invalid argument, expected register, got literal.") }
                Arg::Label(_) =>            { Err("Invalid argument, expected register, got label.") }
            }
        }
        "shr" => {
            let dest_reg = match &args[0] {
                Arg::Ureg(n) =>        { n }
                Arg::Freg(_) =>             { return Err("Invalid argument, expected register, got floating point register.") }  
                Arg::Liter(_) =>            { return Err("Invalid argument, expected register, got literal.") }
                Arg::Label(_) =>            { return Err("Invalid argument, expected register, got label.") }
            };
            let reg = match &args[1] {
                Arg::Ureg(n) =>        { n }
                Arg::Freg(_) =>             { return Err("Invalid argument, expected register, got floating point register.") }  
                Arg::Liter(_) =>            { return Err("Invalid argument, expected register, got literal.") }
                Arg::Label(_) =>            { return Err("Invalid argument, expected register, got label.") }
            };
            match &args[2] {
                Arg::Ureg(n) =>        { Ok((vec![0x3B, *dest_reg, *reg, *n], ci)) }
                Arg::Freg(_) =>             { Err("Invalid argument, expected register, got floating point register.") }  
                Arg::Liter(_) =>            { Err("Invalid argument, expected register, got literal.") }
                Arg::Label(_) =>            { Err("Invalid argument, expected register, got label.") }
            }
        }
        "and" => {
            let dest_reg = match &args[0] {
                Arg::Ureg(n) =>        { n }
                Arg::Freg(_) =>             { return Err("Invalid argument, expected register, got floating point register.") }  
                Arg::Liter(_) =>            { return Err("Invalid argument, expected register, got literal.") }
                Arg::Label(_) =>            { return Err("Invalid argument, expected register, got label.") }
            };
            let reg = match &args[1] {
                Arg::Ureg(n) =>        { n }
                Arg::Freg(_) =>             { return Err("Invalid argument, expected register, got floating point register.") }  
                Arg::Liter(_) =>            { return Err("Invalid argument, expected register, got literal.") }
                Arg::Label(_) =>            { return Err("Invalid argument, expected register, got label.") }
            };
            match &args[2] {
                Arg::Ureg(n) =>        { Ok((vec![0x3C, *dest_reg, *reg, *n], ci)) }
                Arg::Freg(_) =>             { Err("Invalid argument, expected register, got floating point register.") }  
                Arg::Liter(_) =>            { Err("Invalid argument, expected register, got literal.") }
                Arg::Label(_) =>            { Err("Invalid argument, expected register, got label.") }
            }
        }
        "or"  => {
            let dest_reg = match &args[0] {
                Arg::Ureg(n) =>        { n }
                Arg::Freg(_) =>             { return Err("Invalid argument, expected register, got floating point register.") }  
                Arg::Liter(_) =>            { return Err("Invalid argument, expected register, got literal.") }
                Arg::Label(_) =>            { return Err("Invalid argument, expected register, got label.") }
            };
            let reg = match &args[1] {
                Arg::Ureg(n) =>        { n }
                Arg::Freg(_) =>             { return Err("Invalid argument, expected register, got floating point register.") }  
                Arg::Liter(_) =>            { return Err("Invalid argument, expected register, got literal.") }
                Arg::Label(_) =>            { return Err("Invalid argument, expected register, got label.") }
            };
            match &args[2] {
                Arg::Ureg(n) =>        { Ok((vec![0x3D, *dest_reg, *reg, *n], ci)) }
                Arg::Freg(_) =>             { Err("Invalid argument, expected register, got floating point register.") }  
                Arg::Liter(_) =>            { Err("Invalid argument, expected register, got literal.") }
                Arg::Label(_) =>            { Err("Invalid argument, expected register, got label.") }
            }
        }
        "xor" => {
            let dest_reg = match &args[0] {
                Arg::Ureg(n) =>        { n }
                Arg::Freg(_) =>             { return Err("Invalid argument, expected register, got floating point register.") }  
                Arg::Liter(_) =>            { return Err("Invalid argument, expected register, got literal.") }
                Arg::Label(_) =>            { return Err("Invalid argument, expected register, got label.") }
            };
            let reg = match &args[1] {
                Arg::Ureg(n) =>        { n }
                Arg::Freg(_) =>             { return Err("Invalid argument, expected register, got floating point register.") }  
                Arg::Liter(_) =>            { return Err("Invalid argument, expected register, got literal.") }
                Arg::Label(_) =>            { return Err("Invalid argument, expected register, got label.") }
            };
            match &args[2] {
                Arg::Ureg(n) =>        { Ok((vec![0x3E, *dest_reg, *reg, *n], ci)) }
                Arg::Freg(_) =>             { Err("Invalid argument, expected register, got floating point register.") }  
                Arg::Liter(_) =>            { Err("Invalid argument, expected register, got literal.") }
                Arg::Label(_) =>            { Err("Invalid argument, expected register, got label.") }
            }
        }
        "not" => {
            let dest_reg = match &args[0] {
                Arg::Ureg(n) =>        { n }
                Arg::Freg(_) =>             { return Err("Invalid argument, expected register, got floating point register.") }  
                Arg::Liter(_) =>            { return Err("Invalid argument, expected register, got literal.") }
                Arg::Label(_) =>            { return Err("Invalid argument, expected register, got label.") }
            };
            match &args[1] {
                Arg::Ureg(n) =>        { Ok((vec![0x3F, *dest_reg, *n], ci)) }
                Arg::Freg(_) =>             { Err("Invalid argument, expected register, got floating point register.") }  
                Arg::Liter(_) =>            { Err("Invalid argument, expected register, got literal.") }
                Arg::Label(_) =>            { Err("Invalid argument, expected register, got label.") }
            }
        }
        "inc" => {
            match &args[0] {
                Arg::Ureg(n) =>        { Ok((vec![0x40, *n], ci)) }
                Arg::Freg(_) =>             { Err("Invalid argument, expected register, got floating point register.") }  
                Arg::Liter(_) =>            { Err("Invalid argument, expected register, got literal.") }
                Arg::Label(_) =>            { Err("Invalid argument, expected register, got label.") }
            }
        }
        "dec" => {
            match &args[0] {
                Arg::Ureg(n) =>        { Ok((vec![0x41, *n], ci)) }
                Arg::Freg(_) =>             { Err("Invalid argument, expected register, got floating point register.") }  
                Arg::Liter(_) =>            { Err("Invalid argument, expected register, got literal.") }
                Arg::Label(_) =>            { Err("Invalid argument, expected register, got label.") }
            }
        }
        "psh" => {
            match &args[0] {
                Arg::Ureg(n) =>        { Ok((vec![0x42, *n], ci)) }
                Arg::Freg(n) =>        { Ok((vec![0x43, *n], ci)) }  
                Arg::Liter(_) =>            { Err("Invalid argument, expected floating point register or register, got literal.") }
                Arg::Label(_) =>            { Err("Invalid argument, expected floating point register or register, got label.") }
            }
        }
        "pop" => {
            match &args[0] {
                Arg::Ureg(n) =>        { Ok((vec![0x44, *n], ci)) }
                Arg::Freg(n) =>        { Ok((vec![0x45, *n], ci)) }  
                Arg::Liter(_) =>            { Err("Invalid argument, expected floating point register or register, got literal.") }
                Arg::Label(_) =>            { Err("Invalid argument, expected floating point register or register, got label.") }
            }
        }
        "adc" => {
            match &args[0] {
                Arg::Ureg(n) =>        { Ok((vec![0x46, *n], ci)) }
                Arg::Freg(_) =>             { Err("Invalid argument, expected register, got floating point register.") }  
                Arg::Liter(_) =>            { Err("Invalid argument, expected register, got literal.") }
                Arg::Label(_) =>            { Err("Invalid argument, expected register, got label.") }
            }
        }
        "sbc" => {
            match &args[0] {
                Arg::Ureg(n) =>        { Ok((vec![0x47, *n], ci)) }
                Arg::Freg(_) =>             { Err("Invalid argument, expected register, got floating point register.") }  
                Arg::Liter(_) =>            { Err("Invalid argument, expected register, got literal.") }
                Arg::Label(_) =>            { Err("Invalid argument, expected register, got label.") }
            }
        }
        "scf" => { Ok((vec![0x48], ci)) }
        "ccf" => { Ok((vec![0x49], ci)) }

        "jmp" => {
            match &args[0] {
                Arg::Ureg(n) =>        { Ok((vec![0x51, *n], ci)) },
                Arg::Freg(_) =>             { Err("Invalid argument, expected label or register, got floating point register.") },            
                Arg::Liter(n) =>  { let mut b = vec![0x50]; b.extend_from_slice(&n[4..]); Ok((b, ci)) },
                Arg::Label(n) =>  { let mut b = vec![0x50]; b.extend_from_slice(n); Ok((b, rl)) },
            }
        }
        "jlg" => {
            let mut float = false;
            let reg0 = match &args[0] {
                Arg::Ureg(n) =>        { n }
                Arg::Freg(n) =>        { float = true; n }  
                Arg::Liter(_) =>            { return Err("Invalid argument, expected floating point register or register, got literal.") }
                Arg::Label(_) =>            { return Err("Invalid argument, expected floating point register or register, got label.") }
            };
            let reg1 = match &args[1] {
                Arg::Ureg(n) =>        { if float {return Err("Mismatched register types.")} n }
                Arg::Freg(n) =>        { if !float {return Err("Mismatched register types.")} n }  
                Arg::Liter(_) =>            { return Err("Invalid argument, expected floating point register or register, got literal.") }
                Arg::Label(_) =>            { return Err("Invalid argument, expected floating point register or register, got label.") }
            };
            match &args[2] {
                Arg::Ureg(n) =>        { Ok((vec![if float {0x55} else {0x53}, *reg0, *reg1, *n], ci)) },
                Arg::Freg(_) =>             { Err("Invalid argument, expected label or register, got floating point register.") },            
                Arg::Liter(n) =>  { let mut b = vec![if float {0x54} else {0x52}, *reg0, *reg1]; b.extend_from_slice(&n[4..]); Ok((b, ci)) },
                Arg::Label(n) =>  { let mut b = vec![if float {0x54} else {0x52}, *reg0, *reg1]; b.extend_from_slice(n); Ok((b, rl)) },
            }
        }
        "jpe" => {
            let mut float = false;
            let reg0 = match &args[0] {
                Arg::Ureg(n) =>        { n }
                Arg::Freg(n) =>        { float = true; n }  
                Arg::Liter(_) =>            { return Err("Invalid argument, expected floating point register or register, got literal.") }
                Arg::Label(_) =>            { return Err("Invalid argument, expected floating point register or register, got label.") }
            };
            let reg1 = match &args[1] {
                Arg::Ureg(n) =>        { if float {return Err("Mismatched register types.")} n }
                Arg::Freg(n) =>        { if !float {return Err("Mismatched register types.")} n }  
                Arg::Liter(_) =>            { return Err("Invalid argument, expected floating point register or register, got literal.") }
                Arg::Label(_) =>            { return Err("Invalid argument, expected floating point register or register, got label.") }
            };
            match &args[2] {
                Arg::Ureg(n) =>        { Ok((vec![if float {0x59} else {0x57}, *reg0, *reg1, *n], ci)) },
                Arg::Freg(_) =>             { Err("Invalid argument, expected label or register, got floating point register.") },            
                Arg::Liter(n) =>  { let mut b = vec![if float {0x58} else {0x56}, *reg0, *reg1]; b.extend_from_slice(&n[4..]); Ok((b, ci)) },
                Arg::Label(n) =>  { let mut b = vec![if float {0x58} else {0x56}, *reg0, *reg1]; b.extend_from_slice(n); Ok((b, rl)) },
            }
        }
        "jne" => {
            let mut float = false;
            let reg0 = match &args[0] {
                Arg::Ureg(n) =>        { n }
                Arg::Freg(n) =>        { float = true; n }  
                Arg::Liter(_) =>            { return Err("Invalid argument, expected floating point register or register, got literal.") }
                Arg::Label(_) =>            { return Err("Invalid argument, expected floating point register or register, got label.") }
            };
            let reg1 = match &args[1] {
                Arg::Ureg(n) =>        { if float {return Err("Mismatched register types.")} n }
                Arg::Freg(n) =>        { if !float {return Err("Mismatched register types.")} n }  
                Arg::Liter(_) =>            { return Err("Invalid argument, expected floating point register or register, got literal.") }
                Arg::Label(_) =>            { return Err("Invalid argument, expected floating point register or register, got label.") }
            };
            match &args[2] {
                Arg::Ureg(n) =>        { Ok((vec![if float {0x5D} else {0x5B}, *reg0, *reg1, *n], ci)) },
                Arg::Freg(_) =>             { Err("Invalid argument, expected label or register, got floating point register.") },            
                Arg::Liter(n) =>  { let mut b = vec![if float {0x5C} else {0x5A}, *reg0, *reg1]; b.extend_from_slice(&n[4..]); Ok((b, ci)) },
                Arg::Label(n) =>  { let mut b = vec![if float {0x5C} else {0x5A}, *reg0, *reg1]; b.extend_from_slice(n); Ok((b, rl)) },
            }
        }
        "jpc" => { 
            match &args[0] {
                Arg::Ureg(n) =>        { Ok((vec![0x5F, *n], ci)) },
                Arg::Freg(_) =>             { Err("Invalid argument, expected label or register, got floating point register.") },            
                Arg::Liter(n) =>  { let mut b = vec![0x5E]; b.extend_from_slice(&n[4..]); Ok((b, ci)) },
                Arg::Label(n) =>  { let mut b = vec![0x5E]; b.extend_from_slice(n); Ok((b, rl)) },
            }
        }
        "jnc" => {
            match &args[0] {
                Arg::Ureg(n) =>        { Ok((vec![0x61, *n], ci)) },
                Arg::Freg(_) =>             { Err("Invalid argument, expected label or register, got floating point register.") },            
                Arg::Liter(n) =>  { let mut b = vec![0x60]; b.extend_from_slice(&n[4..]); Ok((b, ci)) },
                Arg::Label(n) =>  { let mut b = vec![0x60]; b.extend_from_slice(n); Ok((b, rl)) },
            }
        }

        "hlt" => { Ok((vec![0x70], ci)) }
        "wit" => { 
            match &args[0] {
                Arg::Ureg(n) =>        { Ok((vec![0x72, *n], ci)) }
                Arg::Freg(_) =>             { Err("Invalid argument, expected literal or register, got floating point register.") }            
                Arg::Liter(n) =>  { let mut b = vec![0x71]; b.extend_from_slice(&n[8..]); Ok((b, ci)) }
                Arg::Label(_) =>            { Err("Invalid argument, expected literal or register, got label.") }
            }
        }
        "gst" => {
            match &args[0] {
                Arg::Ureg(n) =>        { Ok((vec![0x73, *n], ci)) }
                Arg::Freg(_) =>             { Err("Invalid argument, expected register, got floating point register.") }  
                Arg::Liter(_) =>            { Err("Invalid argument, expected register, got literal.") }
                Arg::Label(_) =>            { Err("Invalid argument, expected register, got label.") }
            }
        }
        "gpc" => {
            match &args[0] {
                Arg::Ureg(n) =>        { Ok((vec![0x74, *n], ci)) }
                Arg::Freg(_) =>             { Err("Invalid argument, expected register, got floating point register.") }  
                Arg::Liter(_) =>            { Err("Invalid argument, expected register, got literal.") }
                Arg::Label(_) =>            { Err("Invalid argument, expected register, got label.") }
            }
        }
        
        "syscall" => { Ok((vec![0x80], ci)) }
        "sysret" =>  { Ok((vec![0x81], ci)) }
        "memcpy" => {
            match (&args[0], &args[1], &args[2]) {
                (Arg::Liter(dst), Arg::Liter(src), Arg::Liter(len)) => { let mut b = vec![0x82]; b.extend_from_slice(&src[4..]); b.extend_from_slice(&dst[4..]); b.extend_from_slice(&len[5..]); Ok((b, ci)) }
                (Arg::Ureg(dst), Arg::Ureg(src), Arg::Liter(len)) => { let mut b = vec![0x83, *src, *dst]; b.extend_from_slice(&len[5..]); Ok((b, ci)) }
                (Arg::Ureg(dst), Arg::Ureg(src), Arg::Ureg(len)) => { Ok((vec![0x84, *src, *dst, *len], ci)) }
            
                _ => { Err("Invalid arguments.") }
            }
        }

        "out" => {
            let reg = match &args[0] {
                Arg::Ureg(n) =>        { n }
                Arg::Freg(_) =>             { return Err("Invalid argument, expected register, got floating point register.") }  
                Arg::Liter(_) =>            { return Err("Invalid argument, expected register, got literal.") }
                Arg::Label(_) =>            { return Err("Invalid argument, expected register, got label.") }
            };
            match &args[1] {
                Arg::Ureg(n) =>        { Ok((vec![0x91, *reg, *n], ci)) }
                Arg::Freg(_) =>             { Err("Invalid argument, expected literal or register, got floating point register.") }  
                Arg::Liter(n) =>  { let mut b = vec![0x90, *reg]; b.extend_from_slice(&n[6..]); Ok((b, ci)) }
                Arg::Label(_) =>            { Err("Invalid argument, expected literal or register, got label.") }
            }
        }
        "in"  => {
            let reg = match &args[0] {
                Arg::Ureg(n) =>        { n }
                Arg::Freg(_) =>             { return Err("Invalid argument, expected register, got floating point register.") }  
                Arg::Liter(_) =>            { return Err("Invalid argument, expected register, got literal.") }
                Arg::Label(_) =>            { return Err("Invalid argument, expected register, got label.") }
            };
            match &args[1] {
                Arg::Ureg(n) =>        { Ok((vec![0x93, *reg, *n], ci)) }
                Arg::Freg(_) =>             { Err("Invalid argument, expected literal or register, got floating point register.") }  
                Arg::Liter(n) =>  { let mut b = vec![0x92, *reg]; b.extend_from_slice(&n[6..]); Ok((b, ci)) }
                Arg::Label(_) =>            { Err("Invalid argument, expected literal or register, got label.") }
            }
        }

        "grapcpy" => {
            match (&args[0], &args[1], &args[2], &args[3], &args[4], &args[5]) {
                (Arg::Ureg(dst), Arg::Liter(src), Arg::Liter(x), Arg::Liter(y), Arg::Liter(w), Arg::Liter(h)) => { let mut b = vec![0xA0]; b.extend_from_slice(&src[4..]); b.push(*dst); b.extend_from_slice(&h[6..]); b.extend_from_slice(&w[6..]); b.extend_from_slice(&x[6..]); b.extend_from_slice(&y[6..]); Ok((b, ci)) }
                (Arg::Ureg(dst), Arg::Label(src), Arg::Liter(x), Arg::Liter(y), Arg::Liter(w), Arg::Liter(h)) => { let mut b = vec![0xA0]; b.extend_from_slice(src); b.push(*dst); b.extend_from_slice(&h[6..]); b.extend_from_slice(&w[6..]); b.extend_from_slice(&x[6..]); b.extend_from_slice(&y[6..]); Ok((b, rd)) }
                (Arg::Ureg(dst), Arg::Ureg(src), Arg::Ureg(x), Arg::Ureg(y), Arg::Ureg(w), Arg::Ureg(h)) => { Ok((vec![0xA1, *src, *dst, *h, *w, *x, *y], ci)) }
            
                _ => { Err("Invalid arguments.") }
            }
        }

        "db" => {
            let mut b = Vec::new();
            for arg in args { 
                match arg {
                    Arg::Ureg(_) =>             { return Err("Invalid argument, expected literal, got register.") }  
                    Arg::Freg(_) =>             { return Err("Invalid argument, expected literal, got floating point register.") }  
                    Arg::Liter(n) =>   { b.extend_from_slice(&n[7..]) }
                    Arg::Label(_) =>            { return Err("Invalid argument, expected literal, got label.") }           
                }    
            }
            Ok((b, ci))
        }

        _ => { Err("Invalid instruction.") }
    }
}

fn error(line: Line, error: &str) {
    let form_err = format!(indoc! {"
        
        {}: {}
            {} {}:{}
             {}
        {  } {} {} {}
             {}
    "}, 
    "Error".red().bold(), error.bold(),
    "-->".bright_cyan().bold(), line.4, line.5,
    "|".bright_cyan().bold(), 
    format!("{:4}", line.5).bright_cyan().bold(), "|".bright_cyan().bold(), line.2, line.3.join(" "),
    "|".bright_cyan().bold(),
    );

    println!("{}", form_err);

    //process::exit(0);
}


#[cfg(test)]
mod tests {
    use super::*;

    impl Arg {
        fn new(args: &str) -> Vec<Arg> {
            let mut rargs = Vec::new();
            let args = args.split(" ").map(|s| s.to_owned()).collect::<Vec<String>>();
            
            for arg in args {
                rargs.push(resolve_arg(arg.to_owned()).unwrap());
            }
            rargs
        }
    }

    #[test]
    fn nop_00() {
        assert_eq!(resolve_inst(String::from("nop"), Arg::new("")).unwrap().0, vec![0x00]);
    }

    #[test]
    fn mov_01() {
        assert_eq!(resolve_inst(String::from("mov"), Arg::new("r0 r1")).unwrap().0, vec![0x01, 0x00, 0x01]);
    }

    #[test]
    fn mov_02() {
        assert_eq!(resolve_inst(String::from("mov"), Arg::new("f0 f1")).unwrap().0, vec![0x02, 0x00, 0x01]);
    }

    #[test]
    fn mov_03() {
        assert_eq!(resolve_inst(String::from("mov"), Arg::new("f0 r1")).unwrap().0, vec![0x03, 0x00, 0x01]);
    }

    #[test]
    fn mov_04() {
        assert_eq!(resolve_inst(String::from("mov"), Arg::new("r0 f1")).unwrap().0, vec![0x04, 0x00, 0x01]);
    }

    #[test]
    fn mov_05() {
        assert_eq!(resolve_inst(String::from("mov"), Arg::new("r0 &1234")).unwrap().0, vec![0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x12, 0x34]);
    }

    #[test]
    fn mov_06() {
        assert_eq!(resolve_inst(String::from("mov"), Arg::new("f0 &1234")).unwrap().0, vec![0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x12, 0x34]);
    }

    #[test]
    fn mov_07() {
        assert_eq!(resolve_inst(String::from("mov"), Arg::new("r0 1 &1234")).unwrap().0, vec![0x07, 0x00, 0x01, 0x00, 0x00, 0x12, 0x34]);
    }

    #[test]
    fn mov_08() {
        assert_eq!(resolve_inst(String::from("mov"), Arg::new("f0 1 &1234")).unwrap().0, vec![0x08, 0x00, 0x01, 0x00, 0x00, 0x12, 0x34]);
    }

    #[test]
    fn mov_09() {
        assert_eq!(resolve_inst(String::from("mov"), Arg::new("&1234 r0 1")).unwrap().0, vec![0x09, 0x00, 0x00, 0x12, 0x34, 0x01, 0x00]);
    }

    #[test]
    fn mov_0a() {
        assert_eq!(resolve_inst(String::from("mov"), Arg::new("&1234 f0 1")).unwrap().0, vec![0x0A, 0x00, 0x00, 0x12, 0x34, 0x01, 0x00]);
    }

    #[test]
    fn mov_0b() {
        assert_eq!(resolve_inst(String::from("mov"), Arg::new("r0 r1 2")).unwrap().0, vec![0x0B, 0x00, 0x02, 0x01]);
    }

    #[test]
    fn mov_0c() {
        assert_eq!(resolve_inst(String::from("mov"), Arg::new("r0 f1 2")).unwrap().0, vec![0x0C, 0x00, 0x02, 0x01]);
    }

    #[test]
    fn mov_0d() {
        assert_eq!(resolve_inst(String::from("mov"), Arg::new("r0 2 r1")).unwrap().0, vec![0x0D, 0x00, 0x02, 0x01]);
    }

    #[test]
    fn mov_0e() {
        assert_eq!(resolve_inst(String::from("mov"), Arg::new("f0 2 r1")).unwrap().0, vec![0x0E, 0x00, 0x02, 0x01]);
    }

    #[test]
    fn mov_0f() {
        assert_eq!(resolve_inst(String::from("mva"), Arg::new("r0 1 &1234")).unwrap().0, vec![0x0F, 0x00, 0x01, 0x00, 0x00, 0x12, 0x34]);
    }

    #[test]
    fn mov_10() {
        assert_eq!(resolve_inst(String::from("mva"), Arg::new("f0 1 &1234")).unwrap().0, vec![0x10, 0x00, 0x01, 0x00, 0x00, 0x12, 0x34]);
    }

    #[test]
    fn mov_11() {
        assert_eq!(resolve_inst(String::from("mva"), Arg::new("&1234 r0 1")).unwrap().0, vec![0x11, 0x00, 0x00, 0x12, 0x34, 0x01, 0x00]);
    }

    #[test]
    fn mov_12() {
        assert_eq!(resolve_inst(String::from("mva"), Arg::new("&1234 f0 1")).unwrap().0, vec![0x12, 0x00, 0x00, 0x12, 0x34, 0x01, 0x00]);
    }

    #[test]
    fn mov_13() {
        assert_eq!(resolve_inst(String::from("mva"), Arg::new("r0 r1 2")).unwrap().0, vec![0x13, 0x00, 0x02, 0x01]);
    }

    #[test]
    fn mov_14() {
        assert_eq!(resolve_inst(String::from("mva"), Arg::new("r0 f1 2")).unwrap().0, vec![0x14, 0x00, 0x02, 0x01]);
    }

    #[test]
    fn mov_15() {
        assert_eq!(resolve_inst(String::from("mva"), Arg::new("r0 2 r1")).unwrap().0, vec![0x15, 0x00, 0x02, 0x01]);
    }

    #[test]
    fn mov_16() {
        assert_eq!(resolve_inst(String::from("mva"), Arg::new("f0 2 r1")).unwrap().0, vec![0x16, 0x00, 0x02, 0x01]);
    }

    #[test]
    fn mov_17() {
        assert_eq!(resolve_inst(String::from("mov"), Arg::new("&1234 r0 r1 2")).unwrap().0, vec![0x17, 0x00, 0x02, 0x01, 0x00, 0x00, 0x12, 0x34]);
    }

    #[test]
    fn mov_18() {
        assert_eq!(resolve_inst(String::from("mov"), Arg::new("&1234 r0 f1 2")).unwrap().0, vec![0x18, 0x00, 0x02, 0x01, 0x00, 0x00, 0x12, 0x34]);
    }

    #[test]
    fn mov_19() {
        assert_eq!(resolve_inst(String::from("mov"), Arg::new("r0 2 &1234 r1")).unwrap().0, vec![0x19, 0x00, 0x02, 0x01, 0x00, 0x00, 0x12, 0x34]);
    }

    #[test]
    fn mov_1a() {
        assert_eq!(resolve_inst(String::from("mov"), Arg::new("f0 2 &1234 r1")).unwrap().0, vec![0x1A, 0x00, 0x02, 0x01, 0x00, 0x00, 0x12, 0x34]);
    }

    #[test]
    fn mov_1b() {
        assert_eq!(resolve_inst(String::from("mva"), Arg::new("&1234 r0 r1 2")).unwrap().0, vec![0x1B, 0x00, 0x02, 0x01, 0x00, 0x00, 0x12, 0x34]);
    }

    #[test]
    fn mov_1c() {
        assert_eq!(resolve_inst(String::from("mva"), Arg::new("&1234 r0 f1 2")).unwrap().0, vec![0x1C, 0x00, 0x02, 0x01, 0x00, 0x00, 0x12, 0x34]);
    }

    #[test]
    fn mov_1d() {
        assert_eq!(resolve_inst(String::from("mva"), Arg::new("r0 2 &1234 r1")).unwrap().0, vec![0x1D, 0x00, 0x02, 0x01, 0x00, 0x00, 0x12, 0x34]);
    }

    #[test]
    fn mov_1e() {
        assert_eq!(resolve_inst(String::from("mva"), Arg::new("f0 2 &1234 r1")).unwrap().0, vec![0x1E, 0x00, 0x02, 0x01, 0x00, 0x00, 0x12, 0x34]);
    }

    #[test]
    fn mov_1f() {
        assert_eq!(resolve_inst(String::from("mvd"), Arg::new("r0 1 &1234")).unwrap().0, vec![0x1F, 0x00, 0x01, 0x00, 0x00, 0x12, 0x34]);
    }

    #[test]
    fn mov_20() {
        assert_eq!(resolve_inst(String::from("mvd"), Arg::new("f0 1 &1234")).unwrap().0, vec![0x20, 0x00, 0x01, 0x00, 0x00, 0x12, 0x34]);
    }

    #[test]
    fn mov_21() {
        assert_eq!(resolve_inst(String::from("mvd"), Arg::new("&1234 r0 1")).unwrap().0, vec![0x21, 0x00, 0x00, 0x12, 0x34, 0x01, 0x00]);
    }

    #[test]
    fn mov_22() {
        assert_eq!(resolve_inst(String::from("mvd"), Arg::new("&1234 f0 1")).unwrap().0, vec![0x22, 0x00, 0x00, 0x12, 0x34, 0x01, 0x00]);
    }

    #[test]
    fn mov_23() {
        assert_eq!(resolve_inst(String::from("mvd"), Arg::new("r0 r1 2")).unwrap().0, vec![0x23, 0x00, 0x02, 0x01]);
    }

    #[test]
    fn mov_24() {
        assert_eq!(resolve_inst(String::from("mvd"), Arg::new("r0 f1 2")).unwrap().0, vec![0x24, 0x00, 0x02, 0x01]);
    }

    #[test]
    fn mov_25() {
        assert_eq!(resolve_inst(String::from("mvd"), Arg::new("r0 2 r1")).unwrap().0, vec![0x25, 0x00, 0x02, 0x01]);
    }

    #[test]
    fn mov_26() {
        assert_eq!(resolve_inst(String::from("mvd"), Arg::new("f0 2 r1")).unwrap().0, vec![0x26, 0x00, 0x02, 0x01]);
    }

    #[test]
    fn mov_27() {
        assert_eq!(resolve_inst(String::from("mvd"), Arg::new("&1234 r0 r1 2")).unwrap().0, vec![0x27, 0x00, 0x02, 0x01, 0x00, 0x00, 0x12, 0x34]);
    }

    #[test]
    fn mov_28() {
        assert_eq!(resolve_inst(String::from("mvd"), Arg::new("&1234 r0 f1 2")).unwrap().0, vec![0x28, 0x00, 0x02, 0x01, 0x00, 0x00, 0x12, 0x34]);
    }

    #[test]
    fn mov_29() {
        assert_eq!(resolve_inst(String::from("mvd"), Arg::new("r0 2 &1234 r1")).unwrap().0, vec![0x29, 0x00, 0x02, 0x01, 0x00, 0x00, 0x12, 0x34]);
    }

    #[test]
    fn mov_2a() {
        assert_eq!(resolve_inst(String::from("mvd"), Arg::new("f0 2 &1234 r1")).unwrap().0, vec![0x2A, 0x00, 0x02, 0x01, 0x00, 0x00, 0x12, 0x34]);
    }
}