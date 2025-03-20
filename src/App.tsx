import {
  XMarkIcon,
  PlayIcon,
  PauseIcon,
  StopIcon
} from "@heroicons/react/24/outline";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useState, useEffect } from "react";
import { Button } from "./components/ui/button";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuTrigger
} from "./components/ui/context-menu";
import "./tailwind.css";

function App() {
  const [value, setValue] = useState("");
  const [reply, setReply] = useState("");
  const [isPlaying, setIsPlaying] = useState(false);

  useEffect(() => {
    const unlisten = listen("audio-playback-completed", () => {
      setIsPlaying(false);
    });

    return () => {
      unlisten.then((f) => f()).catch(console.error);
    };
  }, []);

  const handleSendMessage = async () => {
    if (!value) return;
    setValue("");
    const reply = await invoke<string>("on_send_message", {
      value
    });
    setReply(reply);
    setIsPlaying(true);
  };

  return (
    <>
      <main className="h-dvh w-dvw flex flex-col bg-black bg-opacity-50 backdrop-blur">
        <ContextMenu>
          <ContextMenuTrigger className="relative overflow-hidden w-full grow cursor-grab active:cursor-grabbing">
            <div className="flex items-center gap-2 select-none h-full w-full px-2 pt-2">
              <img
                src="/public/zunda_smile_001.png"
                alt="zunda"
                className="max-w-24"
              />
              <div className="h-full flex flex-col z-20 grow cursor-auto">
                <div className="overflow-y-auto">
                  <p className="text-base text-white">{reply}</p>
                </div>
                {reply && (
                  <div className="flex items-center gap-2">
                    <Button
                      variant="ghost"
                      size="icon"
                      className="hover:bg-transparent"
                      onClick={async () => {
                        try {
                          if (isPlaying) {
                            await invoke("pause_audio");
                            setIsPlaying(false);
                          } else {
                            await invoke("resume_audio");
                            setIsPlaying(true);
                          }
                        } catch (error) {
                          console.error("Audio control error:", error);
                        }
                      }}
                    >
                      {isPlaying ? (
                        <PauseIcon className="size-10 text-gray-300" />
                      ) : (
                        <PlayIcon className="size-7 text-gray-300" />
                      )}
                    </Button>
                    {isPlaying && (
                      <Button
                        variant="ghost"
                        size="icon"
                        className="hover:bg-transparent"
                        onClick={async () => {
                          try {
                            await invoke("stop_audio");
                            setIsPlaying(false);
                          } catch (error) {
                            console.error("Audio stop error:", error);
                          }
                        }}
                      >
                        <StopIcon className="size-5 text-red-500" />
                      </Button>
                    )}
                  </div>
                )}
              </div>
              <button
                className="text-white px-2 self-start z-20"
                type="button"
                onClick={() => {
                  invoke("exit");
                }}
              >
                <XMarkIcon className="size-5" />
              </button>
              <div
                className="absolute left-0 right-0 h-full w-full z-10"
                data-tauri-drag-region
              />
            </div>
          </ContextMenuTrigger>
          <ContextMenuContent>
            <ContextMenuItem>Profile</ContextMenuItem>
            <ContextMenuItem>Billing</ContextMenuItem>
            <ContextMenuItem>Team</ContextMenuItem>
            <ContextMenuItem>Subscription</ContextMenuItem>
          </ContextMenuContent>
        </ContextMenu>
        <form
          className="flex gap-2 px-4 shrink-0"
          onSubmit={(ev) => ev.preventDefault()}
        >
          <textarea
            className="grow p-2 outline-none"
            rows={2}
            onChange={(ev) => {
              setValue(ev.target.value);
            }}
            value={value}
            onKeyUp={async (ev) => {
              // Ctrl + Enterで送信
              if (ev.key === "Enter") {
                if (ev.ctrlKey) {
                  await handleSendMessage();
                  return;
                }
              }
            }}
          />
          <Button
            onClick={async () => {
              await handleSendMessage();
            }}
          >
            送信
          </Button>
        </form>
      </main>
    </>
  );
}

export default App;
