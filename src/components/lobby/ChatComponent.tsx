import { useState, useRef, useEffect } from "react";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Send } from "lucide-react";
import { cn } from "@/lib/utils";

interface Message {
  id: string;
  sender: string;
  content: string;
  timestamp: Date;
  type: 'system' | 'user' | 'whisper';
}

interface ChatComponentProps {
  channelId: string;
}

export function ChatComponent({ channelId }: ChatComponentProps) {
  const [messages, setMessages] = useState<Message[]>([
    { id: '1', sender: 'System', content: 'Welcome to XMage!', timestamp: new Date(), type: 'system' },
  ]);
  const [input, setInput] = useState('');
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    // Scroll to bottom
    if (scrollRef.current) {
      scrollRef.current.scrollIntoView({ behavior: 'smooth' });
    }
  }, [messages]);

  const handleSend = () => {
    if (!input.trim()) return;
    const newMessage: Message = {
      id: Date.now().toString(),
      sender: 'You',
      content: input,
      timestamp: new Date(),
      type: 'user',
    };
    setMessages([...messages, newMessage]);
    setInput('');
  };

  return (
    <div className="flex flex-col h-full border-t">
      <div className="p-2 border-b bg-muted/20">
        <span className="text-xs font-semibold text-muted-foreground">Chat - {channelId}</span>
      </div>
      <ScrollArea className="flex-1 p-4">
        <div className="space-y-4">
          {messages.map((msg) => (
            <div key={msg.id} className={cn("flex flex-col text-sm", msg.type === 'system' ? 'text-blue-500 italic' : '')}>
              <div className="flex items-baseline space-x-2">
                <span className="font-semibold text-foreground/80">{msg.sender}:</span>
                <span className="text-foreground">{msg.content}</span>
              </div>
            </div>
          ))}
          <div ref={scrollRef} />
        </div>
      </ScrollArea>
      <div className="p-2 border-t flex gap-2">
        <Input 
          placeholder="Type a message..." 
          value={input} 
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={(e) => e.key === 'Enter' && handleSend()}
        />
        <Button size="icon" onClick={handleSend}>
          <Send className="h-4 w-4" />
        </Button>
      </div>
    </div>
  );
}
