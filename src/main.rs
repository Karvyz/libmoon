use libmoon::{
    chat::ChatUpdate,
    gateway::GatewayUpdate,
    moon::{Moon, MoonUpdate},
};

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Error)
        .filter_module("libmoon", log::LevelFilter::Trace)
        .filter_module("llm", log::LevelFilter::Trace)
        .init();

    let mut moon = Moon::new();
    println!("{:?}\n\n", moon.chat.get_history());
    moon.chat.add_user_message("Count to 3".to_string());
    handle(&mut moon).await;
    println!("{:?}\n\n", moon.chat.get_history());

    moon.chat.next(1);
    handle(&mut moon).await;
    println!("{:?}\n\n", moon.chat.get_history());

    // moon.chat.previous(0);
    // moon.chat.previous(0);
    // moon.chat.previous(0);
    // moon.chat.add_edit(1, "This is an user edit.".to_string());
    // handle(&mut moon).await;
    // println!("{:?}\n\n", moon.chat.get_history());
    //
    // moon.chat.add_edit(0, "This is a char edit.".to_string());
    // println!("{:?}\n\n", moon.chat.get_history());
}

async fn handle(moon: &mut Moon) {
    loop {
        match moon.recv().await {
            MoonUpdate::CU(u) => match u {
                ChatUpdate::RequestSent => println!("Request Sent"),
                ChatUpdate::RequestOk => println!("Request Ok"),
                ChatUpdate::RequestError(e) => {
                    println!("Error: {e}");
                    return;
                }
                ChatUpdate::StreamUpdate => println!("StreamUpdate "),
                ChatUpdate::StreamFinished => {
                    println!("StreamFinished");
                    return;
                }
            },
            MoonUpdate::GU(u) => match u {
                GatewayUpdate::Char => println!("Char loaded"),
                GatewayUpdate::User => println!("User loaded"),
            },
            MoonUpdate::Error(e) => println!("Error: {e}"),
        }
    }
}
