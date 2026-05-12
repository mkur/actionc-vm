use std::env;
use std::path::PathBuf;

use action_compiler_vm::{ACTION_OS_PRESET, ImageKind, VmConfig};

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        print_help();
        return Ok(());
    };

    match command.as_str() {
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        "inspect" => inspect(parse_options(args.collect())?),
        "run" => run_vm(parse_options(args.collect())?),
        other => Err(format!("unknown command `{other}`")),
    }
}

fn inspect(config: VmConfig) -> Result<(), String> {
    let vm = config.load()?;
    println!("loaded {} image(s)", vm.images().len());
    for image in vm.images() {
        println!(
            "{:?}: {} byte(s), ${:04X}-${:04X}, checksum16=${:04X}, crc32=${:08X}",
            image.kind,
            image.metadata.size,
            image.metadata.base,
            image.metadata.end,
            image.metadata.checksum16,
            image.metadata.crc32
        );
    }
    Ok(())
}

fn run_vm(config: VmConfig) -> Result<(), String> {
    config.validate_for_execution()?;
    let vm = config.load()?;
    println!(
        "compiler VM skeleton loaded {} image(s); CPU execution is not implemented yet",
        vm.images().len()
    );
    Ok(())
}

fn parse_options(args: Vec<String>) -> Result<VmConfig, String> {
    let mut config = VmConfig::default();
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--preset" => {
                index += 1;
                let value = required_value(&args, index, "--preset")?;
                apply_preset(&mut config, value)?;
            }
            "--cart" => {
                index += 1;
                let path = required_value(&args, index, "--cart")?;
                config.cartridge = Some(PathBuf::from(path));
            }
            "--cart-base" => {
                index += 1;
                let value = required_value(&args, index, "--cart-base")?;
                config.cartridge_base = parse_address(value)?;
            }
            "--os" => {
                index += 1;
                let path = required_value(&args, index, "--os")?;
                config.os_rom = Some(PathBuf::from(path));
            }
            "--os-base" => {
                index += 1;
                let value = required_value(&args, index, "--os-base")?;
                config.os_base = parse_address(value)?;
            }
            "--source" => {
                index += 1;
                let path = required_value(&args, index, "--source")?;
                config.source = Some(PathBuf::from(path));
            }
            "--map" => {
                index += 1;
                let value = required_value(&args, index, "--map")?;
                config.extra_images.push(parse_image_map(value)?);
            }
            other => return Err(format!("unknown option `{other}`")),
        }
        index += 1;
    }

    Ok(config)
}

fn apply_preset(config: &mut VmConfig, value: &str) -> Result<(), String> {
    match value {
        "action-os" => {
            config.apply_preset(ACTION_OS_PRESET);
            Ok(())
        }
        other => Err(format!("unknown preset `{other}`")),
    }
}

fn required_value<'a>(args: &'a [String], index: usize, option: &str) -> Result<&'a str, String> {
    args.get(index)
        .map(String::as_str)
        .ok_or_else(|| format!("{option} requires a value"))
}

fn parse_image_map(value: &str) -> Result<(ImageKind, PathBuf, u16), String> {
    let mut parts = value.splitn(3, ':');
    let kind = match parts.next() {
        Some("ram") => ImageKind::Ram,
        Some("rom") => ImageKind::Rom,
        Some("cart") => ImageKind::Cartridge,
        Some(other) => return Err(format!("unknown image kind `{other}`")),
        None => return Err("missing image kind".to_string()),
    };
    let path = parts
        .next()
        .ok_or_else(|| "image map must be kind:path:address".to_string())?;
    let base = parts
        .next()
        .ok_or_else(|| "image map must be kind:path:address".to_string())
        .and_then(parse_address)?;

    Ok((kind, PathBuf::from(path), base))
}

fn parse_address(value: &str) -> Result<u16, String> {
    let trimmed = value.trim();
    let parsed = if let Some(hex) = trimmed.strip_prefix('$') {
        u16::from_str_radix(hex, 16)
    } else if let Some(hex) = trimmed.strip_prefix("0x") {
        u16::from_str_radix(hex, 16)
    } else {
        trimmed.parse()
    };

    parsed.map_err(|_| format!("invalid address `{value}`"))
}

fn print_help() {
    println!(
        "action-compiler-vm\n\n\
         Usage:\n  \
         action-compiler-vm inspect [options]\n  \
         action-compiler-vm run [options]\n\n\
         Options:\n  \
         --preset <name>      Mapping preset, currently action-os\n  \
         --cart <path>        Load an Action! cartridge image\n  \
         --cart-base <addr>   Cartridge base address, default $A000\n  \
         --os <path>          Load an Atari OS ROM image at $C000\n  \
         --os-base <addr>     OS ROM base address, default $C000\n  \
         --source <path>      Source file reserved for the future compiler harness\n  \
         --map <k:p:a>        Map an extra image: ram:path:addr, rom:path:addr, cart:path:addr"
    );
}
