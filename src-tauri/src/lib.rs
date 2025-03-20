// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

use std::{
    env,
    sync::{Arc, Mutex as StdMutex},
    time::Duration,
};

use tauri::{async_runtime::Mutex, AppHandle, Emitter, Manager};
pub struct AppState {
    client: Arc<reqwest::Client>,
    chat_completion: voicevox_chat::openai::ChatCompletion,
    sender: mpsc::Sender<Vec<u8>>,
    audio_sink: Arc<StdMutex<Option<rodio::Sink>>>,
    last_audio_data: Arc<StdMutex<Option<Vec<u8>>>>,
    audio_state_sender: mpsc::Sender<AudioState>,
}

#[derive(Clone, Debug)]
enum AudioState {
    Playing,
    Stopped,
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
        // Store the last audio data
        if let Ok(mut last_audio_data) = state.last_audio_data.try_lock() {
            *last_audio_data = Some(wav.clone());
        }

        if let Err(err) = state.sender.send(wav).await {
            eprintln!("Failed to send wav: {:?}", err);
        }

        // Signal that audio is now playing
        if let Err(err) = state.audio_state_sender.send(AudioState::Playing).await {
            eprintln!("Failed to send audio state: {:?}", err);
        }
    } else {
        eprintln!("Failed to generate wav");
    }

    Ok(reply)
}

#[tauri::command]
fn pause_audio(state: tauri::State<'_, Arc<Mutex<AppState>>>) -> Result<(), String> {
    if let Ok(sink_arc) = state.try_lock() {
        if let Ok(sink_guard) = sink_arc.audio_sink.try_lock() {
            if let Some(sink) = sink_guard.as_ref() {
                sink.pause();
                Ok(())
            } else {
                Err("No audio is currently playing".to_string())
            }
        } else {
            Err("No audio is currently playing".to_string())
        }
    } else {
        Err("No audio is currently playing".to_string())
    }
}

#[tauri::command]
fn resume_audio(state: tauri::State<'_, Arc<Mutex<AppState>>>) -> Result<(), String> {
    if let Ok(sink_arc) = state.try_lock() {
        if let Ok(sink_guard) = sink_arc.audio_sink.try_lock() {
            if let Some(sink) = sink_guard.as_ref() {
                sink.play();
                Ok(())
            } else {
                Err("No audio is currently playing".to_string())
            }
        } else {
            Err("No audio is currently playing".to_string())
        }
    } else {
        Err("No audio is currently paused".to_string())
    }
}

#[tauri::command]
fn stop_audio(state: tauri::State<'_, Arc<Mutex<AppState>>>) -> Result<(), String> {
    if let Ok(sink_arc) = state.try_lock() {
        if let Ok(sink_guard) = sink_arc.audio_sink.try_lock() {
            if let Some(sink) = sink_guard.as_ref() {
                sink.stop();
                Ok(())
            } else {
                Err("No audio is currently playing".to_string())
            }
        } else {
            Err("No audio is currently playing".to_string())
        }
    } else {
        Err("No audio is currently playing".to_string())
    }
}

#[tauri::command]
fn replay_last_audio(state: tauri::State<'_, Arc<Mutex<AppState>>>) -> Result<(), String> {
    if let Ok(state) = state.try_lock() {
        if let Ok(last_audio) = state.last_audio_data.try_lock() {
            if let Some(last_audio) = last_audio.as_ref() {
                if let Err(err) = state.sender.try_send(last_audio.clone()) {
                    eprintln!("Failed to send wav: {:?}", err);
                    Err("Failed to replay audio".to_string())
                } else {
                    Ok(())
                }
            } else {
                Ok(())
            }
        } else {
            Err("No previous audio data available".to_string())
        }
    } else {
        Err("No previous audio data available".to_string())
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    dotenvy::dotenv()
        .map_err(|err| eprintln!("dotenvy error: {}", err))
        .ok();

    let (sender, receiver) = mpsc::channel(1);
    let (audio_state_sender, audio_state_receiver) = mpsc::channel(1);
    let client = Arc::new(reqwest::Client::new());
    let audio_sink = Arc::new(StdMutex::new(None));
    let last_audio_data = Arc::new(StdMutex::new(None));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            on_send_message,
            exit,
            pause_audio,
            resume_audio,
            stop_audio,
            replay_last_audio
        ])
        .manage(Arc::new(Mutex::new(AppState {
            client: client.clone(),
            chat_completion: voicevox_chat::openai::ChatCompletion::new(
                env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY is not set"),
                client.clone(),
            ),
            sender: sender.clone(),
            audio_sink: audio_sink.clone(),
            last_audio_data: last_audio_data.clone(),
            audio_state_sender: audio_state_sender.clone(),
        })))
        .setup(|app| {
            let app_handle = app.app_handle().clone();
            let audio_sink_clone = audio_sink.clone();
            tauri::async_runtime::spawn(async move {
                audio_state_monitor(audio_state_receiver, audio_sink_clone, app_handle).await
            });

            tauri::async_runtime::spawn(async move { async_process(receiver, audio_sink).await });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

use tokio::sync::mpsc;

async fn audio_state_monitor(
    mut audio_state_receiver: mpsc::Receiver<AudioState>,
    audio_sink_state: Arc<StdMutex<Option<rodio::Sink>>>,
    app_handle: tauri::AppHandle,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut current_state = AudioState::Stopped;

    loop {
        tokio::select! {
            Some(new_state) = audio_state_receiver.recv() => {
                current_state = new_state;
            }
            _ = tokio::time::sleep(Duration::from_millis(500)) => {
                // Periodically check the current audio state
                if let Ok(sink_guard) = audio_sink_state.try_lock() {
                    if let Some(sink) = sink_guard.as_ref() {
                        // Check if the sink is empty (playback completed)
                        if sink.empty() && matches!(current_state, AudioState::Playing) {
                            app_handle.emit("audio-playback-completed", {}).unwrap_or_default();
                            current_state = AudioState::Stopped;
                        }
                    }
                }
            }
        }
    }
}

async fn async_process(
    mut receiver: mpsc::Receiver<Vec<u8>>,
    audio_sink_state: Arc<StdMutex<Option<rodio::Sink>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (_stream, stream_handle) =
        rodio::OutputStream::try_default().expect("Failed to get default output stream");

    // Create a single sink that will be reused
    let sink = rodio::Sink::try_new(&stream_handle).expect("Failed to create sink");

    // Store the initial sink in the shared state
    *audio_sink_state.lock().unwrap() = Some(sink);

    loop {
        if let Ok(bytes) = receiver.try_recv() {
            // Lock the sink state
            let Ok(mut sink_guard) = audio_sink_state.try_lock() else {
                continue;
            };

            if let Some(current_sink) = sink_guard.as_mut() {
                // Clear the current sink
                current_sink.stop();
                current_sink.skip_one();

                // Create a new source from the received bytes
                let cursor = std::io::Cursor::new(bytes);
                let source = rodio::Decoder::new(cursor).expect("Failed to create decoder");

                // Append the new source
                current_sink.append(source);
            }
        }
    }
}
