/**
 * App-level (Radix / shadcn) theme colours.
 * These drive CSS custom properties on :root and style the non-game chrome
 * (menus, dialogs, settings panels, deck editor, lobby, etc.).
 */
export interface ThemeColors {
  background: string;
  foreground: string;
  card: string;
  "card-foreground": string;
  popover: string;
  "popover-foreground": string;
  primary: string;
  "primary-foreground": string;
  secondary: string;
  "secondary-foreground": string;
  muted: string;
  "muted-foreground": string;
  accent: string;
  "accent-foreground": string;
  destructive: string;
  "destructive-foreground": string;
  border: string;
  input: string;
  ring: string;
  selection: string;
  "selection-foreground": string;
  commander: string;
  warning: string;
  overlay: string;
}
