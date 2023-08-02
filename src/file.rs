use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use crate::error::CustomError;
use crate::vanity_addr_generator::VanityMode;

pub struct FileFlags {
    pub force_flags: bool,
    pub is_case_sensitive: bool,
    pub disable_fast_mode: bool,
    pub output_file_name: Option<String>,
    pub vanity_mode: Option<VanityMode>,
}

impl FileFlags {
    pub fn use_cli_flags() -> Self {
        FileFlags {
            force_flags: false,
            is_case_sensitive: false,
            disable_fast_mode: false,
            output_file_name: None,
            vanity_mode: None,
        }
    }
}

pub fn get_flags(line: &String) -> FileFlags {
    let args = line.split(' ').collect::<Vec<_>>();

    if args.len() == 1 { return FileFlags::use_cli_flags() }

    let force_flags = args.contains(&"-f") || args.contains(&"--force-flags");
    let is_case_sensitive = args.contains(&"-c") || args.contains(&"--case-sensitive");
    let disable_fast_mode = args.contains(&"-d") || args.contains(&"--disable-fast");
    let vanity_option = args.iter().find(|&&arg| arg == "-p" || arg == "-s" || arg == "-a" || arg == "--prefix" || arg == "--suffix" || arg == "--anywhere");
    let vanity_mode = match vanity_option {
        Some(vanity) => {
            let vanity = vanity.to_string();
            if vanity == String::from("-p") || vanity == String::from("--prefix") { Some(VanityMode::Prefix) }
            else if vanity == String::from("-s") || vanity == String::from("--suffix") { Some(VanityMode::Suffix) }
            else { Some(VanityMode::Anywhere) }
        },
        None => None,
    };
    let ofn_index = args.iter().position(|&arg| arg == "-o" || arg == "--output-file");
    let output_file_name = match ofn_index {
        Some(i) => Some(args[i + 1].to_string()),
        None => None,
    };

    FileFlags {
        force_flags,
        is_case_sensitive,
        disable_fast_mode,
        output_file_name,
        vanity_mode,
    }
}

// Gets all strings from the input file. All strings must be seperated by a new line.
pub fn get_strings_and_flags_from_file(file_name: &String) -> Result<(Vec<String>, Vec<FileFlags>), Box<dyn std::error::Error>>{
    let data = fs::read_to_string(file_name)?;
    let lines: Vec<&str> = data.lines().collect::<Vec<_>>();
    let strings: Vec<_> = lines.iter().map(|line| {
        let line_split = line.split(' ').collect::<Vec<_>>();
        line_split[0].to_string()
    }).collect();
    let flags: Vec<FileFlags> = lines.iter().map(|&string| get_flags(&string.to_string())).collect();

    Ok((strings, flags))
}

// If file already exists appends else creates an output text file and writes all the found key pairs and addresses.
pub fn write_output_file(output_file_name: &String, buffer: &String) -> Result<(), Box<dyn std::error::Error>> {
    let ofn_len = output_file_name.len();
    if &output_file_name[ofn_len - 4..ofn_len] != ".txt" { return Err(Box::new(CustomError("file must be a text file. ex: output.txt"))) }
    let file_result = OpenOptions::new().append(true).open(output_file_name);
    let mut file = match file_result {
        Ok(file) => file,
        Err(_) => fs::File::create(output_file_name)?,
    };

    file.write_all(buffer.as_bytes())?;
    Ok(())
}