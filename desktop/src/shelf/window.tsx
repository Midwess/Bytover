import ReactDOM from "react-dom/client";
import React from "react";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
        <Window />
    </React.StrictMode>,
);

function Window() {
  return (
    <main className="w-screen h-screen overflow-hidden p-4 dark">
      <div className={"w-full h-full rounded-2xl bg-card border-[1px] border-border shadow-md shadow-background"}>

      </div>
    </main>
  );
}

export default Window;
