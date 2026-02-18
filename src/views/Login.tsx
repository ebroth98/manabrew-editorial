import { zodResolver } from "@hookform/resolvers/zod";
import { useForm } from "react-hook-form";
import * as z from "zod";
import { Button } from "@/components/ui/button";
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@/components/ui/form";
import { Input } from "@/components/ui/input";
import { Checkbox } from "@/components/ui/checkbox";
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "@/components/ui/card";
import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { toast } from "sonner";
import { useAuthStore } from "@/stores/useAuthStore";
import { useConnectionStore } from "@/stores/useConnectionStore";
import { wsClient } from "@/api/websocket";

const formSchema = z.object({
  serverAddress: z.string().min(1, "Server address is required"),
  username: z.string().min(3, "Username must be at least 3 characters"),
  password: z.string().optional(),
  register: z.boolean().default(false),
  flag: z.string().default("us"),
});

type FormValues = z.infer<typeof formSchema>;

export default function Login() {
  const navigate = useNavigate();
  const [isLoading, setIsLoading] = useState(false);
  const { login } = useAuthStore();
  const { setServerAddress } = useConnectionStore();

  const form = useForm({
    resolver: zodResolver(formSchema),
    defaultValues: {
      serverAddress: "xmage.de",
      username: "",
      password: "",
      register: false,
      flag: "us",
    },
  });

  async function onSubmit(values: FormValues) {
    setIsLoading(true);
    try {
      // Connect to WebSocket (Middleware)
      wsClient.connect(`ws://${values.serverAddress}:8080`); // Assume middleware is running on port 8080 or handle this properly
      
      // Simulate handshake/login with middleware
      // For now, we'll just mock it as successful after a delay
      await new Promise(resolve => setTimeout(resolve, 1000));

      setServerAddress(values.serverAddress);
      login(
        { username: values.username, serverAddress: values.serverAddress, flag: values.flag },
        "dummy-session-token"
      );

      toast.success("Connected to XMage Server");
      navigate("/lobby");
    } catch (error) {
      console.error(error);
      toast.error("Failed to connect or login");
    } finally {
      setIsLoading(false);
    }
  }

  return (
    <div className="flex items-center justify-center min-h-screen bg-gray-100 dark:bg-gray-900 p-4">
      <Card className="w-full max-w-md">
        <CardHeader>
          <CardTitle className="text-2xl text-center">XMage Web Client</CardTitle>
          <CardDescription className="text-center">
            Connect to an XMage server to play Magic: The Gathering
          </CardDescription>
        </CardHeader>
        <CardContent>
          <Form {...form}>
            <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
              <FormField
                control={form.control}
                name="serverAddress"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Server Address</FormLabel>
                    <FormControl>
                      <Input placeholder="xmage.de" {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name="username"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Username</FormLabel>
                    <FormControl>
                      <Input placeholder="JaceBeleren" {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name="password"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Password (Optional)</FormLabel>
                    <FormControl>
                      <Input type="password" placeholder="********" {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name="register"
                render={({ field }) => (
                  <FormItem className="flex flex-row items-start space-x-3 space-y-0 rounded-md border p-4">
                    <FormControl>
                      <Checkbox
                        checked={field.value}
                        onCheckedChange={field.onChange}
                      />
                    </FormControl>
                    <div className="space-y-1 leading-none">
                      <FormLabel>
                        Register new account
                      </FormLabel>
                    </div>
                  </FormItem>
                )}
              />
              <Button type="submit" className="w-full" disabled={isLoading}>
                {isLoading ? "Connecting..." : "Connect"}
              </Button>
            </form>
          </Form>
        </CardContent>
        <CardFooter className="text-sm text-center text-muted-foreground">
          v0.1.0 - Alpha
        </CardFooter>
      </Card>
    </div>
  );
}
