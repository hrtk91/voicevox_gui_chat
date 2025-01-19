// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

use std::{env, sync::Arc};

use tauri::{async_runtime::Mutex, AppHandle};
pub struct AppState {
    client: Arc<reqwest::Client>,
    chat_completion: voicevox_chat::openai::ChatCompletion,
    sender: mpsc::Sender<Vec<u8>>,
}

#[tauri::command]
fn exit(app: AppHandle) {
    app.exit(0);
}

#[tauri::command]
async fn on_send_message(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    value: String,
) -> Result<String, String> {
    let mut state = state.lock().await;

    state.chat_completion.push_user_message(&value);

    let reply = match state.chat_completion.completion().await {
        Ok(reply) => reply,
        Err(e) => {
            eprintln!("Failed to get completion: {:?}", e);
            Err("Failed to get completion".to_string())?
        }
    };

    let wav = voicevox_chat::audio::generate_wav(
        state.client.clone(),
        &reply,
        voicevox_chat::audio::Speakers::Zundamon,
    )
    .await;

    if let Ok(wav) = wav {
        if let Err(err) = state.sender.send(wav).await {
            eprintln!("Failed to send wav: {:?}", err);
        }
    } else {
        eprintln!("Failed to generate wav");
    }

    Ok(reply)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    dotenvy::dotenv().map_err(|err| eprintln!("dotenvy error: {}", err)).ok();

    let (sender, receiver) = mpsc::channel(1);
    let client = Arc::new(reqwest::Client::new());

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![on_send_message, exit])
        .manage(Arc::new(Mutex::new(AppState {
            client: client.clone(),
            chat_completion: voicevox_chat::openai::ChatCompletion::new(
                env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY is not set"),
                client.clone(),
            ),
            sender
        })))
        .setup(|_| {
            tauri::async_runtime::spawn(async move {
                async_process(receiver).await
            });
            
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

use tokio::sync::mpsc;

async fn async_process(
    mut receiver: mpsc::Receiver<Vec<u8>>,
    // mut receiver: mpsc::Receiver<Vec<u8>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (_stream, stream_handle) = rodio::OutputStream::try_default().expect("Failed to get default output stream");
    let sink = rodio::Sink::try_new(&stream_handle).expect("Failed to create sink");
    loop {
        if let Ok(bytes) = receiver.try_recv() {
            let cursor = std::io::Cursor::new(bytes);
            let source = rodio::Decoder::new(cursor).expect("Failed to create decoder");
        
            sink.append(source);
        };
    }
}