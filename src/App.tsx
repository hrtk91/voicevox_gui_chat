import "./tailwind.css";

function App() {
  return (
    <main className="h-dvh w-dvw backdrop-blur" data-tauri-drag-region>
      <div className="flex items-center">
        <img
          src="/public/zunda_smile_001.png"
          alt="zunda"
          className="max-w-24"
        />
        <p className="text-3xl text-white">あいうえお</p>
      </div>
    </main>
  );
}

export default App;
