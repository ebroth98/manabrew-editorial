import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable";
import { CardSearch } from "@/components/editor/CardSearch";
import { DeckBuilder } from "@/components/editor/DeckBuilder";

export default function DeckEditor() {
  return (
    <div className="h-full w-full overflow-hidden">
      <ResizablePanelGroup orientation="vertical">
        <ResizablePanel defaultSize={50} minSize={30}>
          <div className="h-full flex flex-col">
            <div className="p-2 bg-muted/20 border-b">
              <h2 className="text-sm font-semibold">Card Search</h2>
            </div>
            <CardSearch />
          </div>
        </ResizablePanel>
        
        <ResizableHandle withHandle />
        
        <ResizablePanel defaultSize={50} minSize={30}>
           <div className="h-full flex flex-col">
            <div className="p-2 bg-muted/20 border-b">
              <h2 className="text-sm font-semibold">Current Deck</h2>
            </div>
            <DeckBuilder />
          </div>
        </ResizablePanel>
      </ResizablePanelGroup>
    </div>
  );
}
