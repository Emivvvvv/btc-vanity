use std::time::Instant;
use btc_vanity::vanity_addr_generator::{VanityAddr, VanityMode};
use btc_vanity::clap::{cli};
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use btc_vanity::error::CustomError;

const MODE: [&str; 3] = ["has the prefix", "has the suffix", "has the string"];
const CASE_SENSITIVITY: [&str; 2] = ["(case sensitive)", "(case sensitivity disabled)"];

// Gets all strings from the input file. All strings must be seperated by a new line.
fn get_strings_from_file(file_name: &String) -> Result<Vec<String>, Box<dyn std::error::Error>>{
    let data = fs::read_to_string(file_name)?;
    let lines: Vec<&str> = data.lines().collect::<Vec<_>>();
    let strings = lines.into_iter().map(|line| line.to_string()).collect();
    Ok(strings)
}

// If file already exists appends else creates an output text file and writes all the found key pairs and addresses.
fn write_output_file(output_file_name: &String, buffer: &String) -> Result<(), Box<dyn std::error::Error>> {
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

fn main() {
    // Sets the cli app.
    let app = cli();

    // Gets all the arguments from the cli.
    let matches = app.get_matches();
    let threads = matches.get_one::<String>("threads")
        .expect("This was unexpected :(. Something went wrong while getting -t or --threads arg")
        .trim().parse::<u64>()
        .expect("Threads must be a number!");
    let is_case_sensitive = matches.get_flag("case-sensitive");
    let is_fast_disabled = matches.get_flag("disable-fast-mode");
    let output_file_name = match matches.get_one::<String>("output-file") {
        Some(output_file_name) => output_file_name.to_string(),
        None => String::from(""),
    };
    let strings = match matches.get_one::<String>("string") {
        Some(string) => vec![string.to_owned()],
        None => {
            let file_name = matches.get_one::<String>("input-file").unwrap();
            get_strings_from_file(file_name).unwrap()
        }
    };

    // Sets vanity_mode for searching and mode to predefined decoration strings.
    let (vanity_mode, mode) =
        if matches.get_flag("anywhere") { (VanityMode::Anywhere, MODE[2]) }
        else if matches.get_flag("suffix") { (VanityMode::Suffix, MODE[1]) }
        else { (VanityMode::Prefix, MODE[0]) };

    // Sets case sensitivity decoration string.
    let case_sensitive = match is_case_sensitive {
        true => CASE_SENSITIVITY[0],
        false => CASE_SENSITIVITY[1],
    };

    // Loop for multiple wallet inputs from text file.
    for string in strings {
        let mut buffer1 = String::new();
        println!("Searching key pair which their address {}: '{}' {} with {} threads.\n",
                 mode,
                 string,
                 case_sensitive,
                 threads);

        if !output_file_name.is_empty() { buffer1 = format!("Key pair which their address {}: '{}' {}\n",
                                                            mode,
                                                            string,
                                                            case_sensitive) }

        // Generates the vanity address and measures the time elapsed while finding the address.
        let start = Instant::now();
        let result = VanityAddr::generate(
            string,
            threads,
            is_case_sensitive,
            !is_fast_disabled,
            vanity_mode);
        let seconds = start.elapsed().as_secs_f64();

        #[allow(unused_assignments)]
        let mut buffer2 = String::new();

        match result{
            Ok(res) => {
                println!("FOUND IN {:.4} SECONDS!\n", seconds);
                // Prints the found key pair and the address which has the string.
                buffer2 = format!(
                    "private_key (wif): {}\n\
                    public_key (compressed): {}\n\
                    address (compressed): {}\n\n",
                        res.get_wif_private_key(),
                        res.get_comp_public_key(),
                        res.get_comp_address())
            },
            Err(err) => buffer2 = format!("Skipping because of error: {}\n\n", err),
        }

        if !output_file_name.is_empty() { write_output_file(&output_file_name, &format!("{}\n{}", buffer1, buffer2)).unwrap() }
        else {println!("{}", buffer2)}
    }
}