import { useState } from "react";
import "./tailwind.css";
import { invoke } from "@tauri-apps/api/core";
import { XMarkIcon } from "@heroicons/react/24/outline";

function App() {
  const [value, setValue] = useState("");
  const [reply, setReply] = useState("");

  const handleSendMessage = async () => {
    if (!value) return;
    setValue("");
    const reply = await invoke<string>("on_send_message", {
      value
    });
    setReply(reply);
  };
  return (
    <>
      <main className="h-dvh w-dvw flex flex-col bg-black bg-opacity-50">
        <div className="relative w-full grow overflow-y-hidden cursor-grab active:cursor-grabbing">
          <div className="flex items-center gap-2 select-none h-full w-full p-2">
            <img
              src="/public/zunda_smile_001.png"
              alt="zunda"
              className="max-w-24"
            />
            <div className="overflow-y-auto h-full grow z-20 cursor-auto">
              <p className="text-base text-white">{reply}</p>
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
        </div>
        <form
          className="flex gap-2 px-4 py-2 shrink-0"
          onSubmit={(ev) => ev.preventDefault()}
        >
          <input
            type="text"
            className="grow p-2 outline-none"
            onChange={(ev) => {
              setValue(ev.target.value);
            }}
            value={value}
            onKeyUp={async (ev) => {
              // Ctrl + Enterで送信
              if (ev.ctrlKey && ev.key === "Enter") {
                await handleSendMessage();
              }
            }}
          />
          <button
            type="button"
            className="text-white px-2 border rounded"
            onClick={async () => {
              await handleSendMessage();
            }}
          >
            送信
          </button>
        </form>
      </main>
    </>
  );
}

export default App;
