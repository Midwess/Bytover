import ReactDOM from "react-dom/client";
import React from "react";
import { Shelf } from "./shelf";
import { Transfer } from "./transfer.tsx";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
        <Window />
    </React.StrictMode>,
);

function Window() {
  return (
    <main className="w-screen h-screen overflow-hidden p-2 dark bg-transparent">
        <div className={"w-full h-full flex flex-row rounded-2xl bg-transparent space-x-1"}>
            <div className={"w-full h-full bg-transparent"}>
                <Shelf/>
            </div>
            <div className={"w-full h-full bg-transparent"}>
                <Transfer/>
            </div>
        </div>
    </main>
  )
}

export default Window;
