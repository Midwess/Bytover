import {
    Tabs,
    TabsPanel,
    TabsPanels,
    TabsList,
    TabsTab,
} from '@/components/animate-ui/components/base/tabs';
import { Button } from '@/components/ui/button';
import {
    Card,
    CardContent,
    CardDescription,
    CardFooter,
    CardHeader,
    CardTitle,
} from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {Mail, MapPin, SendHorizonal} from "lucide-react";

export function Transfer() {
    return (
        <div className="flex w-full max-w-sm flex-col gap-6">
            <Tabs defaultValue="account" className="w-full items-start">
                <TabsList className={"ml-2 border shadow-background shadow-sm"}>
                    <TabsTab value="nearby">People</TabsTab>
                    <TabsTab value="public">Public</TabsTab>
                    <TabsTab value="devices">Devices</TabsTab>
                </TabsList>
                <Card className="px-2 border-none bg-transparent relative">
                    <TabsPanels>
                        <TabsPanel value="nearby" className="flex flex-col">
                            <CardContent className={"p-0 flex flex-col gap-2"}>
                                <Card className="flex flex-col gap-2 py-2 p-1.5 bg-card/80 rounded-lg">
                                    <Label htmlFor="tabs-input-email" className={"flex flex-row items-center gap-1 bg-muted px-2 py-1 w-fit rounded-md"}>
                                        <div className={"bg-white/10 p-[3px] rounded-sm w-5 h-5 flex items-center justify-center"}>
                                            <Mail/>
                                        </div>
                                         Emails:
                                    </Label>
                                    <div className={"flex flex-row gap-2 h-fit"}>
                                        <Input className={"bg-secondary shadow-background"} id="tabs-input-email" type={"email"} defaultValue="someone@company.com"/>
                                        <Button variant={"default"} className={"w-[32px] bg-bluePrimary/80"}>
                                            <SendHorizonal color={"white"}/>
                                        </Button>
                                    </div>
                                </Card>
                                <Card className="flex flex-col gap-2 py-2 border p-1.5">
                                    <Label className={"flex flex-row items-center gap-2 bg-muted px-2 py-1 w-fit rounded-md shadow-black"}>
                                        <div className={"bg-white/10 p-[3px] rounded-sm w-5 h-5 flex items-center justify-center"}>
                                            <MapPin/>
                                        </div>
                                        Nearby:
                                    </Label>
                                    <div className={"flex flex-row gap-2"}>
                                    </div>
                                </Card>
                            </CardContent>
                        </TabsPanel>
                        <TabsPanel value="public" className="flex flex-col gap-6">
                            <CardHeader>
                                <CardTitle>Public</CardTitle>
                                <CardDescription>
                                    Public your resources with protected password.
                                </CardDescription>
                            </CardHeader>
                            <CardContent className="grid gap-6">
                                <div className="grid gap-3">
                                    <Label htmlFor="tabs-demo-current">Password</Label>
                                    <Input id="tabs-demo-current" type="password"/>
                                </div>
                           </CardContent>
                            <CardFooter>
                                <Button>Send</Button>
                            </CardFooter>
                        </TabsPanel>
                        <TabsPanel value="public" className="flex flex-col gap-6">
                            <CardHeader>
                                <CardTitle>Devices</CardTitle>
                                <CardDescription>
                                    Sharing to your devices.
                                </CardDescription>
                            </CardHeader>
                            <CardContent className="grid gap-6">
                                <div className="grid gap-3">
                                    <Label htmlFor="tabs-demo-current">Password</Label>
                                    <Input id="tabs-demo-current" type="password" />
                                </div>
                            </CardContent>
                            <CardFooter>
                                <Button>Send</Button>
                            </CardFooter>
                        </TabsPanel>
                    </TabsPanels>
                </Card>
            </Tabs>
        </div>
    );
}