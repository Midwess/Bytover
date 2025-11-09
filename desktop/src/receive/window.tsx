import ReactDOM from "react-dom/client";
import React from "react";
import {Card} from "@/components/ui/card.tsx";
ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
        <Window />
    </React.StrictMode>,
);

function Window() {
    return <>
        <Card className={"w-screen h-screen dark container"}>
            <p>This is the receive windows, I will be the winner</p>
        </Card>
    </>
}
