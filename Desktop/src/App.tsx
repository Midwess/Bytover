import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";
import { listen } from "@tauri-apps/api/event";

type CounterState = {
  count: 0
}

const initialState: CounterState = { count: 0 };

function App() {
  const [state, setState] = useState<CounterState>(initialState);

  async function increment() {
    await invoke("increment");
  }

  async function decrement() {
    await invoke("decrement");
  }

  useEffect(() => {
    let unlisten: () => void;
    listen<CounterState>("counter_state", (event) => {
      setState(event.payload);
    }).then((listener) => {
      unlisten = listener;
    });

    return () => {
      unlisten?.();
    };
  }, []);

  return (
    <main className="container">
      <div className="flex flex-col items-center justify-center h-screen gap-10">
        <p>Click on increment or decrement to change the counter, the counter is maintained in surrealdb</p>
        <p>Current count: {state.count}</p>
        <div className="flex flex-row justify-between gap-4">
          <button className="bg-blue-500 text-white px-4 py-2 rounded-md hover:bg-blue-600 cursor-pointer" onClick={increment}>Increment</button>
          <button className="bg-red-500 text-white px-4 py-2 rounded-md hover:bg-red-600 cursor-pointer" onClick={decrement}>Decrement</button>
        </div>
      </div>
    </main>
  );
}

export default App;
