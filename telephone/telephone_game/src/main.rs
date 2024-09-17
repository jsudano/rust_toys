use std::io::stdin;

use modifiers::{get_random_garbler, Message};

fn clear_terminal() {
    // prints the ascii control character to clear the screen
    print!("{}[2J", 27 as char);
}

// runs a round of telephone!
fn telephone_game() -> Result<(), anyhow::Error> {
    // ask the user for a number of "callers" (garblers)
    println!("Number of callers? (enter a number)");
    let mut num_callers_str = String::new();
    stdin().read_line(&mut num_callers_str)?;
    let num_callers = num_callers_str.trim().parse::<i32>()?;

    // ask the user for a message to be garbled
    println!("Enter your message! (whitespaced ascii only please)");
    let mut message = String::new();
    stdin().read_line(&mut message)?;

    // do the garbling
    let random_garblers = (0..num_callers).map(|_| get_random_garbler());
    let mut garbled_message: Message = message.into();
    for garbler in random_garblers {
        garbled_message = garbler.garble(garbled_message);
    }

    // give the user their work of art
    println!("Your message is:");
    println!("{garbled_message}");

    // return so the loop knows we succeeded
    Ok(())
}

fn main() {
    clear_terminal();
    println!("Welcome to the telephone game!");
    loop {
        if let Err(e) = telephone_game() {
            // game failed for some reason, print the error then reset
            println!("Failed to garble with error: {e}");
            clear_terminal();
            continue;
        }

        // game succeeded, pause for the user see their message and prompt for reset
        println!("continue? (y)/n");
        let mut continue_response = String::new();
        if stdin().read_line(&mut continue_response).is_err() {
            // failed to parse, just give up
            break;
        };

        match continue_response.as_str() {
            "y\n" | "\n" => {
                clear_terminal();
                continue;
            }
            _ => break,
        }
    }
}
