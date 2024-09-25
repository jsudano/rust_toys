use rand::{self, Rng};
use std::fmt::{Display, Write};

/// A container used to track a collection of words that makes up a `Message` to be garbled
/// Note: we derive `Default` here which gives us `Message::default()` without having to impl the trait
///       we derive `PartialEq` here so we can compare `Message`s to each other
///       we derive `Debug` here so we can debug-print `Message`s
#[derive(Default, PartialEq, Debug)]
pub struct Message {
    words: Vec<String>,
}

/// Tells you how to print a `Message`
impl Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for word in &self.words {
            f.write_str(word)?;
            f.write_char(' ')?;
        }
        f.write_char('!')?;
        Ok(())
    }
}

/// Converts a `String`, presumably containing whitespace, into a `Message` of words
impl From<String> for Message {
    fn from(value: String) -> Self {
        let words = value
            .split_ascii_whitespace()
            .map(|s: &str| s.to_owned())
            .collect();
        Self { words }
    }
}

/// Converts a `&str`, presumably containing whitespace, into a `Message` of words
impl From<&str> for Message {
    fn from(value: &str) -> Self {
        let words = value
            .split_ascii_whitespace()
            .map(|s: &str| s.to_owned())
            .collect();
        Self { words }
    }
}

/// The "template" for a thing which garbles
pub trait Garbler {
    fn garble(&self, message: Message) -> Message;
}

/// Passes on the message with no changes
pub struct GoodListener;

impl Garbler for GoodListener {
    fn garble(&self, message: Message) -> Message {
        // good listeners pass on the message as heard
        message
    }
}

/// Removes every third word in a `Message`
/// Note that this is an empty struct, simply because we can't impl a trait without an assocaited struct
pub struct ThirdRemover;

impl Garbler for ThirdRemover {
    fn garble(&self, message: Message) -> Message {
        let new_words = message
            .words
            .iter()
            .enumerate()
            .filter_map(|(i, word)| if i % 3 == 2 { None } else { Some(word.clone()) })
            .collect();

        // note that this consumes `message` and returns a new `Message` instead of mutating it, this doesn't leak
        // memory as `message` will be dropped at the end of the expression since it is "owned" by this `garble` fn
        Message { words: new_words }
    }
}

/// Pair-wise swaps words in a `Message`
pub struct PairSwapper;

impl Garbler for PairSwapper {
    fn garble(&self, mut message: Message) -> Message {
        for i in (0..message.words.len()).step_by(2) {
            if i + 1 == message.words.len() {
                break;
            }
            message.words.swap(i, i + 1);
        }

        // note that this mutates message and returns it, instead of consuming it
        message
    }
}

/// Replaces the fifth word from the current `Message` with the fifth word from the previously parsed message.
/// If the previous message didn't have a fifth word, it replaces the fifth word of the current message with "Wumpus".
/// If the current message doesn't have a fifth word, add "Wumpus" to the end.
#[allow(dead_code)] // Note: allowed because nothing currently uses `last_word`, you should remove this!
pub struct TemporalDisplacer {
    last_word: String,
}

impl Default for TemporalDisplacer {
    fn default() -> Self {
        Self {
            last_word: String::from("Wumpus"),
        }
    }
}

impl Garbler for TemporalDisplacer {
    fn garble(&self, _message: Message) -> Message {
        // TODO: implement the garbling!
        Message::default()
    }
}

/// Returns a random one of our three garblers
/// Note, the return type must be `Box`ed as return types must be concrete (not traits)
pub fn get_random_garbler() -> Box<dyn Garbler> {
    match rand::thread_rng().gen_range(0..4) {
        0 => Box::new(GoodListener),
        1 => Box::new(ThirdRemover),
        _ => Box::new(PairSwapper),
        // TODO once you have implemented Garbler for TemporalDisplacer, uncomment this so you can use it in the game!
        // _ => Box::new(TemporalDisplacer::default()),
    }
}

// Unit tests live here!
#[cfg(test)]
mod tests {
    use crate::{Garbler, Message, PairSwapper, TemporalDisplacer, ThirdRemover};

    #[test]
    fn test_remove_third() {
        let rt = ThirdRemover;

        let msg = Message::default();

        assert_eq!(rt.garble(msg).to_string(), String::from("!"));

        let msg = Message::from("one two three");

        assert_eq!(rt.garble(msg).to_string(), String::from("one two !"));

        let msg = Message::from("un deux trois quatre cinq six");

        assert_eq!(
            rt.garble(msg).to_string(),
            String::from("un deux quatre cinq !")
        );
    }

    #[test]
    fn test_pair_swapper() {
        let ps = PairSwapper;

        let msg = Message::default();

        assert_eq!(ps.garble(msg).to_string(), String::from("!"));

        let msg = Message::from("one two three");

        assert_eq!(ps.garble(msg).to_string(), String::from("two one three !"));

        let msg = Message::from("un deux trois quatre cinq six");

        assert_eq!(
            ps.garble(msg).to_string(),
            String::from("deux un quatre trois six cinq !")
        );
    }

    #[test]
    fn test_temporal_displacer() {
        // TODO this test will fail because `Garbler` is not properly implemented for `TemporalDisplacer!`
        let td = TemporalDisplacer::default();

        let msg = Message::default();

        // empty messages don't have a fifth word, append Wumpus
        assert_eq!(td.garble(msg).to_string(), String::from("Wumpus !"));

        let msg = Message::from("one two three");

        // this message didn't have a fifth word, append Wumpus
        assert_eq!(
            td.garble(msg).to_string(),
            String::from("one two three Wumpus !")
        );

        let msg = Message::from("un deux trois quatre cinq six");

        // this message has a fifth word, replace Wumpus
        assert_eq!(
            td.garble(msg).to_string(),
            String::from("un deux trois quatre Wumpus six !")
        );

        let msg = Message::from("uno dos tres cuatro cinco seis");

        // "cinq" was the fifth word of the last message, insert it
        assert_eq!(
            td.garble(msg).to_string(),
            String::from("uno dos tres cuatro cinq seis !")
        );

        let msg = Message::from("eins zwei drei vier funf");

        // "cinco" was the fifth word of the last message, insert it
        assert_eq!(
            td.garble(msg).to_string(),
            String::from("eins zwei drei vier cinco !")
        );

        let msg = Message::from("uno due tre quattro");

        // this message doesn't have a fifth word, append Wumpus
        assert_eq!(
            td.garble(msg).to_string(),
            String::from("uno due tre quattro Wumpus !")
        );
    }
}
