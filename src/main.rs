use libmoon::chat::{Chat, ChatUpdate};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Error)
        .filter_module("libmoon", log::LevelFilter::Trace)
        .filter_module("llm", log::LevelFilter::Trace)
        .init();

    let mut chat = Chat::load();
    let mut rx = chat.get_rx();

    println!("{:?}\n\n", chat.get_history());
    chat.add_user_message("Count to 3".to_string());
    handle(&mut rx).await;
    println!("{:?}\n\n", chat.get_history());

    chat.next(0);
    println!("{:?}\n\n", chat.get_history());

    chat.next(0);
    handle(&mut rx).await;
    println!("{:?}\n\n", chat.get_history());

    chat.previous(0);
    chat.previous(0);
    chat.previous(0);
    chat.add_edit(1, "This is an user edit.".to_string());
    handle(&mut rx).await;
    println!("{:?}\n\n", chat.get_history());

    chat.add_edit(0, "This is a char edit.".to_string());
    println!("{:?}\n\n", chat.get_history());
}

async fn handle(rx: &mut mpsc::Receiver<ChatUpdate>) {
    loop {
        match rx.recv().await {
            Some(u) => match u {
                ChatUpdate::MessageCreated => println!("MessageCreated"),
                ChatUpdate::StreamUpdate => println!("StreamUpdate "),
                ChatUpdate::StreamFinished => {
                    println!("StreamFinished");
                    return;
                }
                ChatUpdate::Error(e) => println!("Error: {e}"),
            },
            None => return,
        }
    }
}
