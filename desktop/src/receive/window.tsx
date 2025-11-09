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
        <Card className={"w-screen h-screen dark container rounded-3xl"}>
        </Card>
    </>
}
