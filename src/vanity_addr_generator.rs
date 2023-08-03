use crate::keys_and_address::KeysAndAddress;
use crate::error::CustomError;
use std::thread;
use std::sync::mpsc;

pub struct VanityAddr;

#[derive(Copy, Clone)]
pub enum  VanityMode {
    Prefix,
    Suffix,
    Anywhere,
}

impl VanityAddr {
    pub fn generate(
        string: &str,
        threads: u64,
        case_sensitive: bool,
        fast_mode: bool,
        vanity_mode: VanityMode
    ) -> Result<KeysAndAddress, CustomError> {
        if string.is_empty() { return Ok( KeysAndAddress::generate_random()) }
        if string.len() > 4 && fast_mode {
            return Err(CustomError("You're asking for too much!\n\
            If you know this will take for a long time and really want to find something longer than 4 characters\n\
             disable fast mode with -df or --disable_fas flags."))
        }

        let is_base58 = string
            .chars()
            .find(|c| c == &'0' || c == &'I' || c == &'O' || c == &'l');

        if is_base58.is_some() {
            return Err(CustomError("Your input is not in base58. Don't include zero: '0', uppercase i: 'I', uppercase o: 'O', lowercase L: 'l', in your input!"))
        }

        Ok(find_vanity_address(string, threads, case_sensitive, vanity_mode))
    }
}

fn find_vanity_address(string: &str, threads: u64, case_sensitive: bool, vanity_mode: VanityMode) -> KeysAndAddress {
    let string_len = string.len();
    let (sender, receiver) = mpsc::channel();

    for _ in 0..threads {
        let sender = sender.clone();
        let string = string.to_string();
        let mut anywhere_flag = false;
        let mut prefix_suffix_flag = false;

        let _ = thread::spawn(move || {
            loop {
                let new_pair = KeysAndAddress::generate_random();
                let address = new_pair.get_comp_address();

                match vanity_mode {
                    VanityMode::Prefix => {
                        let slice = &address[1..=string_len];
                        prefix_suffix_flag = match case_sensitive {
                            true => slice == string,
                            false => slice.to_lowercase() == string.to_lowercase(),
                        };
                    }
                    VanityMode::Suffix => {
                        let address_len = address.len();
                        let slice = &address[address_len - string_len..address_len];
                        prefix_suffix_flag = match case_sensitive {
                            true => slice == string,
                            false => slice.to_lowercase() == string.to_lowercase(),
                        };
                    }
                    VanityMode::Anywhere => {
                        anywhere_flag = match case_sensitive {
                            true => address.contains(&string),
                            false => address.to_lowercase().contains(&string.to_lowercase()),
                        };
                    }
                }
                if prefix_suffix_flag || anywhere_flag {if sender.send(new_pair).is_err() {return}} // If the channel closed, that means another thread found a keypair and closed it so we just return and kill the thread if an error occurs.
            }
        });
    }

    loop {
        match receiver.try_recv() {
            Ok(pair) => return pair,
            Err(_) => continue
        }
    }
}