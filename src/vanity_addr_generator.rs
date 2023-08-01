use crate::keys_and_address::KeysAndAddress;
use crate::error::CustomError;
use std::thread;
use std::sync::mpsc;
use std::sync::mpsc::TryRecvError;

pub struct VanityAddr;

impl VanityAddr {
    pub fn generate(
        string: String,
        threads: u64,
        case_sensitive: bool,
        fast_mode: bool
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

        Ok(find_vanity_address(string, threads, case_sensitive))
    }
}

fn find_vanity_address(string: String, threads: u64, case_sensitive: bool) -> KeysAndAddress {

    let string_len = string.len();

    let (sender, receiver) = mpsc::channel();

    for _ in 0..threads {
        let sender = sender.clone();
        let string = string.clone();
        let _ = thread::spawn(move || {
            loop {
                let new_pair = KeysAndAddress::generate_random();
                let address = new_pair.get_comp_address();
                let slice = &address[1..= string_len];

                if case_sensitive {
                    if slice == string { sender.send(new_pair).unwrap()  };
                } else {
                    todo!() //TODO: implement case_sensitive: false. For now it just sends a random pair.
                }
            }
        });
    }

    loop {
        match receiver.try_recv() {
            Ok(pair) => return pair,
            Err(err) => match err {
                TryRecvError::Empty => continue,
                _ => panic!("{}",err),
            }
        }
    }
}