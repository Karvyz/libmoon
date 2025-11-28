use std::{thread::sleep, time::Duration};

use libmoon::chat::Chat;

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Error)
        .filter_module("libmoon", log::LevelFilter::Trace)
        .filter_module("llm", log::LevelFilter::Trace)
        .init();

    let mut chat = Chat::load();
    println!("{:?}", chat.get_history());
    chat.add_user_message("Count to 3".to_string());
    sleep(Duration::from_secs(10));
    println!("{:?}", chat.get_history());
    chat.next(0);
    println!("{:?}", chat.get_history());
    chat.next(0);
    sleep(Duration::from_secs(10));
    println!("{:?}", chat.get_history());
}
